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
