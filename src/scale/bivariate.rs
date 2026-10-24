//! Bivariate fill scale: two binned variables → an `n × m` palette
//! (biscale-style; Stevens 1994's bivariate-choropleth method), backing a
//! `::BIVARIATE` choropleth. Ported from the design in
//! anofox-visualization's `docs/plans/plan-maps.md` (§15, D10) — DataZooDE/
//! anofox-visualization#318, #182.
//!
//! The *binning itself* (computing each variable's tertile/class index) is
//! the caller's job, same as [`super::binned::ScaleFillBinned`]'s break
//! maths — this scale only colours an already-binned composite class.
//! Follow the `biscale` R package's own convention: a composite class is a
//! 1-indexed `"{x_class}-{y_class}"` string (e.g. `"3-2"` = x-tertile 3,
//! y-tertile 2), the value a `bi_class()`-style data-prep step would put in
//! a `fill` column.

use crate::aes::Aesthetic;
use crate::data::Value;

use super::Scale;

/// The colour for class `(i, j)` (0-indexed) of an `nx × ny` grid: each
/// channel of `base` is pulled toward the matching channel of `color_x` by
/// `tx = i/(nx-1)` and toward `color_y` by `ty = j/(ny-1)`, subtracted
/// (not averaged) so the two axes compound rather than cancel —
/// `base − tx·(base − color_x) − ty·(base − color_y)`, clamped to
/// `0..=255`. At `(0, 0)` this is exactly `base`; at `(nx-1, 0)` exactly
/// `color_x`; at `(0, ny-1)` exactly `color_y`; at the high-high corner the
/// two subtractions compound (usually clamping low), the classic
/// bivariate-choropleth "dark/saturated corner" look. `base` is the
/// low-low cell's colour — white for a light theme; pass a theme's card
/// colour for a "lifted over the card colour" dark variant (plan-maps.md
/// §15's D10) instead of guessing a theme from inside this crate.
pub fn bivariate_cell_color(
    base: (u8, u8, u8),
    color_x: (u8, u8, u8),
    color_y: (u8, u8, u8),
    nx: usize,
    ny: usize,
    i: usize,
    j: usize,
) -> (u8, u8, u8) {
    let tx = if nx <= 1 {
        1.0
    } else {
        i as f64 / (nx - 1) as f64
    };
    let ty = if ny <= 1 {
        1.0
    } else {
        j as f64 / (ny - 1) as f64
    };
    let chan = |b: u8, x: u8, y: u8| -> u8 {
        let v = b as f64 - tx * (b as f64 - x as f64) - ty * (b as f64 - y as f64);
        v.clamp(0.0, 255.0).round() as u8
    };
    (
        chan(base.0, color_x.0, color_y.0),
        chan(base.1, color_x.1, color_y.1),
        chan(base.2, color_x.2, color_y.2),
    )
}

/// Parse a biscale-style `"{x}-{y}"` composite class (1-indexed) into a
/// 0-indexed `(i, j)`, clamped into `0..nx`/`0..ny`.
fn parse_class(s: &str, nx: usize, ny: usize) -> Option<(usize, usize)> {
    let (xs, ys) = s.split_once('-')?;
    let x: i64 = xs.trim().parse().ok()?;
    let y: i64 = ys.trim().parse().ok()?;
    let i = (x - 1).clamp(0, nx as i64 - 1) as usize;
    let j = (y - 1).clamp(0, ny as i64 - 1) as usize;
    Some((i, j))
}

/// Bivariate fill scale: `nx × ny` classes, each class `"{i}-{j}"`
/// (1-indexed) coloured by [`bivariate_cell_color`]. See the
/// [module docs](self).
#[derive(Clone, Debug)]
pub struct ScaleFillBivariate {
    aesthetic: Aesthetic,
    nx: usize,
    ny: usize,
    base: (u8, u8, u8),
    color_x: (u8, u8, u8),
    color_y: (u8, u8, u8),
    x_name: String,
    y_name: String,
    name: String,
}

