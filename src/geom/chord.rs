//! Chord diagram / dependency wheel: `geom_chord`. Nodes are arcs around a
//! circle; each link is a ribbon from its source's band to its target's band
//! through the centre. Layout by [`StatChord`].

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::chord::StatChord;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Chord diagram. Map `from`, `to` and optionally `weight`; node arcs are
/// coloured through the fill scale by name, ribbons by their source.
pub struct GeomChord {
    /// Gap between neighbouring node arcs, in radians.
    pub pad_angle: f64,
    /// Node arc thickness as a fraction of the radius.
    pub arc_width: f64,
    /// Fill used when no fill scale applies.
    pub fill: (u8, u8, u8),
    pub node_alpha: f64,
    pub link_alpha: f64,
    /// Draw node names around the circle.
    pub labels: bool,
    pub label_size: f64,
    pub label_color: (u8, u8, u8),
}

impl Default for GeomChord {
    fn default() -> Self {
        GeomChord {
            pad_angle: 0.04,
            arc_width: 0.07,
            fill: (97, 156, 255),
            node_alpha: 1.0,
            link_alpha: 0.55,
            labels: true,
            label_size: 10.0,
            label_color: (40, 40, 40),
        }
    }
}

impl GeomChord {
    /// The stat's trained half-extents `(x, y)`; the builder derives the
    /// panel aspect ratio from them so the circle stays round.
    pub(crate) fn extents(&self) -> (f64, f64) {
        let s = StatChord::default();
        (s.x_extent, s.y_extent)
    }
}

/// Point on the circle of radius `r` at angle `a` (clockwise from 12 o'clock).
fn polar(a: f64, r: f64) -> (f64, f64) {
    (r * a.sin(), r * a.cos())
}

/// Points along the arc `a0..a1` at radius `r` (inclusive of both ends).
fn arc(a0: f64, a1: f64, r: f64, out: &mut Vec<(f64, f64)>) {
    let steps = (((a1 - a0).abs() / 0.03).ceil() as usize).clamp(2, 400);
    for k in 0..=steps {
        out.push(polar(a0 + (a1 - a0) * k as f64 / steps as f64, r));
    }
}

/// Quadratic Bézier from `p` to `q` with the control point at the centre.
fn through_centre(p: (f64, f64), q: (f64, f64), out: &mut Vec<(f64, f64)>) {
    const STEPS: usize = 24;
    for k in 1..STEPS {
        let t = k as f64 / STEPS as f64;
        let (u, v) = ((1.0 - t) * (1.0 - t), t * t);
        out.push((u * p.0 + v * q.0, u * p.1 + v * q.1));
    }
}

impl Geom for GeomChord {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let col = |n: &str| {
            data.column(n)
                .ok_or_else(|| RenderError::MissingAesthetic(n.into()))
        };
        let (part, a0c, a1c, b0c, b1c) =
            (col("part")?, col("a0")?, col("a1")?, col("b0")?, col("b1")?);
        let (node, from, to, value) = (col("node")?, col("from")?, col("to")?, col("value")?);
        let fill_col = data.column("fill");
        let area = backend.plot_area();
        let map = |(x, y): (f64, f64)| {
            let sx = scales.get(&Aesthetic::X).map(|s| s.map(&Value::Float(x)));
            let sy = scales.get(&Aesthetic::Y).map(|s| s.map(&Value::Float(y)));
            coord.transform((sx.unwrap_or(0.5), sy.unwrap_or(0.5)), &area)
        };
        let fill_of = |i: usize| {
            fill_col
                .and_then(|c| scales.map_color(&Aesthetic::Fill, &c[i]))
                .unwrap_or(self.fill)
        };
        let f = |c: &[Value], i: usize| c[i].as_f64().unwrap_or(f64::NAN);
        let is = |i: usize, p: &str| part[i].key_str() == p;
        let r_in = 1.0 - self.arc_width;
        let style = |fill, alpha| RectStyle {
            fill: Some(fill),
            stroke: None,
            stroke_width: 0.0,
            alpha,
            clip: true,
            corner_radius: 0.0,
        };

        for i in (0..data.nrows()).filter(|&i| is(i, "link")) {
            let (a0, a1, b0, b1) = (f(a0c, i), f(a1c, i), f(b0c, i), f(b1c, i));
            if ![a0, a1, b0, b1].iter().all(|v| v.is_finite()) {
                continue;
            }
            let r = r_in * 0.99;
            let mut pts = Vec::new();
            arc(a0, a1, r, &mut pts);
            through_centre(polar(a1, r), polar(b0, r), &mut pts);
            arc(b0, b1, r, &mut pts);
            through_centre(polar(b1, r), polar(a0, r), &mut pts);
            let pts: Vec<(f64, f64)> = pts.into_iter().map(map).collect();
            let (a, b) = (super::tip_value(&from[i]), super::tip_value(&to[i]));
            super::set_mark(
                backend,
                Some(format!("{a} → {b}: {}", super::tip_value(&value[i]))),
                Some(a.clone()),
                Some(a),
                super::raw_value(&value[i]),
            );
            backend.draw_polygon(&pts, &style(fill_of(i), self.link_alpha))?;
        }

        let mut labels = Vec::new();
        for i in (0..data.nrows()).filter(|&i| is(i, "node")) {
            let (a0, a1) = (f(a0c, i), f(a1c, i));
            if !(a0.is_finite() && a1.is_finite()) {
                continue;
            }
            let mut pts = Vec::new();
            arc(a0, a1, 1.0, &mut pts);
            let mut inner = Vec::new();
            arc(a0, a1, r_in, &mut inner);
            pts.extend(inner.into_iter().rev());
            let pts: Vec<(f64, f64)> = pts.into_iter().map(map).collect();
            let name = super::tip_value(&node[i]);
            super::set_mark(
                backend,
                Some(format!("{name}: {}", super::tip_value(&value[i]))),
                Some(name.clone()),
                Some(name.clone()),
                super::raw_value(&value[i]),
            );
            backend.draw_polygon(&pts, &style(fill_of(i), self.node_alpha))?;
            if self.labels {
                let mid = (a0 + a1) / 2.0;
                let side = mid.sin();
                let anchor = if side.abs() < 0.12 {
                    TextAnchor::Middle
                } else if side > 0.0 {
                    TextAnchor::Start
                } else {
                    TextAnchor::End
                };
                // Labels near 12/6 o'clock sit a little further out so they
                // clear the arc vertically.
                let r = 1.06 + 0.06 * mid.cos().abs().powi(4);
                labels.push((name, map(polar(mid, r)), anchor));
            }
        }
        super::clear_mark(backend);
        let family = super::resolve_font_family("", theme);
        for (text, pos, anchor) in labels {
            backend.draw_text(
                &text,
                pos,
                &TextStyle {
                    color: self.label_color,
                    size: self.label_size,
                    anchor,
                    angle: 0.0,
                    family: family.clone(),
                    face: FontFace::Plain,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::From, Aesthetic::To]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatChord {
            pad_angle: self.pad_angle,
            ..Default::default()
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "chord"
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let Some(ink) = el.ink {
            self.label_color = ink;
        }
    }
}
