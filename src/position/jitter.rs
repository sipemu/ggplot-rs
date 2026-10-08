use crate::data::{DataFrame, Value};
use crate::rng::SplitMix64;

use super::{Position, PositionParams};

/// Default seed: the same data must render the same SVG (stable snapshots,
/// cacheable output, no flicker when a host re-renders), as with ggplot2's
/// `position_jitter(seed = …)`.
pub const JITTER_SEED: u64 = 0x6A17_7E2D;

/// Add random noise to x and y positions to reduce overplotting.
///
/// Each value moves by a uniform amount in `[-width, width)` (x) and
/// `[-height, height)` (y). `None` (the default) means "40% of the data's
/// resolution", as in ggplot2: the smallest gap between distinct values, or 1
/// for integer/constant data — so integer-coded categories (1, 2, 3, …) get
/// ±0.4 and finely-spaced continuous data gets proportionally small noise.
/// `Some(0.0)` disables jitter along that axis.
#[derive(Clone, Debug)]
pub struct PositionJitter {
    /// Horizontal jitter amount in data units; `None` = 0.4 × resolution(x).
    pub width: Option<f64>,
    /// Vertical jitter amount in data units; `None` = 0.4 × resolution(y).
    pub height: Option<f64>,
    /// Seed of the deterministic noise stream.
    pub seed: u64,
}

impl Default for PositionJitter {
    fn default() -> Self {
        PositionJitter {
            width: None,
            height: None,
            seed: JITTER_SEED,
        }
    }
}

impl PositionJitter {
    /// Explicit jitter amounts (data units), default seed.
    pub fn new(width: f64, height: f64) -> Self {
        PositionJitter {
            width: Some(width),
            height: Some(height),
            seed: JITTER_SEED,
        }
    }

    /// Set the horizontal jitter amount (data units).
    pub fn with_width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the vertical jitter amount (data units).
    pub fn with_height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    /// Use a different seed (ggplot2's `position_jitter(seed = …)`).
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

/// ggplot2's `resolution(x, zero = FALSE)`: the smallest positive gap between
/// distinct finite values; 1 for integer-typed data, a single distinct value, or
/// no numeric values at all.
pub fn resolution(values: &[Value]) -> f64 {
    let mut all_int = true;
    let mut xs: Vec<f64> = Vec::with_capacity(values.len());
    for v in values {
        match v {
            Value::Integer(i) => xs.push(*i as f64),
            Value::Float(f) if f.is_finite() => {
                all_int = false;
                xs.push(*f);
            }
            Value::DateTime(s) => {
                all_int = false;
                xs.push(*s as f64);
            }
            _ => {}
        }
    }
    if all_int || xs.len() < 2 {
        return 1.0;
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    let span = xs[xs.len() - 1] - xs[0];
    // Ignore floating-point dust between "equal" values (ggplot2 uses a
    // tolerance of sqrt(.Machine$double.eps) relative to the range).
    let tol = span.abs() * 1e-8;
    let min_gap = xs
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|d| *d > tol)
        .fold(f64::INFINITY, f64::min);
    if min_gap.is_finite() && min_gap > 0.0 {
        min_gap
    } else {
        1.0
    }
}

fn jitter_column(col: &mut [Value], amount: f64, rng: &mut SplitMix64) {
    for v in col.iter_mut() {
        if let Some(x) = v.as_f64() {
            *v = Value::Float(x + rng.range_f64(-amount, amount));
        }
    }
}

impl Position for PositionJitter {
    fn compute(&self, data: &mut DataFrame, _params: &PositionParams) {
        let mut rng = SplitMix64::new(self.seed);

        if let Some(x_col) = data.column_mut("x") {
            let w = self.width.unwrap_or_else(|| 0.4 * resolution(x_col));
            if w > 0.0 && w.is_finite() {
                jitter_column(x_col, w, &mut rng);
            }
        }

        if let Some(y_col) = data.column_mut("y") {
            let h = self.height.unwrap_or_else(|| 0.4 * resolution(y_col));
            if h > 0.0 && h.is_finite() {
                jitter_column(y_col, h, &mut rng);
            }
        }
    }

    fn name(&self) -> &str {
        "jitter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(x: Vec<Value>, y: Vec<Value>) -> DataFrame {
        let mut df = DataFrame::new();
        df.add_column("x".into(), x);
        df.add_column("y".into(), y);
        df
    }

    fn floats(v: &[f64]) -> Vec<Value> {
        v.iter().map(|f| Value::Float(*f)).collect()
    }

    #[test]
    fn resolution_matches_ggplot2() {
        assert_eq!(resolution(&floats(&[1.0, 2.0, 3.0])), 1.0);
        assert_eq!(resolution(&floats(&[0.1, 0.3, 0.2])), 0.09999999999999998);
        assert_eq!(resolution(&floats(&[5.0])), 1.0);
        assert_eq!(resolution(&floats(&[2.0, 2.0])), 1.0);
        assert_eq!(
            resolution(&[Value::Integer(10), Value::Integer(20)]),
            1.0,
            "integer data has resolution 1"
        );
        assert_eq!(resolution(&[Value::Str("a".into())]), 1.0);
    }

    #[test]
    fn default_width_scales_with_resolution() {
        // Values spaced 0.01 apart: jitter must stay within ±0.004.
        let xs: Vec<f64> = (0..50).map(|i| i as f64 * 0.01).collect();
        let mut df = frame(floats(&xs), floats(&xs));
        PositionJitter::default().compute(&mut df, &PositionParams::default());
        for (orig, v) in xs.iter().zip(df.column("x").unwrap()) {
            let d = (v.as_f64().unwrap() - orig).abs();
            assert!(d <= 0.004 + 1e-12, "moved {d}");
        }
        // Integer-coded positions keep the classic ±0.4.
        let mut df = frame(floats(&[1.0, 2.0, 3.0]), floats(&[1.0, 2.0, 3.0]));
        PositionJitter::default().compute(&mut df, &PositionParams::default());
        let moved: Vec<f64> = df
            .column("x")
            .unwrap()
            .iter()
            .zip([1.0, 2.0, 3.0])
            .map(|(v, o)| (v.as_f64().unwrap() - o).abs())
            .collect();
        assert!(moved.iter().all(|d| *d <= 0.4));
        assert!(moved.iter().any(|d| *d > 0.01));
    }

    #[test]
    fn seed_controls_output() {
        let run = |p: PositionJitter| {
            let mut df = frame(floats(&[1.0, 2.0, 3.0]), floats(&[1.0, 2.0, 3.0]));
            p.compute(&mut df, &PositionParams::default());
            df.column("x").unwrap().to_vec()
        };
        assert_eq!(
            run(PositionJitter::default()),
            run(PositionJitter::default())
        );
        assert_ne!(
            run(PositionJitter::default()),
            run(PositionJitter::default().with_seed(1))
        );
        assert_eq!(
            run(PositionJitter::default().with_seed(1)),
            run(PositionJitter::default().with_seed(1))
        );
    }

    #[test]
    fn zero_amount_disables_axis() {
        let mut df = frame(floats(&[1.0, 2.0]), floats(&[5.0, 6.0]));
        PositionJitter::new(0.3, 0.0).compute(&mut df, &PositionParams::default());
        assert_eq!(df.column("y").unwrap(), &floats(&[5.0, 6.0])[..]);
        assert_ne!(df.column("x").unwrap(), &floats(&[1.0, 2.0])[..]);
    }
}
