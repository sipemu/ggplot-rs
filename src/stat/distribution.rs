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
