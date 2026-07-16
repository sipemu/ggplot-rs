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

#[cfg(all(test, feature = "regression"))]
mod tests {
    use super::qt;

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
