//! A color scale that wraps another scale and adjusts the lightness of its
//! mapped colors — the backing implementation for `after_scale` color
//! derivations (e.g. a fill that is a darker version of the mapped color).

use crate::aes::Aesthetic;
use crate::data::Value;
use crate::render::backend::{Linetype, PointShape};

use super::sec_axis::SecAxis;
use super::Scale;

/// Adjust an RGB color's lightness: `f > 0` blends toward white, `f < 0` toward
/// black, clamped to `-1.0..=1.0`.
fn adjust_lightness((r, g, b): (u8, u8, u8), f: f64) -> (u8, u8, u8) {
    let f = f.clamp(-1.0, 1.0);
    let ch = |c: u8| -> u8 {
        let c = c as f64;
        let out = if f >= 0.0 {
            c + (255.0 - c) * f
        } else {
            c * (1.0 + f)
        };
        out.round().clamp(0.0, 255.0) as u8
    };
    (ch(r), ch(g), ch(b))
}

/// Wraps a source scale, exposing it under a different (target) aesthetic and
/// lightness-adjusting every mapped color. All non-color behaviour delegates to
/// the inner scale.
pub struct ScaleColorModified {
    inner: Box<dyn Scale>,
    aesthetic: Aesthetic,
    lightness: f64,
}

impl ScaleColorModified {
    pub fn new(inner: Box<dyn Scale>, target: Aesthetic, lightness: f64) -> Self {
        ScaleColorModified {
            inner,
            aesthetic: target,
            lightness,
        }
    }
}

impl Scale for ScaleColorModified {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }
    fn train(&mut self, values: &[Value]) {
        self.inner.train(values)
    }
    fn map(&self, value: &Value) -> f64 {
        self.inner.map(value)
    }
    fn breaks(&self) -> Vec<(f64, String)> {
        self.inner.breaks()
    }
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn set_name(&mut self, name: &str) {
        self.inner.set_name(name)
    }
    fn is_discrete(&self) -> bool {
        self.inner.is_discrete()
    }
    fn map_to_color(&self, value: &Value) -> Option<(u8, u8, u8)> {
        self.inner
            .map_to_color(value)
            .map(|c| adjust_lightness(c, self.lightness))
    }
    fn map_to_shape(&self, value: &Value) -> Option<PointShape> {
        self.inner.map_to_shape(value)
    }
    fn map_to_linetype(&self, value: &Value) -> Option<Linetype> {
        self.inner.map_to_linetype(value)
    }
    fn map_to_size(&self, value: &Value) -> Option<f64> {
        self.inner.map_to_size(value)
    }
    fn map_to_alpha(&self, value: &Value) -> Option<f64> {
        self.inner.map_to_alpha(value)
    }
    fn sec_axis(&self) -> Option<&SecAxis> {
        None
    }
    fn domain(&self) -> Option<(f64, f64)> {
        self.inner.domain()
    }
    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(ScaleColorModified {
            inner: self.inner.clone_box(),
            aesthetic: self.aesthetic.clone(),
            lightness: self.lightness,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darken_and_lighten() {
        assert_eq!(adjust_lightness((100, 100, 100), -0.5), (50, 50, 50));
        assert_eq!(adjust_lightness((100, 100, 100), 0.0), (100, 100, 100));
        assert_eq!(adjust_lightness((100, 100, 100), 1.0), (255, 255, 255));
        assert_eq!(adjust_lightness((100, 100, 100), -1.0), (0, 0, 0));
    }
}
