//! Student-t quantile used for confidence-interval multipliers.
//!
//! With the `regression` feature the quantile is computed by
//! anofox-regression (`statrs::StudentsT`). Without it, a self-contained
//! inversion of the t CDF (regularised incomplete beta, bisection) gives the
//! same value to ~1e-10 — before #414 this fell back to the normal 1.96 for
//! every `p` and `df`, so small-sample intervals were too narrow.

/// Standard-normal 0.975 quantile — the fallback if statrs can't build the
/// t distribution (the `regression` path).
#[cfg(feature = "regression")]
pub(crate) const Z_975: f64 = 1.959_963_984_540_054;

/// Student-t quantile: the `t` with `P(T ≤ t) = p` for `df` degrees of freedom
/// (R's `qt(p, df)`). `df` may be non-integer (e.g. loess effective df).
///
/// With the `regression` feature this delegates to anofox-regression's
/// `StudentsT` (statrs); without it, [`qt_native`] computes it.
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

/// See the `regression`-enabled variant above.
#[cfg(not(feature = "regression"))]
pub fn qt(p: f64, df: f64) -> f64 {
    qt_native(p, df)
}

#[cfg_attr(feature = "regression", allow(dead_code))]
/// Continued fraction for the regularised incomplete beta (Numerical Recipes'
/// `betacf`, modified Lentz).
fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    const TINY: f64 = 1e-300;
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=300 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        d = if d.abs() < TINY { TINY } else { d };
        c = 1.0 + aa / c;
        c = if c.abs() < TINY { TINY } else { c };
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        d = if d.abs() < TINY { TINY } else { d };
        c = 1.0 + aa / c;
        c = if c.abs() < TINY { TINY } else { c };
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-15 {
            break;
        }
    }
    h
}

#[cfg_attr(feature = "regression", allow(dead_code))]
/// Regularised incomplete beta `I_x(a, b)`.
fn inc_beta(a: f64, b: f64, x: f64) -> f64 {
    use crate::stat::distribution::ln_gamma;
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front =
        (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_cf(a, b, x) / a
    } else {
        1.0 - front * beta_cf(b, a, 1.0 - x) / b
    }
}

#[cfg_attr(feature = "regression", allow(dead_code))]
/// Student-t CDF `P(T ≤ t)` for `df` degrees of freedom.
pub(crate) fn pt_native(t: f64, df: f64) -> f64 {
    let tail = 0.5 * inc_beta(df / 2.0, 0.5, df / (df + t * t));
    if t >= 0.0 {
        1.0 - tail
    } else {
        tail
    }
}

#[cfg_attr(feature = "regression", allow(dead_code))]
/// Student-t quantile by bisection on [`pt_native`] (any `df > 0`).
pub(crate) fn qt_native(p: f64, df: f64) -> f64 {
    if df.is_nan() || df <= 0.0 || !(0.0..=1.0).contains(&p) {
        return f64::NAN;
    }
    if p == 0.0 {
        return f64::NEG_INFINITY;
    }
    if p == 1.0 {
        return f64::INFINITY;
    }
    let (mut lo, mut hi) = (-1.0, 1.0);
    while pt_native(lo, df) > p {
        lo *= 2.0;
    }
    while pt_native(hi, df) < p {
        hi *= 2.0;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if pt_native(mid, df) < p {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-12 * hi.abs().max(1.0) {
            break;
        }
    }
    0.5 * (lo + hi)
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

#[cfg(test)]
mod native_t_tests {
    use super::qt_native;

    #[test]
    fn qt_matches_r() {
        // R: qt(c(.975, .95, .995, .025, .9), df)
        for (p, df, want) in [
            (0.975, 6.0, 2.446911851),
            (0.95, 6.0, 1.943180281),
            (0.995, 3.0, 5.840909310),
            (0.025, 10.0, -2.228138852),
            (0.9, 1.0, 3.077683537),
            (0.975, 1000.0, 1.962339081),
            (0.975, 2.5, 3.574654842),
        ] {
            let got = qt_native(p, df);
            assert!((got - want).abs() < 1e-7, "qt({p}, {df}) = {got}, R {want}");
        }
    }
}
