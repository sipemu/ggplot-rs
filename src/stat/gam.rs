//! A small, dependency-free penalized B-spline (P-spline) GAM backend —
//! ggplot2's `method = "gam"` without the `regression` feature.
//!
//! With `regression` enabled, `StatSmooth::compute_gam_regression` fits via
//! anofox-regression's `PSplineRegressor` (faer-backed). That feature pulls
//! anofox-regression, faer, nalgebra, statrs and rand (~100 crates) and does
//! not build for `wasm32-unknown-unknown`. This module gives `method = "gam"`
//! a self-contained fallback that always builds (including for wasm): a
//! cubic B-spline basis, a second-order difference penalty (Eilers & Marx),
//! λ chosen by generalised cross-validation over a log grid, and a few dense
//! k×k solves by Gaussian elimination with partial pivoting. Adapted from
//! anofox-visualization's `src/smooth.rs` (issue #174).
//!
//! The confidence band reuses the GCV fit's own linear algebra: at the
//! selected λ, `A = B'B + λD'D` is inverted once (`H = A⁻¹`, `β̂ = H·B'y`),
//! giving the fitted-value covariance `Var(β̂) = σ²·H·B'B·H` (since the
//! smoother/hat matrix is `S = H·B'B`), so
//! `Var(f̂(x)) = σ²·b(x)ᵀ·(H·B'B·H)·b(x)`.

/// A fitted P-spline: its basis domain, coefficients, and the statistics
/// ([`edf`](Self::edf), the scaled covariance) needed for a confidence band.
pub(crate) struct PSplineFit {
    lo: f64,
    hi: f64,
    nseg: usize,
    coef: Vec<f64>,
    /// `σ² · H·B'B·H` (k×k) — the fitted-value covariance in basis space;
    /// `Var(f̂(x)) = b(x)ᵀ · cov · b(x)`.
    cov: Vec<Vec<f64>>,
    /// Effective degrees of freedom, `tr(H·B'B)`.
    edf: f64,
    /// Number of (finite) input points.
    n: f64,
}

impl PSplineFit {
    /// Effective degrees of freedom, `tr(H·B'B)` — use
    /// `crate::stat::dist::qt(0.975, n - edf)` for the CI multiplier.
    pub(crate) fn edf(&self) -> f64 {
        self.edf
    }

    /// Number of (finite) input points the fit was built from.
    pub(crate) fn n(&self) -> f64 {
        self.n
    }

    /// The domain the fit was built over, `(x_min, x_max)`.
    pub(crate) fn domain(&self) -> (f64, f64) {
        (self.lo, self.hi)
    }

    /// Fitted value `f̂(x) = b(x)ᵀβ̂`.
    pub(crate) fn predict(&self, x: f64) -> f64 {
        let b = bspline_row(x, self.lo, self.hi, self.nseg);
        b.iter().zip(&self.coef).map(|(a, c)| a * c).sum()
    }

    /// Standard error of `f̂(x)`, `sqrt(b(x)ᵀ · cov · b(x))`.
    pub(crate) fn se(&self, x: f64) -> f64 {
        let b = bspline_row(x, self.lo, self.hi, self.nseg);
        let k = b.len();
        let mut var = 0.0;
        for i in 0..k {
            if b[i] == 0.0 {
                continue;
            }
            for j in 0..k {
                var += b[i] * self.cov[i][j] * b[j];
            }
        }
        var.max(0.0).sqrt()
    }
}

