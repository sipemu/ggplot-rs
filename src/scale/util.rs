/// Extended-Wilkinson tick locations (Talbot, Lin & Hanrahan 2010), matching R's
/// `scales::extended_breaks()` / `labeling::extended()`. Returns "nice" break
/// values covering `[dmin, dmax]` with roughly `m` labels.
pub fn extended_breaks(dmin: f64, dmax: f64, m: usize) -> Vec<f64> {
    if !(dmin.is_finite() && dmax.is_finite()) || dmax <= dmin || m < 2 {
        return vec![];
    }
    // Preferred step mantissas and score weights (simplicity, coverage, density,
    // legibility) — the paper's defaults, as in the `labeling` package.
    const Q: [f64; 6] = [1.0, 5.0, 2.0, 2.5, 4.0, 3.0];
    const W: [f64; 4] = [0.25, 0.2, 0.5, 0.05];
    let m = m as f64;

    let simplicity = |qi: usize, j: f64, lmin: f64, lmax: f64, step: f64| {
        let eps = 1e-10;
        let modulo = lmin.rem_euclid(step);
        let v = if (modulo < eps || step - modulo < eps) && lmin <= 0.0 && lmax >= 0.0 {
            1.0
        } else {
            0.0
        };
        1.0 - qi as f64 / (Q.len() as f64 - 1.0) - j + v
    };
    let simplicity_max = |qi: usize, j: f64| 1.0 - qi as f64 / (Q.len() as f64 - 1.0) - j + 1.0;
    let coverage = |lmin: f64, lmax: f64| {
        let r = dmax - dmin;
        1.0 - 0.5 * ((dmax - lmax).powi(2) + (dmin - lmin).powi(2)) / (0.1 * r).powi(2)
    };
    let coverage_max = |span: f64| {
        let r = dmax - dmin;
        if span > r {
            let half = (span - r) / 2.0;
            1.0 - 0.5 * (half * half + half * half) / (0.1 * r).powi(2)
        } else {
            1.0
        }
    };
    let density = |k: f64, lmin: f64, lmax: f64| {
        let r = (k - 1.0) / (lmax - lmin);
        let rt = (m - 1.0) / (lmax.max(dmax) - lmin.min(dmin));
        2.0 - (r / rt).max(rt / r)
    };
    let density_max = |k: f64| {
        if k >= m {
            2.0 - (k - 1.0) / (m - 1.0)
        } else {
            1.0
        }
    };

    let mut best_score = -2.0;
    let mut best = (dmin, dmax, (dmax - dmin) / (m - 1.0));

    for j_i in 1..=4u32 {
        let j = j_i as f64;
        for (qi, &q) in Q.iter().enumerate() {
            let sm = simplicity_max(qi, j);
            if W[0] * sm + W[1] + W[2] + W[3] < best_score {
                break;
            }
            for k_i in 2..=(2.0 * m + 4.0) as usize {
                let k = k_i as f64;
                let dm = density_max(k);
                if W[0] * sm + W[1] + W[2] * dm + W[3] < best_score {
                    break;
                }
                let delta = (dmax - dmin) / (k + 1.0) / j / q;
                let z0 = delta.log10().ceil() as i32;
                for z in z0..(z0 + 6) {
                    let step = j * q * 10f64.powi(z);
                    let cm = coverage_max(step * (k - 1.0));
                    if W[0] * sm + W[1] * cm + W[2] * dm + W[3] < best_score {
                        break;
                    }
                    let min_start = (dmax / step).floor() * j - (k - 1.0) * j;
                    let max_start = (dmin / step).ceil() * j;
                    if min_start > max_start {
                        continue;
                    }
                    let mut start = min_start;
                    while start <= max_start {
                        let lmin = start * step / j;
                        let lmax = lmin + step * (k - 1.0);
                        let s = simplicity(qi, j, lmin, lmax, step);
                        let c = coverage(lmin, lmax);
                        let g = density(k, lmin, lmax);
                        let score = W[0] * s + W[1] * c + W[2] * g + W[3];
                        if score > best_score {
                            best_score = score;
                            best = (lmin, lmax, step);
                        }
                        start += 1.0;
                    }
                }
            }
        }
    }

    let (lmin, lmax, step) = best;
    if step <= 0.0 {
        return vec![];
    }
    let n = ((lmax - lmin) / step).round() as usize;
    (0..=n).map(|i| lmin + i as f64 * step).collect()
}

/// Round step size to a "nice" value (1, 2, 5, 10, 20, 50, ...).
pub fn nice_step(raw: f64) -> f64 {
    let magnitude = 10f64.powf(raw.abs().log10().floor());
    let fraction = raw / magnitude;

    let nice = if fraction <= 1.5 {
        1.0
    } else if fraction <= 3.5 {
        2.0
    } else if fraction <= 7.5 {
        5.0
    } else {
        10.0
    };

    nice * magnitude
}

/// Format a number nicely, removing trailing zeros.
pub fn format_number(v: f64) -> String {
    if v == v.round() && v.abs() < 1e10 {
        format!("{}", v as i64)
    } else {
        let s = format!("{:.2}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
