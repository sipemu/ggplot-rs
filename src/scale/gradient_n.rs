use crate::aes::Aesthetic;
use crate::data::Value;

use super::color::RGBAColor;
use super::util::{format_number, nice_step};
use super::Scale;

/// N-stop continuous color gradient scale.
/// Interpolates linearly between user-defined color stops.
#[derive(Clone, Debug)]
pub struct ScaleColorGradientN {
    aesthetic: Aesthetic,
    name: String,
    /// Color stops as (position_0_to_1, color) pairs, sorted by position.
    stops: Vec<(f64, RGBAColor)>,
    min: f64,
    max: f64,
}

impl ScaleColorGradientN {
    /// Create a new N-stop gradient for the given aesthetic.
    /// Stops are `(position, color)` where position is in [0, 1].
    pub fn new(aesthetic: Aesthetic, stops: Vec<(f64, RGBAColor)>) -> Self {
        let mut stops = stops;
        stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        ScaleColorGradientN {
            aesthetic,
            name: String::new(),
            stops,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    /// Create a continuous viridis palette.
    pub fn viridis(aesthetic: Aesthetic) -> Self {
        Self::new(aesthetic, viridis_stops())
    }

    /// Create a continuous magma palette.
    pub fn magma(aesthetic: Aesthetic) -> Self {
        Self::new(aesthetic, magma_stops())
    }

    /// Create a continuous plasma palette.
    pub fn plasma(aesthetic: Aesthetic) -> Self {
        Self::new(aesthetic, plasma_stops())
    }

    /// Create a continuous inferno palette.
    pub fn inferno(aesthetic: Aesthetic) -> Self {
        Self::new(aesthetic, inferno_stops())
    }

    /// Interpolate the color at a normalized position t in [0, 1].
    fn color_at(&self, t: f64) -> RGBAColor {
        let t = t.clamp(0.0, 1.0);
        if self.stops.is_empty() {
            return RGBAColor::new(127, 127, 127);
        }
        if self.stops.len() == 1 {
            return self.stops[0].1;
        }
        // Find the two surrounding stops
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        if t >= self.stops[self.stops.len() - 1].0 {
            return self.stops[self.stops.len() - 1].1;
        }
        for i in 0..self.stops.len() - 1 {
            let (p0, c0) = &self.stops[i];
            let (p1, c1) = &self.stops[i + 1];
            if t >= *p0 && t <= *p1 {
                let range = p1 - p0;
                let local_t = if range.abs() < f64::EPSILON {
                    0.0
                } else {
                    (t - p0) / range
                };
                return c0.lerp(c1, local_t);
            }
        }
        self.stops[self.stops.len() - 1].1
    }
}

impl Scale for ScaleColorGradientN {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    fn train(&mut self, values: &[Value]) {
        for v in values {
            if let Some(f) = v.as_f64() {
                if f.is_finite() {
                    if f < self.min {
                        self.min = f;
                    }
                    if f > self.max {
                        self.max = f;
                    }
                }
            }
        }
    }

    fn map(&self, value: &Value) -> f64 {
        let f = match value.as_f64() {
            Some(f) => f,
            None => return 0.0,
        };
        let range = self.max - self.min;
        if range.abs() < f64::EPSILON {
            0.5
        } else {
            (f - self.min) / range
        }
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        if self.min > self.max || !self.min.is_finite() || !self.max.is_finite() {
            return vec![];
        }
        let range = self.max - self.min;
        if range.abs() < f64::EPSILON {
            return vec![(0.5, format_number(self.min))];
        }
        let n_breaks = 5;
        let raw_step = range / n_breaks as f64;
        let step = nice_step(raw_step);
        let start = (self.min / step).ceil() * step;
        let mut breaks = Vec::new();
        let mut v = start;
        while v <= self.max + step * 0.001 {
            let pos = self.map(&Value::Float(v));
            breaks.push((pos, format_number(v)));
            v += step;
        }
        breaks
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn map_to_color(&self, value: &Value) -> Option<(u8, u8, u8)> {
        let t = self.map(value);
        let c = self.color_at(t);
        Some((c.r, c.g, c.b))
    }

    fn domain(&self) -> Option<(f64, f64)> {
        if self.min.is_finite() && self.max.is_finite() && self.min <= self.max {
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
    }
}

// ─── Continuous palette color stops ──────────────────────────────

fn c(r: u8, g: u8, b: u8) -> RGBAColor {
    RGBAColor::new(r, g, b)
}

fn viridis_stops() -> Vec<(f64, RGBAColor)> {
    let colors = [
        c(68, 1, 84),
        c(72, 26, 108),
        c(71, 47, 126),
        c(65, 68, 135),
        c(57, 86, 140),
        c(47, 104, 142),
        c(38, 121, 142),
        c(31, 138, 141),
        c(30, 155, 138),
        c(42, 172, 130),
        c(70, 188, 115),
        c(109, 202, 93),
        c(155, 213, 67),
        c(200, 222, 39),
        c(240, 229, 30),
        c(253, 231, 37),
    ];
    evenly_spaced_stops(&colors)
}

fn magma_stops() -> Vec<(f64, RGBAColor)> {
    let colors = [
        c(0, 0, 4),
        c(16, 12, 50),
        c(41, 17, 90),
        c(72, 12, 110),
        c(101, 19, 110),
        c(131, 29, 103),
        c(160, 42, 93),
        c(187, 55, 84),
        c(213, 72, 72),
        c(232, 99, 62),
        c(247, 131, 57),
        c(254, 167, 69),
        c(254, 203, 99),
        c(252, 235, 141),
        c(252, 254, 188),
        c(252, 253, 191),
    ];
    evenly_spaced_stops(&colors)
}

fn plasma_stops() -> Vec<(f64, RGBAColor)> {
    let colors = [
        c(13, 8, 135),
        c(53, 5, 157),
        c(82, 1, 163),
        c(109, 1, 159),
        c(133, 7, 147),
        c(156, 23, 127),
        c(175, 42, 106),
        c(192, 61, 85),
        c(206, 82, 66),
        c(218, 105, 46),
        c(228, 130, 24),
        c(236, 157, 6),
        c(240, 185, 11),
        c(239, 213, 38),
        c(232, 240, 73),
        c(240, 249, 33),
    ];
    evenly_spaced_stops(&colors)
}

fn inferno_stops() -> Vec<(f64, RGBAColor)> {
    let colors = [
        c(0, 0, 4),
        c(14, 11, 49),
        c(39, 15, 90),
        c(67, 10, 107),
        c(95, 13, 106),
        c(122, 21, 97),
        c(149, 33, 81),
        c(174, 49, 60),
        c(196, 69, 38),
        c(215, 95, 15),
        c(231, 124, 3),
        c(243, 155, 7),
        c(250, 189, 28),
        c(252, 222, 67),
        c(247, 252, 118),
        c(252, 255, 164),
    ];
    evenly_spaced_stops(&colors)
}

fn evenly_spaced_stops(colors: &[RGBAColor]) -> Vec<(f64, RGBAColor)> {
    let n = colors.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![(0.0, colors[0])];
    }
    colors
        .iter()
        .enumerate()
        .map(|(i, c)| (i as f64 / (n - 1) as f64, *c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_n_interpolation() {
        let g = ScaleColorGradientN::new(
            Aesthetic::Color,
            vec![
                (0.0, RGBAColor::new(0, 0, 0)),
                (0.5, RGBAColor::new(255, 0, 0)),
                (1.0, RGBAColor::new(255, 255, 255)),
            ],
        );
        // At t=0 should be black
        let c0 = g.color_at(0.0);
        assert_eq!((c0.r, c0.g, c0.b), (0, 0, 0));
        // At t=0.5 should be red
        let c5 = g.color_at(0.5);
        assert_eq!((c5.r, c5.g, c5.b), (255, 0, 0));
        // At t=1.0 should be white
        let c1 = g.color_at(1.0);
        assert_eq!((c1.r, c1.g, c1.b), (255, 255, 255));
        // At t=0.25 should be ~midpoint between black and red
        let c25 = g.color_at(0.25);
        assert_eq!(c25.r, 127); // roughly half of 255
    }

    #[test]
    fn test_viridis_continuous_endpoints() {
        let g = ScaleColorGradientN::viridis(Aesthetic::Fill);
        let c0 = g.color_at(0.0);
        assert_eq!((c0.r, c0.g, c0.b), (68, 1, 84));
        let c1 = g.color_at(1.0);
        assert_eq!((c1.r, c1.g, c1.b), (253, 231, 37));
    }
}