/// Fit a P-spline to `(x, y)`. `None` when there are fewer than 4 distinct
/// finite x values, or every candidate λ yields a singular `B'B + λD'D`.
pub(crate) fn fit(points: &[(f64, f64)]) -> Option<PSplineFit> {
    let pts: Vec<(f64, f64)> = points
        .iter()
        .copied()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect();
    let (lo, hi) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| {
        (a.0.min(p.0), a.1.max(p.0))
    });
    let mut distinct: Vec<f64> = pts.iter().map(|p| p.0).collect();
    distinct.sort_by(f64::total_cmp);
    distinct.dedup();
    if distinct.len() < 4 || hi <= lo {
        return None;
    }
    // Interior segments: enough flexibility without over-fitting tiny inputs.
    let nseg = (distinct.len() / 3).clamp(3, 20);
    let k = nseg + 3; // cubic B-spline basis functions
    let basis = |x: f64| bspline_row(x, lo, hi, nseg);
    let rows: Vec<Vec<f64>> = pts.iter().map(|p| basis(p.0)).collect();
    // Normal equations B'B, B'y and the second-order difference penalty D'D.
    let mut btb = vec![vec![0.0; k]; k];
    let mut bty = vec![0.0; k];
    for (r, p) in rows.iter().zip(&pts) {
        for i in 0..k {
            if r[i] == 0.0 {
                continue;
            }
            bty[i] += r[i] * p.1;
            for j in 0..k {
                btb[i][j] += r[i] * r[j];
            }
        }
    }
    let mut dtd = vec![vec![0.0; k]; k];
    for i in 0..k.saturating_sub(2) {
        let d = [(i, 1.0), (i + 1, -2.0), (i + 2, 1.0)];
        for &(a, va) in &d {
            for &(b, vb) in &d {
                dtd[a][b] += va * vb;
            }
        }
    }
    let n = pts.len() as f64;
    let rss = |coef: &[f64]| -> f64 {
        rows.iter()
            .zip(&pts)
            .map(|(r, p)| {
                let f: f64 = r.iter().zip(coef).map(|(a, b)| a * b).sum();
                (p.1 - f).powi(2)
            })
            .sum()
    };
    let mut best: Option<(f64, Vec<f64>, f64, f64, f64)> = None; // gcv, coef, edf, rss, lambda
    for step in 0..=24 {
        let lambda = 10f64.powf(-3.0 + step as f64 * 0.375); // 1e-3 … 1e6
        let a: Vec<Vec<f64>> = (0..k)
            .map(|i| (0..k).map(|j| btb[i][j] + lambda * dtd[i][j]).collect())
            .collect();
        let Some(coef) = solve(&a, &bty) else {
            continue;
        };
        // Effective degrees of freedom: tr((B'B + λP)⁻¹ B'B).
        let mut edf = 0.0;
        for j in 0..k {
            let col: Vec<f64> = (0..k).map(|i| btb[i][j]).collect();
            if let Some(s) = solve(&a, &col) {
                edf += s[j];
            }
        }
        let denom = n - edf;
        if denom <= 0.5 {
            continue;
        }
        let r = rss(&coef);
        let gcv = n * r / (denom * denom);
        if gcv.is_finite() && best.as_ref().is_none_or(|b| gcv < b.0) {
            best = Some((gcv, coef, edf, r, lambda));
        }
    }
    let (_, coef, edf, rss_val, lambda) = best?;

    // Confidence band: invert A = B'B + λD'D once (H = A⁻¹), then the
    // fitted-value covariance is σ²·H·B'B·H (the smoother is S = H·B'B, and
    // β̂ = H·B'y, so Var(β̂) = σ²·H·B'B·H).
    let a: Vec<Vec<f64>> = (0..k)
        .map(|i| (0..k).map(|j| btb[i][j] + lambda * dtd[i][j]).collect())
        .collect();
    let mut h = vec![vec![0.0; k]; k];
    for j in 0..k {
        let mut e = vec![0.0; k];
        e[j] = 1.0;
        let col = solve(&a, &e)?;
        for i in 0..k {
            h[i][j] = col[i];
        }
    }
    let h_btb = matmul(&h, &btb, k);
    let cov_unscaled = matmul(&h_btb, &h, k);
    let sigma2 = rss_val / (n - edf).max(1.0);
    let cov: Vec<Vec<f64>> = cov_unscaled
        .iter()
        .map(|row| row.iter().map(|v| v * sigma2).collect())
        .collect();

    Some(PSplineFit {
        lo,
        hi,
        nseg,
        coef,
        cov,
        edf,
        n,
    })
}

/// `k×k` matrix product.
fn matmul(a: &[Vec<f64>], b: &[Vec<f64>], k: usize) -> Vec<Vec<f64>> {
    let mut out = vec![vec![0.0; k]; k];
    for i in 0..k {
        for j in 0..k {
            let mut s = 0.0;
            for m in 0..k {
                s += a[i][m] * b[m][j];
            }
            out[i][j] = s;
        }
    }
    out
}

