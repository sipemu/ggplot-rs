//! Student-t quantile used for confidence-interval multipliers.
//!
//! Per this crate's rule (no statistical calculation lives here), the quantile
//! itself is computed by anofox-regression (`statrs::StudentsT`) when the
//! `regression` feature is enabled. Without that feature there is no
//! distribution library available, so we fall back to the large-sample normal
//! 97.5% quantile `1.96` — enable `regression` for exact `qt(p, df)`.

/// Standard-normal 0.975 quantile — the large-sample CI multiplier used as the
/// fallback when the `regression` feature (and thus a t-distribution) is absent.
pub(crate) const Z_975: f64 = 1.959_963_984_540_054;

/// Student-t quantile: the `t` with `P(T ≤ t) = p` for `df` degrees of freedom
/// (R's `qt(p, df)`). `df` may be non-integer (e.g. loess effective df).
///
/// With the `regression` feature this delegates to anofox-regression's
/// `StudentsT` (statrs) for an exact value; without it, it returns the normal
/// approximation [`Z_975`] (correct only for `p = 0.975`).
#[cfg(feature = "regression")]
pub fn qt(p: f64, df: f64) -> f64 {
    use anofox_regression::distributions::{ContinuousCDF, StudentsT};
    if df <= 0.0 {
        return f64::NAN;
    }
    StudentsT::new(0.0, 1.0, df)
        .map(|d| d.inverse_cdf(p))
        .unwrap_or(Z_975)
}

/// See the `regression`-enabled variant above; without a distribution library
/// this returns the large-sample normal 0.975 quantile.
#[cfg(not(feature = "regression"))]
pub fn qt(_p: f64, _df: f64) -> f64 {
    Z_975
}

/// Standard-normal quantile (probit), R's `qnorm(p)`.
///
/// With the `regression` feature this delegates to anofox-regression's `Normal`
/// (statrs) for an exact value; without it, it uses the Abramowitz & Stegun
/// 26.2.23 rational approximation (|error| < ~4.5e-4 in z).
#[cfg(feature = "regression")]
pub fn qnorm(p: f64) -> f64 {
    use anofox_regression::distributions::{ContinuousCDF, Normal};
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    Normal::new(0.0, 1.0)
        .map(|d| d.inverse_cdf(p))
        .unwrap_or(0.0)
}

/// See the `regression`-enabled variant; this is the Abramowitz & Stegun
/// rational approximation used when no distribution library is available.
#[cfg(not(feature = "regression"))]
pub fn qnorm(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    // A&S 26.2.23, using symmetry about p = 0.5.
    fn rational_approx(t: f64) -> f64 {
        let (c0, c1, c2) = (2.515_517, 0.802_853, 0.010_328);
        let (d1, d2, d3) = (1.432_788, 0.189_269, 0.001_308);
        t - (c0 + c1 * t + c2 * t * t) / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t)
    }
    if p < 0.5 {
        -rational_approx((-2.0 * p.ln()).sqrt())
    } else if p > 0.5 {
        rational_approx((-2.0 * (1.0 - p).ln()).sqrt())
    } else {
        0.0
    }
}

/// Radius scaling for a bivariate confidence ellipse at `level` from `n` points,
/// matching ggplot2's `stat_ellipse`: `sqrt(2 · F⁻¹(level; 2, n−1))`.
///
/// With the `regression` feature the F quantile comes from anofox-regression's
/// `FisherSnedecor` (statrs). Without it, we fall back to the large-sample
/// limit `sqrt(-2·ln(1−level))` (the χ²₂ quantile, exact as n → ∞).
#[cfg(feature = "regression")]
pub fn ellipse_radius(level: f64, n: usize) -> f64 {
    use anofox_regression::distributions::{ContinuousCDF, FisherSnedecor};
    let dfd = (n as f64 - 1.0).max(1.0);
    match FisherSnedecor::new(2.0, dfd) {
        Ok(f) => (2.0 * f.inverse_cdf(level)).max(0.0).sqrt(),
        Err(_) => (-2.0 * (1.0 - level).ln()).sqrt(),
    }
}

/// See the `regression`-enabled variant; this uses the χ²₂ closed form.
#[cfg(not(feature = "regression"))]
pub fn ellipse_radius(level: f64, _n: usize) -> f64 {
    (-2.0 * (1.0 - level).ln()).sqrt()
}

#[cfg(all(test, feature = "regression"))]
mod tests {
    use super::{ellipse_radius, qnorm, qt};

    #[test]
    fn qnorm_matches_r() {
        // R: qnorm(c(0.025, 0.25, 0.5, 0.75, 0.975)).
        let cases = [
            (0.025, -1.959_963_984_540_054),
            (0.25, -0.674_489_750_196_082),
            (0.5, 0.0),
            (0.75, 0.674_489_750_196_082),
            (0.975, 1.959_963_984_540_054),
        ];
        for (p, want) in cases {
            assert!((qnorm(p) - want).abs() < 1e-9, "qnorm({p})");
        }
    }

    #[test]
    fn ellipse_radius_is_monotonic_and_finite() {
        let small = ellipse_radius(0.5, 30);
        let big = ellipse_radius(0.99, 30);
        assert!(big > small && small > 0.0 && big.is_finite());
    }

    // Reference values from R's qt(0.975, df).
    #[test]
    fn qt_matches_r() {
        let cases = [
            (1.0, 12.706_204_736_432_1),
            (2.0, 4.302_652_729_912_0),
            (5.0, 2.570_581_835_636_1),
            (10.0, 2.228_138_851_986_3),
            (30.0, 2.042_272_456_301_4),
            (100.0, 1.983_971_518_449_5),
        ];
        for (df, want) in cases {
            let got = qt(0.975, df);
            assert!(
                (got - want).abs() < 1e-6,
                "qt(0.975,{df}) = {got}, want {want}"
            );
        }
    }
}
