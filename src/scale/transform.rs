use crate::data::Value;

/// Scale transformation types.
#[derive(Clone, Debug)]
pub enum ScaleTransform {
    Identity,
    Log10,
    Log2,
    Ln,
    Sqrt,
    Reverse,
    /// Logit: `ln(p / (1 - p))`, for proportions in (0, 1).
    Logit,
    /// Probit: the inverse normal CDF, for proportions in (0, 1).
    Probit,
    /// Sign-preserving pseudo-log (`asinh(x / 2)`) — handles zero and negatives.
    PseudoLog,
    /// Reciprocal `1 / x`.
    Reciprocal,
    /// Exponential (`exp`); axis labels are therefore spaced logarithmically.
    Exp,
    /// Box–Cox with the given lambda: `(x^λ − 1) / λ`, or `ln(x)` at λ = 0 (x > 0).
    BoxCox(f64),
}

impl ScaleTransform {
    /// Apply the forward transformation.
    pub fn apply(&self, value: f64) -> f64 {
        match self {
            ScaleTransform::Identity => value,
            ScaleTransform::Log10 => {
                if value > 0.0 {
                    value.log10()
                } else {
                    f64::NEG_INFINITY
                }
            }
            ScaleTransform::Log2 => {
                if value > 0.0 {
                    value.log2()
                } else {
                    f64::NEG_INFINITY
                }
            }
            ScaleTransform::Ln => {
                if value > 0.0 {
                    value.ln()
                } else {
                    f64::NEG_INFINITY
                }
            }
            ScaleTransform::Sqrt => {
                if value >= 0.0 {
                    value.sqrt()
                } else {
                    f64::NAN
                }
            }
            ScaleTransform::Reverse => -value,
            ScaleTransform::Logit => {
                if value <= 0.0 {
                    f64::NEG_INFINITY
                } else if value >= 1.0 {
                    f64::INFINITY
                } else {
                    (value / (1.0 - value)).ln()
                }
            }
            ScaleTransform::Probit => qnorm(value),
            ScaleTransform::PseudoLog => (value / 2.0).asinh(),
            ScaleTransform::Reciprocal => {
                if value != 0.0 {
                    1.0 / value
                } else {
                    f64::NAN
                }
            }
            ScaleTransform::Exp => value.exp(),
            ScaleTransform::BoxCox(lambda) => {
                if value <= 0.0 {
                    f64::NAN
                } else if lambda.abs() < 1e-9 {
                    value.ln()
                } else {
                    (value.powf(*lambda) - 1.0) / lambda
                }
            }
        }
    }

    /// Apply the inverse transformation.
    pub fn inverse(&self, value: f64) -> f64 {
        match self {
            ScaleTransform::Identity => value,
            ScaleTransform::Log10 => 10f64.powf(value),
            ScaleTransform::Log2 => 2f64.powf(value),
            ScaleTransform::Ln => value.exp(),
            ScaleTransform::Sqrt => value * value,
            ScaleTransform::Reverse => -value,
            ScaleTransform::Logit => 1.0 / (1.0 + (-value).exp()),
            ScaleTransform::Probit => pnorm(value),
            ScaleTransform::PseudoLog => 2.0 * value.sinh(),
            ScaleTransform::Reciprocal => {
                if value != 0.0 {
                    1.0 / value
                } else {
                    f64::NAN
                }
            }
            ScaleTransform::Exp => value.ln(),
            ScaleTransform::BoxCox(lambda) => {
                if lambda.abs() < 1e-9 {
                    value.exp()
                } else {
                    (value * lambda + 1.0).powf(1.0 / lambda)
                }
            }
        }
    }

    /// Transform a Value.
    pub fn transform_value(&self, value: &Value) -> Value {
        match value.as_f64() {
            Some(f) => {
                let t = self.apply(f);
                if t.is_finite() {
                    Value::Float(t)
                } else {
                    Value::Na
                }
            }
            None => value.clone(),
        }
    }

    pub fn is_identity(&self) -> bool {
        matches!(self, ScaleTransform::Identity)
    }
}

/// Standard normal CDF, via the Abramowitz & Stegun 7.1.26 erf approximation.
fn pnorm(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let y = 1.0
        - (((((1.061_405_429 * t - 1.453_152_027) * t) + 1.421_413_741) * t - 0.284_496_736) * t
            + 0.254_829_592)
            * t
            * (-x * x).exp();
    sign * y
}

/// Inverse normal CDF (probit), Abramowitz & Stegun 26.2.23 rational approximation.
fn qnorm(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    if p < 0.5 {
        -rational_approx((-2.0 * p.ln()).sqrt())
    } else if p > 0.5 {
        rational_approx((-2.0 * (1.0 - p).ln()).sqrt())
    } else {
        0.0
    }
}

fn rational_approx(t: f64) -> f64 {
    let c0 = 2.515_517;
    let c1 = 0.802_853;
    let c2 = 0.010_328;
    let d1 = 1.432_788;
    let d2 = 0.189_269;
    let d3 = 0.001_308;
    t - (c0 + c1 * t + c2 * t * t) / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t)
}

#[cfg(test)]
mod tests {
    use super::ScaleTransform::*;

    fn roundtrip(t: super::ScaleTransform, v: f64, tol: f64) {
        let back = t.inverse(t.apply(v));
        assert!(
            (back - v).abs() < tol,
            "{t:?}: {v} -> {} -> {back}",
            t.apply(v)
        );
    }

    #[test]
    fn transforms_roundtrip() {
        roundtrip(Logit, 0.3, 1e-9);
        roundtrip(Probit, 0.3, 1e-2); // approximation
        roundtrip(PseudoLog, -4.0, 1e-9);
        roundtrip(PseudoLog, 0.0, 1e-9);
        roundtrip(Reciprocal, 2.5, 1e-9);
        roundtrip(Exp, 1.7, 1e-9);
        roundtrip(BoxCox(0.5), 4.0, 1e-9);
        roundtrip(BoxCox(0.0), 4.0, 1e-9); // lambda 0 == ln
    }

    #[test]
    fn transforms_domain_edges() {
        assert_eq!(Logit.apply(0.0), f64::NEG_INFINITY);
        assert_eq!(Logit.apply(1.0), f64::INFINITY);
        assert!(Reciprocal.apply(0.0).is_nan());
        assert!(BoxCox(0.5).apply(-1.0).is_nan());
        assert_eq!(Probit.apply(0.5), 0.0);
        // Box-Cox at lambda 0 is ln.
        assert!((BoxCox(0.0).apply(std::f64::consts::E) - 1.0).abs() < 1e-9);
    }
}
