//! Small statistical-distribution helpers used for confidence intervals.
//!
//! `qt(p, df)` is the Student-t quantile (inverse CDF), matching R's `qt()`, so
//! smoother/summary confidence bands use the correct multiplier `qt(0.975, df)`
//! for finite samples instead of the large-sample normal `1.96`.

/// Natural log of the gamma function (Lanczos, g = 7). Valid for x > 0.
fn ln_gamma(x: f64) -> f64 {
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_1,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // reflection: ln Γ(x) = ln(π / sin(πx)) − ln Γ(1−x)
        (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).ln() - ln_gamma(1.0 - x)
    } else {
        let x = x - 1.0;
        let t = x + 7.5;
        let mut a = C[0];
        for (i, &c) in C.iter().enumerate().skip(1) {
            a += c / (x + i as f64);
        }
        0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
    }
}

/// Continued-fraction expansion for the incomplete beta (Numerical Recipes).
fn betacf(a: f64, b: f64, x: f64) -> f64 {
    const MAXIT: usize = 300;
    const EPS: f64 = 3e-16;
    const FPMIN: f64 = 1e-300;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FPMIN {
        d = FPMIN;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=MAXIT {
        let m = m as f64;
        let m2 = 2.0 * m;
        let mut aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        h *= d * c;
        aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < EPS {
            break;
        }
    }
    h
}

/// Regularized incomplete beta I_x(a, b).
fn betai(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bt = (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(a, b, x) / a
    } else {
        1.0 - bt * betacf(b, a, 1.0 - x) / b
    }
}

/// Student-t CDF: P(T ≤ t) for `df` degrees of freedom.
pub fn t_cdf(t: f64, df: f64) -> f64 {
    if df <= 0.0 {
        return f64::NAN;
    }
    let x = df / (df + t * t);
    let ib = 0.5 * betai(df / 2.0, 0.5, x);
    if t > 0.0 {
        1.0 - ib
    } else {
        ib
    }
}

/// Student-t quantile: the `t` with P(T ≤ t) = `p` for `df` degrees of freedom
/// (R's `qt(p, df)`). `df` may be non-integer (e.g. loess effective df).
pub fn qt(p: f64, df: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    if df <= 0.0 {
        return f64::NAN;
    }
    // Bisection on the CDF. For p = 0.975 the quantile is small even at df = 1
    // (≈12.7), so a ±1e4 bracket is ample; 200 halvings → ~1e-56 precision.
    let (mut lo, mut hi) = (-1.0e4_f64, 1.0e4_f64);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if t_cdf(mid, df) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

#[cfg(test)]
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
