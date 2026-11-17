//! Self-contained quantile / density functions for the theoretical
//! distributions of `stat_qq` / `stat_qq_band` (normal, Student t,
//! exponential, half-normal).
//!
//! These must work in every feature configuration — including the minimal,
//! plotters-free `--no-default-features` build used by the DuckDB extensions —
//! so they don't depend on the optional `regression` feature's distribution
//! library. Accuracy (checked against R in the tests):
//!
//! - `qnorm`: Wichura's AS 241 (PPND16), ~1e-16 relative;
//! - `pt` / `qt`: regularized incomplete beta (Lentz continued fraction) and a
//!   safeguarded Newton inversion, ~1e-12;
//! - `ln_gamma`: Lanczos (g = 7, n = 9), ~1e-15.

// Published algorithm coefficients are kept verbatim.
#![allow(clippy::excessive_precision)]

use std::f64::consts::PI;

/// Evaluate a polynomial with coefficients in ascending order (Horner).
fn poly(c: &[f64], x: f64) -> f64 {
    c.iter().rev().fold(0.0, |acc, &k| acc * x + k)
}

/// Standard-normal quantile (R's `qnorm(p)`), Wichura's AS 241.
pub fn qnorm(p: f64) -> f64 {
    if p.is_nan() || !(0.0..=1.0).contains(&p) {
        return f64::NAN;
    }
    if p == 0.0 {
        return f64::NEG_INFINITY;
    }
    if p == 1.0 {
        return f64::INFINITY;
    }
    const A: [f64; 8] = [
        3.387_132_872_796_366_608,
        133.141_667_891_784_377_45,
        1_971.590_950_306_551_442_7,
        13_731.693_765_509_461_125,
        45_921.953_931_549_871_457,
        67_265.770_927_008_700_853,
        33_430.575_583_588_128_105,
        2_509.080_928_730_122_672_7,
    ];
    const B: [f64; 8] = [
        1.0,
        42.313_330_701_600_911_252,
        687.187_007_492_057_908_3,
        5_394.196_021_424_751_107_7,
        21_213.794_301_586_595_867,
        39_307.895_800_092_710_61,
        28_729.085_735_721_942_674,
        5_226.495_278_852_854_561,
    ];
    const C: [f64; 8] = [
        1.423_437_110_749_683_577_34,
        4.630_337_846_156_545_295_9,
        5.769_497_221_460_691_405_5,
        3.647_848_324_763_204_605_04,
        1.270_458_252_452_368_382_58,
        0.241_780_725_177_450_611_77,
        0.022_723_844_989_269_184_583_3,
        7.745_450_142_783_414_076_4e-4,
    ];
    const D: [f64; 8] = [
        1.0,
        2.053_191_626_637_758_821_87,
        1.676_384_830_183_803_849_4,
        0.689_767_334_985_100_004_55,
        0.148_103_976_427_480_074_59,
        0.015_198_666_563_616_457_196_6,
        5.475_938_084_995_344_946e-4,
        1.050_750_071_644_416_843_24e-9,
    ];
    const E: [f64; 8] = [
        6.657_904_643_501_103_777_2,
        5.463_784_911_164_114_369_9,
        1.784_826_539_917_291_335_8,
        0.296_560_571_828_504_891_23,
        0.026_532_189_526_576_123_093,
        0.001_242_660_947_388_078_438_6,
        2.711_555_568_743_487_578_15e-5,
        2.010_334_399_292_288_132_65e-7,
    ];
    const F: [f64; 8] = [
        1.0,
        0.599_832_206_555_887_937_69,
        0.136_929_880_922_735_805_31,
        0.014_875_361_290_850_614_852_5,
        7.868_691_311_456_132_591e-4,
        1.846_318_317_510_054_681_8e-5,
        1.421_511_758_316_445_888_7e-7,
        2.044_263_103_389_939_785_64e-15,
    ];
    let q = p - 0.5;
    if q.abs() <= 0.425 {
        let r = 0.180_625 - q * q;
        return q * poly(&A, r) / poly(&B, r);
    }
    let r = if q < 0.0 { p } else { 1.0 - p };
    let r = (-r.ln()).sqrt();
    let v = if r <= 5.0 {
        let r = r - 1.6;
        poly(&C, r) / poly(&D, r)
    } else {
        let r = r - 5.0;
        poly(&E, r) / poly(&F, r)
    };
    if q < 0.0 {
        -v
    } else {
        v
    }
}

