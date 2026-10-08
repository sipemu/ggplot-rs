use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Gaussian kernel density estimation with Silverman bandwidth.
pub struct StatDensity {
    pub n_points: usize,
}

impl Default for StatDensity {
    fn default() -> Self {
        StatDensity { n_points: 512 }
    }
}

impl Stat for StatDensity {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let values: Vec<f64> = x_col.iter().filter_map(|v| v.as_f64()).collect();
        if values.len() < 2 {
            return DataFrame::new();
        }

        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let var = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let sd = var.sqrt();

        // Silverman's rule of thumb
        let bandwidth = 0.9 * sd.min(iqr(&values) / 1.34) * n.powf(-0.2);
        let bandwidth = if bandwidth > 0.0 { bandwidth } else { sd * 0.5 };

        let x_min = values.iter().cloned().fold(f64::INFINITY, f64::min) - 3.0 * bandwidth;
        let x_max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 3.0 * bandwidth;
        let step = (x_max - x_min) / (self.n_points - 1) as f64;

        // Exact O(n × n_points) evaluation for small inputs (bit-for-bit the
        // historical output); linear binning + a truncated kernel table for
        // large ones (O(n + n_points × window)), within ~2e-5 of the peak.
        let dens = if values.len() >= BINNED_MIN_N {
            binned_kde(&values, bandwidth, x_min, step, self.n_points)
        } else {
            None
        }
        .unwrap_or_else(|| exact_kde(&values, bandwidth, x_min, step, self.n_points));

        let x_vals: Vec<Value> = (0..self.n_points)
            .map(|i| Value::Float(x_min + i as f64 * step))
            .collect();
        let y_vals: Vec<Value> = dens.into_iter().map(Value::Float).collect();

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);

        // Carry over grouping columns
        for col_name in &["color", "fill", "group"] {
            if let Some(col) = data.column(col_name) {
                if let Some(first) = col.first() {
                    result.add_column(col_name.to_string(), vec![first.clone(); self.n_points]);
                }
            }
        }

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn name(&self) -> &str {
        "density"
    }
}

/// Inputs with at least this many finite values use the binned estimator.
const BINNED_MIN_N: usize = 2048;

/// Exact Gaussian KDE evaluated on the grid `x_min + i·step`.
fn exact_kde(values: &[f64], bw: f64, x_min: f64, step: f64, n_points: usize) -> Vec<f64> {
    let n = values.len() as f64;
    (0..n_points)
        .map(|i| {
            let x = x_min + i as f64 * step;
            values
                .iter()
                .map(|xi| gaussian_kernel((x - xi) / bw))
                .sum::<f64>()
                / (n * bw)
        })
        .collect()
}

/// Binned Gaussian KDE (Wand 1994): linearly bin the data onto a fine grid
/// whose spacing is at most `bw / 50` (and divides the output step), then
/// convolve with a kernel table truncated at ±8 bandwidths. Returns `None` when
/// the inputs aren't suitable (non-finite data/bandwidth, or a fine grid that
/// would be unreasonably large) so the caller falls back to the exact sum.
fn binned_kde(values: &[f64], bw: f64, x_min: f64, step: f64, n_points: usize) -> Option<Vec<f64>> {
    const MAX_FINE: usize = 1 << 20;
    if !(bw.is_finite() && bw > 0.0 && step.is_finite() && step > 0.0 && n_points >= 2) {
        return None;
    }
    if values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    // Refinement: r fine cells per output step, fine spacing delta <= bw/50.
    let r = (step * 50.0 / bw).ceil().max(1.0);
    let m = (n_points - 1) as f64 * r + 1.0;
    if !m.is_finite() || m > MAX_FINE as f64 {
        return None;
    }
    let (r, m) = (r as usize, m as usize);
    let delta = step / r as f64;

    // Linear binning: split each point's unit mass between its two neighbours.
    let mut w = vec![0.0f64; m];
    for &x in values {
        let pos = ((x - x_min) / delta).clamp(0.0, (m - 1) as f64);
        let j = (pos.floor() as usize).min(m - 1);
        let f = pos - j as f64;
        w[j] += 1.0 - f;
        if f > 0.0 && j + 1 < m {
            w[j + 1] += f;
        }
    }

    // Kernel weights by fine-grid offset, truncated at 8 bandwidths (< 1e-14).
    let half = ((8.0 * bw / delta).ceil() as usize).min(m);
    let table: Vec<f64> = (0..=half)
        .map(|o| gaussian_kernel(o as f64 * delta / bw))
        .collect();

    let norm = values.len() as f64 * bw;
    Some(
        (0..n_points)
            .map(|i| {
                let c = i * r;
                let lo = c.saturating_sub(half);
                let hi = (c + half).min(m - 1);
                let mut acc = 0.0;
                for (j, wj) in w.iter().enumerate().take(hi + 1).skip(lo) {
                    if *wj != 0.0 {
                        acc += wj * table[j.abs_diff(c)];
                    }
                }
                acc / norm
            })
            .collect(),
    )
}