impl ScaleFillBivariate {
    /// `nx × ny` classes (biscale's usual is 3×3), tinted from `base`
    /// toward `color_x`/`color_y`. `x_name`/`y_name` are the two legend
    /// axis captions (the mapped variables' names).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        aesthetic: Aesthetic,
        nx: usize,
        ny: usize,
        base: (u8, u8, u8),
        color_x: (u8, u8, u8),
        color_y: (u8, u8, u8),
        x_name: impl Into<String>,
        y_name: impl Into<String>,
    ) -> Self {
        ScaleFillBivariate {
            aesthetic,
            nx: nx.max(1),
            ny: ny.max(1),
            base,
            color_x,
            color_y,
            x_name: x_name.into(),
            y_name: y_name.into(),
            name: String::new(),
        }
    }

    /// The classic biscale 3×3, white base.
    pub fn tertile(
        aesthetic: Aesthetic,
        color_x: (u8, u8, u8),
        color_y: (u8, u8, u8),
        x_name: impl Into<String>,
        y_name: impl Into<String>,
    ) -> Self {
        Self::new(
            aesthetic,
            3,
            3,
            (255, 255, 255),
            color_x,
            color_y,
            x_name,
            y_name,
        )
    }

    pub fn nx(&self) -> usize {
        self.nx
    }
    pub fn ny(&self) -> usize {
        self.ny
    }
    pub fn base(&self) -> (u8, u8, u8) {
        self.base
    }
    pub fn color_x(&self) -> (u8, u8, u8) {
        self.color_x
    }
    pub fn color_y(&self) -> (u8, u8, u8) {
        self.color_y
    }
    pub fn x_name(&self) -> &str {
        &self.x_name
    }
    pub fn y_name(&self) -> &str {
        &self.y_name
    }

    fn cell_color(&self, i: usize, j: usize) -> (u8, u8, u8) {
        bivariate_cell_color(
            self.base,
            self.color_x,
            self.color_y,
            self.nx,
            self.ny,
            i,
            j,
        )
    }

    fn class_label(&self, i: usize, j: usize) -> String {
        format!("{}-{}", i + 1, j + 1)
    }
}

impl Scale for ScaleFillBivariate {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    /// No-op: the domain is the fixed `nx × ny` grid, not derived from
    /// data (same reasoning as [`super::steps::ScaleColorSteps`]/
    /// [`super::binned::ScaleFillBinned`]).
    fn train(&mut self, _values: &[Value]) {}

    fn map(&self, _value: &Value) -> f64 {
        0.0
    }