/// Standard-normal density.
pub fn dnorm(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * PI).sqrt()
}

/// `ln Γ(x)` for `x > 0` (Lanczos approximation, reflection below 0.5).
pub fn ln_gamma(x: f64) -> f64 {
    const G: f64 = 7.0;
    const COEF: [f64; 9] = [
        0.999_999_999_999_809_93,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_13,
        -176.615_029_162_140_59,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_571_6e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // Γ(x)Γ(1−x) = π / sin(πx)
        return (PI / (PI * x).sin()).abs().ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = COEF[0];
    let t = x + G + 0.5;
    for (i, &c) in COEF.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Continued fraction for the incomplete beta (modified Lentz).
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
    for m in 1..=500 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-15 {
            break;
        }
    }
    h
}

/// Regularized incomplete beta `I_x(a, b)`.
pub fn beta_inc(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let ln_front = ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln();
    if x < (a + 1.0) / (a + b + 2.0) {
        ln_front.exp() * beta_cf(a, b, x) / a
    } else {
        1.0 - ln_front.exp() * beta_cf(b, a, 1.0 - x) / b
    }
}

/// Student-t lower-tail probability `P(T ≤ t)` (R's `pt(t, df)`).
pub fn pt(t: f64, df: f64) -> f64 {
    if t.is_nan() || df.is_nan() || df <= 0.0 {
        return f64::NAN;
    }
    if t.is_infinite() {
        return if t > 0.0 { 1.0 } else { 0.0 };
    }
    let x = df / (df + t * t);
    let tail = 0.5 * beta_inc(df / 2.0, 0.5, x);
    if t > 0.0 {
        1.0 - tail
    } else {
        tail
    }
}

/// Student-t density (R's `dt(x, df)`).
pub fn dt(x: f64, df: f64) -> f64 {
    if df <= 0.0 || x.is_nan() {
        return f64::NAN;
    }
    (ln_gamma((df + 1.0) / 2.0)
        - ln_gamma(df / 2.0)
        - 0.5 * (df * PI).ln()
        - (df + 1.0) / 2.0 * (1.0 + x * x / df).ln())
    .exp()
}

/// Student-t quantile for any `p` (R's `qt(p, df)`): closed forms for
/// `df = 1, 2`, otherwise a bracketed Newton inversion of [`pt`].
pub fn qt(p: f64, df: f64) -> f64 {
    if p.is_nan() || df.is_nan() || df <= 0.0 || !(0.0..=1.0).contains(&p) {
        return f64::NAN;
    }
    if p == 0.0 {
        return f64::NEG_INFINITY;
    }
    if p == 1.0 {
        return f64::INFINITY;
    }
    if p == 0.5 {
        return 0.0;
    }
    if (df - 1.0).abs() < 1e-12 {
        return (PI * (p - 0.5)).tan();
    }
    if (df - 2.0).abs() < 1e-12 {
        return (2.0 * p - 1.0) / (2.0 * p * (1.0 - p)).sqrt();
    }
    if p > 0.5 {
        return -qt_lower(1.0 - p, df);
    }
    qt_lower(p, df)
}

/// `qt` for `p < 0.5` (a negative quantile), solved in the lower tail so
/// tiny `p` keep full precision.
fn qt_lower(p: f64, df: f64) -> f64 {
    // Bracket [lo, hi] with pt(lo) < p < pt(hi) = 0.5.
    let mut hi = 0.0;
    let mut lo = -1.0;
    let mut guard = 0;
    while pt(lo, df) > p && guard < 2000 {
        hi = lo;
        lo *= 2.0;
        guard += 1;
    }
    // Start from the normal quantile with a Cornish–Fisher correction.
    let z = qnorm(p);
    let g1 = (z.powi(3) + z) / 4.0;
    let mut t = (z + g1 / df).clamp(lo, hi);
    for _ in 0..200 {
        let f = pt(t, df) - p;
        if f.abs() <= 1e-15 * p.max(1e-300) {
            break;
        }
        if f > 0.0 {
            hi = t;
        } else {
            lo = t;
        }
        let d = dt(t, df);
        let mut next = t - f / d;
        if !(next.is_finite() && next > lo && next < hi) {
            next = 0.5 * (lo + hi);
        }
        if (next - t).abs() <= 1e-14 * t.abs().max(1.0) {
            t = next;
            break;
        }
        t = next;
    }
    t
}