fn gaussian_kernel(x: f64) -> f64 {
    (-(x * x) / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

fn iqr(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    quantile_type7(&sorted, 0.75) - quantile_type7(&sorted, 0.25)
}

/// R-compatible type-7 quantile interpolation (R's default `quantile()` method).
fn quantile_type7(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return sorted[0];
    }
    let h = (n - 1) as f64 * p;
    let lo = h.floor() as usize;
    let hi = (lo + 1).min(n - 1);
    let frac = h - lo as f64;
    sorted[lo] + frac * (sorted[hi] - sorted[lo])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(values: &[f64]) -> DataFrame {
        let mut df = DataFrame::new();
        df.add_column(
            "x".into(),
            values.iter().map(|v| Value::Float(*v)).collect(),
        );
        df
    }

    /// Deterministic, bimodal, heavy-ish tailed sample.
    fn sample(n: usize) -> Vec<f64> {
        let mut rng = crate::rng::SplitMix64::new(99);
        (0..n)
            .map(|i| {
                // Box–Muller normal draws, two modes.
                let u1 = rng.next_f64().max(1e-12);
                let u2 = rng.next_f64();
                let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
                if i % 3 == 0 {
                    10.0 + 0.5 * z
                } else {
                    z * 2.0
                }
            })
            .collect()
    }

    fn setup(values: &[f64]) -> (f64, f64, f64) {
        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let sd = (values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        let bw = 0.9 * sd.min(iqr(values) / 1.34) * n.powf(-0.2);
        let lo = values.iter().cloned().fold(f64::INFINITY, f64::min) - 3.0 * bw;
        let hi = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 3.0 * bw;
        (bw, lo, (hi - lo) / 511.0)
    }

    #[test]
    fn binned_matches_exact_within_tight_tolerance() {
        for n in [2048, 5000, 20_000] {
            let v = sample(n);
            let (bw, lo, step) = setup(&v);
            let exact = exact_kde(&v, bw, lo, step, 512);
            let binned = binned_kde(&v, bw, lo, step, 512).expect("binned path");
            let peak = exact.iter().cloned().fold(0.0, f64::max);
            let max_err = exact
                .iter()
                .zip(&binned)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0, f64::max);
            assert!(
                max_err / peak < 5e-5,
                "n={n}: relative error {}",
                max_err / peak
            );
            // Both integrate to ~1 over the grid.
            let area: f64 = binned.iter().sum::<f64>() * step;
            assert!((area - 1.0).abs() < 1e-3, "area {area}");
        }
    }

    #[test]
    fn large_input_uses_binned_path_and_stays_close() {
        let v = sample(10_000);
        let out = StatDensity::default().compute_group(&frame(&v), &ScaleSet::new());
        let ys: Vec<f64> = out
            .column("y")
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let (bw, lo, step) = setup(&v);
        let exact = exact_kde(&v, bw, lo, step, 512);
        let peak = exact.iter().cloned().fold(0.0, f64::max);
        for (a, b) in exact.iter().zip(&ys) {
            assert!((a - b).abs() / peak < 1e-4);
        }
    }

    #[test]
    fn binned_declines_unsuitable_inputs() {
        assert!(binned_kde(&[1.0, f64::NAN], 0.5, 0.0, 0.01, 512).is_none());
        assert!(binned_kde(&[1.0, 2.0], 0.0, 0.0, 0.01, 512).is_none());
        // A tiny bandwidth over a huge range would need > 2^20 fine cells.
        assert!(binned_kde(&[0.0, 1e9], 1e-6, 0.0, 1e9 / 511.0, 512).is_none());
    }
}