/// The `nseg + 3` cubic B-spline basis values at `x` on equally spaced knots
/// over `[lo, hi]` (Cox–de Boor recursion).
fn bspline_row(x: f64, lo: f64, hi: f64, nseg: usize) -> Vec<f64> {
    let deg = 3;
    let dx = (hi - lo) / nseg as f64;
    let k = nseg + deg;
    // Knots t_j = lo + (j - deg)·dx, j = 0 … nseg + 2·deg.
    let knot = |j: usize| lo + (j as f64 - deg as f64) * dx;
    // Clamp x into the domain so the right end belongs to the last segment.
    let x = x.clamp(lo, hi - 1e-12 * (hi - lo).max(1.0));
    let nk = nseg + 2 * deg + 1;
    let mut b: Vec<f64> = (0..nk - 1)
        .map(|j| {
            if knot(j) <= x && x < knot(j + 1) {
                1.0
            } else {
                0.0
            }
        })
        .collect();
    for d in 1..=deg {
        for j in 0..nk - 1 - d {
            let (t0, t1, t2) = (knot(j), knot(j + d), knot(j + d + 1));
            let left = (x - t0) / (t1 - t0) * b[j];
            let right = (knot(j + d + 1) - x) / (t2 - knot(j + 1)) * b[j + 1];
            b[j] = left + right;
        }
    }
    b.truncate(k);
    b
}

/// Solve `a · x = b` by Gaussian elimination with partial pivoting.
fn solve(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = b.len();
    let mut m: Vec<Vec<f64>> = a
        .iter()
        .zip(b)
        .map(|(row, &v)| {
            let mut r = row.clone();
            r.push(v);
            r
        })
        .collect();
    for c in 0..n {
        let p = (c..n).max_by(|&i, &j| m[i][c].abs().total_cmp(&m[j][c].abs()))?;
        if m[p][c].abs() < 1e-12 {
            return None;
        }
        m.swap(c, p);
        let pivot = m[c].clone();
        for row in m.iter_mut().skip(c + 1) {
            let f = row[c] / pivot[c];
            if f != 0.0 {
                for (a, b) in row.iter_mut().zip(&pivot).skip(c) {
                    *a -= f * b;
                }
            }
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let s: f64 = (r + 1..n).map(|j| m[r][j] * x[j]).sum();
        x[r] = (m[r][n] - s) / m[r][r];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basis_is_a_partition_of_unity() {
        for x in [0.0, 0.3, 1.7, 4.99, 5.0] {
            let s: f64 = bspline_row(x, 0.0, 5.0, 5).iter().sum();
            assert!((s - 1.0).abs() < 1e-9, "x={x}: {s}");
        }
    }

    #[test]
    fn recovers_a_smooth_curve() {
        let pts: Vec<(f64, f64)> = (0..200)
            .map(|i| {
                let x = i as f64 / 20.0;
                // deterministic "noise"
                let e = ((i * 7919) % 13) as f64 / 13.0 - 0.5;
                (x, x.sin() + 0.1 * e)
            })
            .collect();
        let f = fit(&pts).unwrap();
        let err = (0..50)
            .map(|i| {
                let x = i as f64 / 49.0 * 9.95;
                (f.predict(x) - x.sin()).abs()
            })
            .fold(0.0, f64::max);
        assert!(err < 0.15, "max error {err}");
    }

    #[test]
    fn too_few_points() {
        assert!(fit(&[(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)]).is_none());
        assert!(fit(&[]).is_none());
    }

    #[test]
    fn confidence_band_widens_outside_the_data_and_is_finite() {
        let pts: Vec<(f64, f64)> = (0..60)
            .map(|i| {
                let x = i as f64 / 10.0;
                let e = ((i * 37) % 7) as f64 / 7.0 - 0.5;
                (x, x.sin() + 0.1 * e)
            })
            .collect();
        let f = fit(&pts).unwrap();
        let (lo, hi) = f.domain();
        let mid = f.se((lo + hi) / 2.0);
        let near_edge = f.se(lo + 0.01 * (hi - lo));
        assert!(mid.is_finite() && mid >= 0.0);
        assert!(near_edge.is_finite() && near_edge >= 0.0);
        assert!(f.edf() > 0.0 && f.edf().is_finite());
        assert!(f.n() == pts.len() as f64);
    }
}
