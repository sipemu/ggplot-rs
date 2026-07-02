use crate::aes::Aesthetic;
use crate::data::Value;

use super::Scale;

/// Binned (stepped) continuous colour scale — R's `scale_*_steps` /
/// `scale_*_stepsn` / `scale_*_fermenter`.
///
/// A continuous variable is bucketed into `n_bins` equal-width bins over the
/// trained domain, and each bin is filled with a discrete colour sampled from
/// the gradient. It reports as discrete, so the legend shows one stepped swatch
/// per bin (a `guide_bins`/`guide_coloursteps`-style legend).
#[derive(Clone)]
pub struct ScaleColorSteps {
    aesthetic: Aesthetic,
    stops: Vec<(u8, u8, u8)>,
    n_bins: usize,
    name: String,
    min: f64,
    max: f64,
    trained: bool,
}

impl ScaleColorSteps {
    /// Build from evenly-spaced gradient stops (`>= 1`) and a bin count.
    pub fn new(aesthetic: Aesthetic, stops: Vec<(u8, u8, u8)>, n_bins: usize) -> Self {
        ScaleColorSteps {
            aesthetic,
            stops: if stops.is_empty() {
                vec![(50, 50, 200), (200, 50, 50)]
            } else {
                stops
            },
            n_bins: n_bins.max(1),
            name: String::new(),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            trained: false,
        }
    }

    /// Two-colour low→high binned scale.
    pub fn two(aesthetic: Aesthetic, low: (u8, u8, u8), high: (u8, u8, u8), n_bins: usize) -> Self {
        Self::new(aesthetic, vec![low, high], n_bins)
    }

    fn interp(&self, f: f64) -> (u8, u8, u8) {
        let f = f.clamp(0.0, 1.0);
        if self.stops.len() == 1 {
            return self.stops[0];
        }
        let segs = self.stops.len() - 1;
        let pos = f * segs as f64;
        let i = (pos.floor() as usize).min(segs - 1);
        let t = pos - i as f64;
        let (r0, g0, b0) = self.stops[i];
        let (r1, g1, b1) = self.stops[i + 1];
        let lerp = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
        (lerp(r0, r1), lerp(g0, g1), lerp(b0, b1))
    }

    fn bin_index(&self, v: f64) -> usize {
        if !self.trained || self.max <= self.min {
            return 0;
        }
        let frac = (v - self.min) / (self.max - self.min);
        ((frac * self.n_bins as f64).floor() as isize).clamp(0, self.n_bins as isize - 1) as usize
    }

    fn bin_color(&self, i: usize) -> (u8, u8, u8) {
        // Colour at the bin centre.
        self.interp((i as f64 + 0.5) / self.n_bins as f64)
    }

    fn bin_label(&self, i: usize) -> String {
        if !self.trained || self.max <= self.min {
            return format!("bin {i}");
        }
        let w = (self.max - self.min) / self.n_bins as f64;
        let lo = self.min + i as f64 * w;
        format!("[{:.2}, {:.2})", lo, lo + w)
    }
}

impl Scale for ScaleColorSteps {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    fn train(&mut self, values: &[Value]) {
        for v in values {
            if let Some(f) = v.as_f64() {
                if f.is_finite() {
                    self.min = self.min.min(f);
                    self.max = self.max.max(f);
                    self.trained = true;
                }
            }
        }
    }

    fn map(&self, _value: &Value) -> f64 {
        0.0
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        (0..self.n_bins)
            .map(|i| ((i as f64 + 0.5) / self.n_bins as f64, self.bin_label(i)))
            .collect()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn is_discrete(&self) -> bool {
        true
    }

    fn map_to_color(&self, value: &Value) -> Option<(u8, u8, u8)> {
        if let Some(f) = value.as_f64() {
            return Some(self.bin_color(self.bin_index(f)));
        }
        // Legend passes the bin label back as a string — match it to a bin.
        if let Value::Str(s) = value {
            for i in 0..self.n_bins {
                if &self.bin_label(i) == s {
                    return Some(self.bin_color(i));
                }
            }
        }
        None
    }

    fn domain(&self) -> Option<(f64, f64)> {
        if self.trained {
            Some((self.min, self.max))
        } else {
            None
        }
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        self.min = f64::INFINITY;
        self.max = f64::NEG_INFINITY;
        self.trained = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trained(n: usize) -> ScaleColorSteps {
        let mut s = ScaleColorSteps::two(Aesthetic::Color, (0, 0, 0), (255, 255, 255), n);
        s.train(&[Value::Float(0.0), Value::Float(100.0)]);
        s
    }

    #[test]
    fn bins_map_to_distinct_colors() {
        let s = trained(4);
        let low = s.map_to_color(&Value::Float(5.0)).unwrap();
        let high = s.map_to_color(&Value::Float(95.0)).unwrap();
        assert_ne!(low, high);
        // Endpoints clamp within range.
        assert_eq!(
            s.map_to_color(&Value::Float(-10.0)),
            s.map_to_color(&Value::Float(0.0))
        );
    }

    #[test]
    fn same_bin_same_color() {
        let s = trained(4); // bins of width 25
        let a = s.map_to_color(&Value::Float(1.0)).unwrap();
        let b = s.map_to_color(&Value::Float(24.0)).unwrap();
        assert_eq!(a, b, "values in the same bin share a colour");
    }

    #[test]
    fn legend_label_roundtrips_to_bin_color() {
        let s = trained(5);
        let breaks = s.breaks();
        assert_eq!(breaks.len(), 5);
        // Each break label must map (as a string) to the same colour as the bin.
        for (i, (_, label)) in breaks.iter().enumerate() {
            let via_label = s.map_to_color(&Value::Str(label.clone())).unwrap();
            assert_eq!(via_label, s.bin_color(i));
        }
    }
}