/// Standard-normal lower-tail probability `Φ(x)` (via the complementary
/// error function, |error| < 1.2e-7).
pub fn pnorm(x: f64) -> f64 {
    // Numerical Recipes' erfc Chebyshev fit
    let z = x.abs() / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + 0.5 * z);
    let erfc = t
        * (-z * z - 1.265_512_23
            + t * (1.000_023_68
                + t * (0.374_091_96
                    + t * (0.096_784_18
                        + t * (-0.186_288_06
                            + t * (0.278_868_07
                                + t * (-1.135_203_98
                                    + t * (1.488_515_87
                                        + t * (-0.822_152_23 + t * 0.170_872_77)))))))))
            .exp();
    if x >= 0.0 {
        1.0 - 0.5 * erfc
    } else {
        0.5 * erfc
    }
}

/// Regularized lower incomplete gamma `P(a, x)` (series below `a + 1`,
/// continued fraction above; Numerical Recipes `gammp`).
pub fn gamma_inc(a: f64, x: f64) -> f64 {
    if x <= 0.0 || a <= 0.0 {
        return 0.0;
    }
    let ln_front = -x + a * x.ln() - ln_gamma(a);
    if x < a + 1.0 {
        let (mut ap, mut sum) = (a, 1.0 / a);
        let mut del = sum;
        for _ in 0..1000 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * 1e-15 {
                break;
            }
        }
        (sum.ln() + ln_front).exp().min(1.0)
    } else {
        const TINY: f64 = 1e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / TINY;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..1000 {
            let an = -(i as f64) * (i as f64 - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < TINY {
                d = TINY;
            }
            c = b + an / c;
            if c.abs() < TINY {
                c = TINY;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-15 {
                break;
            }
        }
        (1.0 - (ln_front.exp() * h)).max(0.0)
    }
}

/// The `p` quantile of a continuous, increasing `cdf` on `(lo, hi)`:
/// bisection (the bracket grows while `hi` is too small), to a relative
/// 1e-12.
fn invert(cdf: impl Fn(f64) -> f64, p: f64, mut lo: f64, mut hi: f64, grow: bool) -> f64 {
    if !(0.0..=1.0).contains(&p) || p.is_nan() {
        return f64::NAN;
    }
    if grow {
        for _ in 0..200 {
            if cdf(hi) >= p {
                break;
            }
            lo = hi;
            hi *= 2.0;
        }
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if cdf(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo <= 1e-12 * hi.abs().max(1e-300) {
            break;
        }
    }
    0.5 * (lo + hi)
}

/// Normal(mean, sd): density, lower-tail probability, quantile.
pub fn dnorm_ms(x: f64, mean: f64, sd: f64) -> f64 {
    dnorm((x - mean) / sd) / sd
}
pub fn pnorm_ms(x: f64, mean: f64, sd: f64) -> f64 {
    pnorm((x - mean) / sd)
}
pub fn qnorm_ms(p: f64, mean: f64, sd: f64) -> f64 {
    mean + sd * qnorm(p)
}

/// LogNormal(meanlog, sdlog): density, lower-tail probability, quantile.
pub fn dlnorm(x: f64, meanlog: f64, sdlog: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    dnorm((x.ln() - meanlog) / sdlog) / (sdlog * x)
}
pub fn plnorm(x: f64, meanlog: f64, sdlog: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    pnorm((x.ln() - meanlog) / sdlog)
}
pub fn qlnorm(p: f64, meanlog: f64, sdlog: f64) -> f64 {
    (meanlog + sdlog * qnorm(p)).exp()
}

/// Gamma(shape, rate): density, lower-tail probability, quantile.
pub fn dgamma(x: f64, shape: f64, rate: f64) -> f64 {
    if x < 0.0 || (x == 0.0 && shape > 1.0) {
        return 0.0;
    }
    if x == 0.0 {
        return if shape == 1.0 { rate } else { f64::INFINITY };
    }
    (shape * rate.ln() + (shape - 1.0) * x.ln() - rate * x - ln_gamma(shape)).exp()
}
pub fn pgamma(x: f64, shape: f64, rate: f64) -> f64 {
    gamma_inc(shape, rate * x)
}
pub fn qgamma(p: f64, shape: f64, rate: f64) -> f64 {
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    invert(|x| gamma_inc(shape, x), p, 0.0, shape.max(1.0), true) / rate
}

/// Beta(a, b): density, lower-tail probability, quantile.
pub fn dbeta(x: f64, a: f64, b: f64) -> f64 {
    if !(0.0..=1.0).contains(&x) {
        return 0.0;
    }
    if x == 0.0 || x == 1.0 {
        let e = if x == 0.0 { a } else { b };
        return if e < 1.0 {
            f64::INFINITY
        } else if e == 1.0 {
            (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b)).exp()
        } else {
            0.0
        };
    }
    (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + (a - 1.0) * x.ln() + (b - 1.0) * (1.0 - x).ln())
        .exp()
}
pub fn pbeta(x: f64, a: f64, b: f64) -> f64 {
    beta_inc(a, b, x)
}
pub fn qbeta(p: f64, a: f64, b: f64) -> f64 {
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    invert(|x| beta_inc(a, b, x), p, 0.0, 1.0, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol * b.abs().max(1.0)
    }

    #[test]
    fn qnorm_matches_r() {
        // R: sprintf("%.17g", qnorm(c(1e-10, 0.001, 0.025, 0.25, 0.6, 0.975, 0.999999)))
        let cases = [
            (1e-10, -6.361_340_902_404_056_6),
            (0.001, -3.090_232_306_167_813_5),
            (0.025, -1.959_963_984_540_054),
            (0.25, -0.674_489_750_196_081_74),
            (0.6, 0.253_347_103_135_799_78),
            (0.975, 1.959_963_984_540_054),
            (0.999_999, 4.753_424_308_817_089),
        ];
        for (p, want) in cases {
            assert!(close(qnorm(p), want, 1e-13), "qnorm({p}) = {}", qnorm(p));
        }
        assert_eq!(qnorm(0.5), 0.0);
        assert_eq!(qnorm(0.0), f64::NEG_INFINITY);
        assert!(qnorm(1.5).is_nan());
    }

    #[test]
    fn t_distribution_matches_r() {
        // R: qt(c(0.01, 0.1, 0.3, 0.975), df = 3); qt(0.001, 7.5); pt(-2.5, 4); dt(1.3, 5)
        let q3 = [
            (0.01, -4.540_702_858_568_133),
            (0.1, -1.637_744_353_696_208_9),
            (0.3, -0.584_389_727_439_818_65),
            (0.975, 3.182_446_305_283_707_8),
        ];
        for (p, want) in q3 {
            assert!(
                close(qt(p, 3.0), want, 1e-10),
                "qt({p}, 3) = {}",
                qt(p, 3.0)
            );
        }
        assert!(close(qt(0.001, 7.5), -4.630_319_094_034_099_7, 1e-9));
        assert!(close(pt(-2.5, 4.0), 0.033_383_272_405_994_063, 1e-11));
        assert!(close(dt(1.3, 5.0), 0.158_476_735_728_982_41, 1e-12));
        // Closed forms.
        assert!(close(qt(0.9, 1.0), 3.077_683_537_175_254_4, 1e-12));
        assert!(close(qt(0.9, 2.0), 1.885_618_083_164_126_9, 1e-12));
        // Round trip in the far tail.
        let t = qt(1e-9, 4.0);
        assert!(close(pt(t, 4.0), 1e-9, 1e-8), "{t}");
    }

    #[test]
    fn ln_gamma_matches_known_values() {
        assert!(close(ln_gamma(0.5), 0.5 * PI.ln(), 1e-14));
        assert!(close(ln_gamma(10.0), (362_880.0f64).ln(), 1e-14));
        assert!(close(ln_gamma(1.0), 0.0, 1e-14));
    }
}

#[cfg(test)]
mod family_tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn pnorm_matches_known_values() {
        assert!(close(pnorm(0.0), 0.5, 1e-7));
        assert!(close(pnorm(1.959_964), 0.975, 1e-6));
        assert!(close(pnorm(-1.644_854), 0.05, 1e-6));
    }

    #[test]
    fn gamma_inc_matches_r() {
        // R: pgamma(2, shape = 3) = 0.3233236, pgamma(10, 3) = 0.9972306
        assert!(close(gamma_inc(3.0, 2.0), 0.323_323_6, 1e-7));
        assert!(close(gamma_inc(3.0, 10.0), 0.997_230_6, 1e-7));
        // shape 1 is the exponential
        assert!(close(gamma_inc(1.0, 0.7), 1.0 - (-0.7f64).exp(), 1e-12));
    }

    #[test]
    fn quantiles_invert_their_cdfs() {
        for &(a, b) in &[(2.0, 3.0), (0.5, 0.5), (20.0, 4.0), (1.0, 1.0)] {
            for &p in &[0.01, 0.05, 0.5, 0.95, 0.99] {
                let q = qbeta(p, a, b);
                assert!(close(pbeta(q, a, b), p, 1e-9), "beta({a},{b}) p={p}");
            }
        }
        for &(k, r) in &[(0.7, 2.0), (3.0, 0.5), (50.0, 10.0)] {
            for &p in &[0.01, 0.05, 0.5, 0.95, 0.99] {
                let q = qgamma(p, k, r);
                assert!(close(pgamma(q, k, r), p, 1e-9), "gamma({k},{r}) p={p}");
            }
        }
        // scipy: beta.ppf(0.95, 2, 3) = 0.75139537, gamma.ppf(0.05, 3, scale = 2) = 1.6353829
        assert!(close(qbeta(0.95, 2.0, 3.0), 0.751_395_37, 1e-7));
        assert!(close(qgamma(0.05, 3.0, 0.5), 1.635_382_9, 1e-6));
        assert!(close(qlnorm(0.5, 1.0, 0.4), 1f64.exp(), 1e-12));
    }

    #[test]
    fn edges_and_out_of_domain_values() {
        // outside the support
        assert_eq!(dlnorm(0.0, 0.0, 1.0), 0.0);
        assert_eq!(plnorm(-1.0, 0.0, 1.0), 0.0);
        assert_eq!(dgamma(-1.0, 2.0, 1.0), 0.0);
        assert_eq!(dgamma(0.0, 2.0, 1.0), 0.0);
        assert_eq!(dgamma(0.0, 1.0, 3.0), 3.0);
        assert!(dgamma(0.0, 0.5, 1.0).is_infinite());
        assert_eq!(dbeta(1.5, 2.0, 2.0), 0.0);
        assert_eq!(dbeta(0.0, 2.0, 3.0), 0.0);
        assert!(dbeta(1.0, 2.0, 0.5).is_infinite());
        assert!(close(dbeta(0.0, 1.0, 3.0), 3.0, 1e-9));
        assert_eq!(gamma_inc(2.0, 0.0), 0.0);
        assert_eq!(gamma_inc(-1.0, 1.0), 0.0);
        // the ends of the quantiles
        assert_eq!(qgamma(0.0, 2.0, 1.0), 0.0);
        assert!(qgamma(1.0, 2.0, 1.0).is_infinite());
        assert_eq!(qbeta(0.0, 2.0, 3.0), 0.0);
        assert_eq!(qbeta(1.0, 2.0, 3.0), 1.0);
        assert!(invert(|x| x, f64::NAN, 0.0, 1.0, false).is_nan());
        assert!(invert(|x| x, 1.5, 0.0, 1.0, false).is_nan());
        // the CDFs and their quantiles agree
        assert!(close(
            pnorm_ms(qnorm_ms(0.9, 3.0, 2.0), 3.0, 2.0),
            0.9,
            1e-6
        ));
        assert!(close(plnorm(qlnorm(0.2, 0.5, 0.3), 0.5, 0.3), 0.2, 1e-6));
        assert!(close(pnorm(-3.0) + pnorm(3.0), 1.0, 1e-7));
    }

    #[test]
    fn densities_integrate_to_one() {
        let integ = |f: &dyn Fn(f64) -> f64, lo: f64, hi: f64| {
            let n = 20_000;
            let h = (hi - lo) / n as f64;
            (0..n).map(|i| f(lo + (i as f64 + 0.5) * h)).sum::<f64>() * h
        };
        assert!(close(integ(&|x| dbeta(x, 2.0, 5.0), 0.0, 1.0), 1.0, 1e-6));
        assert!(close(integ(&|x| dgamma(x, 3.0, 2.0), 0.0, 30.0), 1.0, 1e-6));
        assert!(close(integ(&|x| dlnorm(x, 0.0, 0.5), 0.0, 40.0), 1.0, 1e-5));
        assert!(close(
            integ(&|x| dnorm_ms(x, 5.0, 2.0), -20.0, 30.0),
            1.0,
            1e-6
        ));
    }
}
