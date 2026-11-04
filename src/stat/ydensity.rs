use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Kernel density estimation on Y per group (for violin plots).
/// Outputs: x (group value), y (eval points), violinwidth (density normalized to
/// [0, 1]). The geom mirrors `violinwidth` around the group's x slot.
///
/// Also emits `density` (the raw estimate), `n` (the group's count) — which
/// the geom's `scale = "area" | "count"` need — and, when `quantiles` is set,
/// a `quantiles` cell with the group's sample quantiles (`"q1,q2,…"`).
#[derive(Clone, Debug, PartialEq)]
pub struct StatYDensity {
    pub n_points: usize,
    /// Evaluate over the data range only (`trim = TRUE`, default); `false`
    /// extends ±3 bandwidths into the tails.
    pub trim: bool,
    /// Fixed kernel bandwidth; `None` = R's `bw.nrd0`.
    pub bw: Option<f64>,
    /// Bandwidth multiplier (`adjust`, default 1).
    pub adjust: f64,
    /// Sample quantiles to report (`quantiles`, e.g. `[0.25, 0.5, 0.75]`).
    pub quantiles: Vec<f64>,
}

impl Default for StatYDensity {
    fn default() -> Self {
        StatYDensity {
            n_points: 512,
            trim: true,
            bw: None,
            adjust: 1.0,
            quantiles: Vec::new(),
        }
    }
}

/// R's default (type 7) sample quantile of sorted `v`.
fn quantile7(v: &[f64], p: f64) -> f64 {
    let h = (v.len() - 1) as f64 * p.clamp(0.0, 1.0);
    let (lo, hi) = (h.floor() as usize, h.ceil() as usize);
    v[lo] + (h - lo as f64) * (v[hi] - v[lo])
}

impl Stat for StatYDensity {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = data.column("x");
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let values: Vec<f64> = y_col.iter().filter_map(|v| v.as_f64()).collect();
        if values.len() < 2 {
            return DataFrame::new();
        }

        // Keep the group's x *value* as-is (e.g. the discrete label "A"). The geom
        // maps it through the X scale, exactly like boxplot — converting to f64 here
        // would collapse every discrete group to 0.0.
        let group_x = x_col
            .and_then(|c| c.first())
            .cloned()
            .unwrap_or(Value::Float(0.0));

        let n = values.len() as f64;
        // R's bw.nrd0 (positive even for zero-spread data), or a fixed `bw`,
        // times `adjust`.
        let adjust = if self.adjust.is_finite() && self.adjust > 0.0 {
            self.adjust
        } else {
            1.0
        };
        let bandwidth = self
            .bw
            .filter(|b| b.is_finite() && *b > 0.0)
            .unwrap_or_else(|| super::bw_nrd0(&values))
            * adjust;

        // ggplot2's geom_violin defaults to trim = TRUE: evaluate the density
        // over the observed data range; trim = FALSE extends ±3 bandwidths.
        let tail = if self.trim { 0.0 } else { 3.0 * bandwidth };
        let y_min = values.iter().cloned().fold(f64::INFINITY, f64::min) - tail;
        let y_max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + tail;
        let step = (y_max - y_min) / (self.n_points - 1) as f64;

        let mut x_vals = Vec::with_capacity(self.n_points);
        let mut y_vals = Vec::with_capacity(self.n_points);

        // Compute density at each evaluation point
        let mut densities = Vec::with_capacity(self.n_points);
        let mut max_density: f64 = 0.0;
        for i in 0..self.n_points {
            let y = y_min + i as f64 * step;
            let density: f64 = values
                .iter()
                .map(|yi| gaussian_kernel((y - yi) / bandwidth))
                .sum::<f64>()
                / (n * bandwidth);
            densities.push((y, density));
            if density > max_density {
                max_density = density;
            }
        }

        // Normalize density to [0, 1] (peak = 1). The geom scales this by the
        // per-group slot half-width, so the widest point fills the group's slot.
        let scale = if max_density > 0.0 {
            1.0 / max_density
        } else {
            1.0
        };

        let mut width_vals = Vec::with_capacity(self.n_points);
        let mut density_vals = Vec::with_capacity(self.n_points);
        for (y, density) in &densities {
            x_vals.push(group_x.clone());
            y_vals.push(Value::Float(*y));
            width_vals.push(Value::Float(density * scale));
            density_vals.push(Value::Float(*density));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result.add_column("violinwidth".to_string(), width_vals);
        result.add_column("density".to_string(), density_vals);
        result.add_column("n".to_string(), vec![Value::Float(n); self.n_points]);
        if !self.quantiles.is_empty() {
            let mut sorted = values.clone();
            sorted.sort_by(f64::total_cmp);
            let q = self
                .quantiles
                .iter()
                .map(|&p| quantile7(&sorted, p).to_string())
                .collect::<Vec<_>>()
                .join(",");
            result.add_column("quantiles".to_string(), vec![Value::Str(q); self.n_points]);
        }

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
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "ydensity"
    }
}

fn gaussian_kernel(x: f64) -> f64 {
    (-(x * x) / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ydensity_basic() {
        let mut data = DataFrame::new();
        data.add_column("x".to_string(), vec![Value::Float(1.0); 50]);
        let y_vals: Vec<Value> = (0..50).map(|i| Value::Float(i as f64)).collect();
        data.add_column("y".to_string(), y_vals);

        let stat = StatYDensity::default();
        let scales = ScaleSet::new();
        let result = stat.compute_group(&data, &scales);

        assert!(result.nrows() > 0);
        assert!(result.column("x").is_some());
        assert!(result.column("y").is_some());
        assert!(result.column("violinwidth").is_some());
        // Normalized width peaks at 1.0.
        let max_w = result
            .column("violinwidth")
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64())
            .fold(0.0_f64, f64::max);
        assert!(
            (max_w - 1.0).abs() < 1e-9,
            "peak width should be 1.0, got {max_w}"
        );
    }
}