    /// Every `(i, j)` cell, in row-major order — the full grid regardless
    /// of which classes actually appear in the data, so a legend (a
    /// dedicated grid, not the standard discrete-swatch list — see
    /// [`crate::geom::bivariate_legend`]) always shows the complete
    /// palette.
    fn breaks(&self) -> Vec<(f64, String)> {
        let mut out = Vec::with_capacity(self.nx * self.ny);
        for j in 0..self.ny {
            for i in 0..self.nx {
                out.push((0.0, self.class_label(i, j)));
            }
        }
        out
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
        let s = match value {
            Value::Str(s) => s.as_str(),
            _ => return None,
        };
        let (i, j) = parse_class(s, self.nx, self.ny)?;
        Some(self.cell_color(i, j))
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_low_is_the_base_colour() {
        let s = ScaleFillBivariate::tertile(
            Aesthetic::Fill,
            (220, 0, 0),
            (0, 0, 220),
            "income",
            "education",
        );
        assert_eq!(
            s.map_to_color(&Value::Str("1-1".into())),
            Some((255, 255, 255))
        );
    }

    #[test]
    fn high_x_only_tints_toward_color_x() {
        let s = ScaleFillBivariate::tertile(Aesthetic::Fill, (220, 0, 0), (0, 0, 220), "x", "y");
        let c = s.map_to_color(&Value::Str("3-1".into())).unwrap();
        // Multiply-blended with white (255,255,255) in the y channel is a
        // no-op, so this should equal plain color_x.
        assert_eq!(c, (220, 0, 0));
    }

    #[test]
    fn high_high_is_darker_than_either_axis_alone() {
        let s = ScaleFillBivariate::tertile(Aesthetic::Fill, (220, 0, 0), (0, 0, 220), "x", "y");
        let hh = s.map_to_color(&Value::Str("3-3".into())).unwrap();
        let hx = s.map_to_color(&Value::Str("3-1".into())).unwrap();
        let hy = s.map_to_color(&Value::Str("1-3".into())).unwrap();
        let luma = |(r, g, b): (u8, u8, u8)| 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
        assert!(luma(hh) < luma(hx), "high-high darker than high-x-only");
        assert!(luma(hh) < luma(hy), "high-high darker than high-y-only");
    }

    #[test]
    fn all_nine_cells_are_distinguishable() {
        let s =
            ScaleFillBivariate::tertile(Aesthetic::Fill, (217, 95, 2), (117, 112, 179), "x", "y");
        let mut colors = Vec::new();
        for j in 1..=3 {
            for i in 1..=3 {
                colors.push(s.map_to_color(&Value::Str(format!("{i}-{j}"))).unwrap());
            }
        }
        for a in 0..colors.len() {
            for b in (a + 1)..colors.len() {
                assert_ne!(
                    colors[a], colors[b],
                    "cells {a} and {b} collide: {colors:?}"
                );
            }
        }
    }

    /// sRGB → CIE Lab, for the ΔE (CIE76) check below. Standard D65
    /// conversion (sRGB → linear → XYZ → Lab).
    fn srgb_to_lab((r, g, b): (u8, u8, u8)) -> (f64, f64, f64) {
        let to_linear = |c: u8| {
            let c = c as f64 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        let (rl, gl, bl) = (to_linear(r), to_linear(g), to_linear(b));
        // sRGB -> XYZ (D65)
        let x = rl * 0.4124564 + gl * 0.3575761 + bl * 0.1804375;
        let y = rl * 0.2126729 + gl * 0.7151522 + bl * 0.0721750;
        let z = rl * 0.0193339 + gl * 0.1191920 + bl * 0.9503041;
        // D65 white point
        let (xn, yn, zn) = (0.95047, 1.0, 1.08883);
        let f = |t: f64| {
            if t > (6.0f64 / 29.0).powi(3) {
                t.cbrt()
            } else {
                t / (3.0 * (6.0f64 / 29.0).powi(2)) + 4.0 / 29.0
            }
        };
        let (fx, fy, fz) = (f(x / xn), f(y / yn), f(z / zn));
        (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
    }

    /// CIE76 ΔE: Euclidean distance in Lab space.
    fn delta_e(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
        let (l1, a1, b1) = srgb_to_lab(a);
        let (l2, a2, b2) = srgb_to_lab(b);
        ((l1 - l2).powi(2) + (a1 - a2).powi(2) + (b1 - b2).powi(2)).sqrt()
    }

    /// plan-maps.md §15 D10's own acceptance test: the 9 cells stay
    /// perceptually distinguishable (CIE76 ΔE above a "just noticeable
    /// difference" floor) in both a light-theme (white base) and a
    /// dark-theme ("lifted over the card colour") palette.
    #[test]
    fn all_nine_cells_pass_a_cie76_delta_e_floor_in_light_and_dark() {
        const JND: f64 = 5.0;
        for base in [(255, 255, 255), (45, 45, 48)] {
            let s = ScaleFillBivariate::new(
                Aesthetic::Fill,
                3,
                3,
                base,
                (217, 95, 2),
                (117, 112, 179),
                "x",
                "y",
            );
            let mut colors = Vec::new();
            for j in 1..=3 {
                for i in 1..=3 {
                    colors.push(s.map_to_color(&Value::Str(format!("{i}-{j}"))).unwrap());
                }
            }
            for a in 0..colors.len() {
                for b in (a + 1)..colors.len() {
                    let de = delta_e(colors[a], colors[b]);
                    assert!(
                        de > JND,
                        "base {base:?}: cells {a} ({:?}) and {b} ({:?}) too close: ΔE={de:.2}",
                        colors[a],
                        colors[b]
                    );
                }
            }
        }
    }

    #[test]
    fn class_label_round_trips() {
        let s = ScaleFillBivariate::tertile(Aesthetic::Fill, (1, 2, 3), (4, 5, 6), "x", "y");
        for (_, label) in s.breaks() {
            assert!(
                s.map_to_color(&Value::Str(label.clone())).is_some(),
                "{label}"
            );
        }
        assert_eq!(s.breaks().len(), 9);
    }

    #[test]
    fn out_of_range_class_clamps_instead_of_panicking() {
        let s = ScaleFillBivariate::tertile(Aesthetic::Fill, (1, 2, 3), (4, 5, 6), "x", "y");
        assert_eq!(
            s.map_to_color(&Value::Str("9-9".into())),
            s.map_to_color(&Value::Str("3-3".into()))
        );
        assert_eq!(
            s.map_to_color(&Value::Str("0-0".into())),
            s.map_to_color(&Value::Str("1-1".into()))
        );
    }

    #[test]
    fn malformed_class_string_returns_none() {
        let s = ScaleFillBivariate::tertile(Aesthetic::Fill, (1, 2, 3), (4, 5, 6), "x", "y");
        assert_eq!(s.map_to_color(&Value::Str("nope".into())), None);
        assert_eq!(s.map_to_color(&Value::Float(1.0)), None);
    }

    #[test]
    fn custom_grid_shape_and_base() {
        let s = ScaleFillBivariate::new(
            Aesthetic::Fill,
            4,
            2,
            (240, 240, 240),
            (200, 0, 0),
            (0, 0, 200),
            "x",
            "y",
        );
        assert_eq!(s.nx(), 4);
        assert_eq!(s.ny(), 2);
        assert_eq!(s.breaks().len(), 8);
        assert_eq!(
            s.map_to_color(&Value::Str("1-1".into())),
            Some((240, 240, 240))
        );
    }
}
