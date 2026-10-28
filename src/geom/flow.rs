//! Flow maps: `geom_flow`. Origin → destination links between geographic
//! points (`x`/`y` = origin lon/lat, `xend`/`yend` = destination), drawn as
//! curved bands whose width is the `weight`, with an arrowhead at the
//! destination. Endpoints are projected with the layer's
//! [`SfProjection`] (match the `geom_sf` layers' projection).

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::spatial::SfProjection;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::arrow::{arrow, draw_heads, heads, ArrowType};
use super::{Geom, GeomParams};

/// Flow map. Map `x`, `y`, `xend`, `yend` (lon/lat), `weight`, optionally
/// `color` and `label`.
pub struct GeomFlow {
    pub projection: SfProjection,
    /// Bend as a fraction of the link length (0 = straight; sign flips side).
    pub curvature: f64,
    /// Band width range in pixels (by weight).
    pub width: (f64, f64),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    /// Arrowhead at the destination.
    pub arrows: bool,
}

impl Default for GeomFlow {
    fn default() -> Self {
        GeomFlow {
            projection: SfProjection::PlateCarree,
            curvature: 0.25,
            width: (1.0, 9.0),
            color: (214, 39, 40),
            alpha: 0.6,
            arrows: true,
        }
    }
}

impl GeomFlow {
    /// Project endpoints with `projection` (use the map's projection).
    pub fn project(mut self, projection: SfProjection) -> Self {
        self.projection = projection;
        self
    }
}

impl Geom for GeomFlow {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let col = |n: &str| {
            data.column(n)
                .ok_or_else(|| RenderError::MissingAesthetic(n.into()))
        };
        let (x0c, y0c, x1c, y1c) = (col("x")?, col("y")?, col("xend")?, col("yend")?);
        let (weight, color_col, label) = (
            data.column("weight"),
            data.column("color"),
            data.column("label"),
        );
        let area = backend.plot_area();
        let px = |x: f64, y: f64| {
            let sx = scales.get(&Aesthetic::X).map(|s| s.map(&Value::Float(x)));
            let sy = scales.get(&Aesthetic::Y).map(|s| s.map(&Value::Float(y)));
            coord.transform((sx.unwrap_or(0.5), sy.unwrap_or(0.5)), &area)
        };
        let max_w = weight
            .map(|w| w.iter().filter_map(|v| v.as_f64()).fold(0.0, f64::max))
            .unwrap_or(1.0);
        // Thin flows on top.
        let mut order: Vec<usize> = (0..data.nrows()).collect();
        let wt = |i: usize| weight.and_then(|w| w[i].as_f64()).unwrap_or(1.0);
        order.sort_by(|&a, &b| wt(b).total_cmp(&wt(a)));
        for i in order {
            let (Some(x0), Some(y0), Some(x1), Some(y1)) = (
                x0c[i].as_f64(),
                y0c[i].as_f64(),
                x1c[i].as_f64(),
                y1c[i].as_f64(),
            ) else {
                continue;
            };
            let (a, b) = (px(x0, y0), px(x1, y1));
            if ![a.0, a.1, b.0, b.1].iter().all(|v| v.is_finite()) {
                continue;
            }
            // Quadratic Bézier with the control point off the midpoint.
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let ctrl = (
                (a.0 + b.0) / 2.0 - dy * self.curvature,
                (a.1 + b.1) / 2.0 + dx * self.curvature,
            );
            let path: Vec<(f64, f64)> = (0..=32)
                .map(|k| {
                    let t = k as f64 / 32.0;
                    let u = 1.0 - t;
                    (
                        u * u * a.0 + 2.0 * u * t * ctrl.0 + t * t * b.0,
                        u * u * a.1 + 2.0 * u * t * ctrl.1 + t * t * b.1,
                    )
                })
                .collect();
            let w = self.width.0
                + (self.width.1 - self.width.0)
                    * if max_w > 0.0 {
                        (wt(i) / max_w).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
            let color = color_col
                .and_then(|c| scales.map_color(&Aesthetic::Color, &c[i]))
                .unwrap_or(self.color);
            let name = label.map(|l| super::tip_value(&l[i])).unwrap_or_default();
            let tip = if name.is_empty() {
                format!("flow: {}", super::tip_value(&Value::Float(wt(i))))
            } else {
                format!("{name}: {}", super::tip_value(&Value::Float(wt(i))))
            };
            super::set_mark(
                backend,
                Some(tip),
                None,
                super::series_key(data, i),
                weight.and_then(|c| super::raw_value(&c[i])),
            );
            super::set_row_extras(backend, data, i);
            backend.draw_polygon(
                &super::arcdiagram::stroke_band(&path, w),
                &RectStyle {
                    fill: Some(color),
                    stroke: None,
                    stroke_width: 0.0,
                    alpha: self.alpha,
                    clip: true,
                    corner_radius: 0.0,
                },
            )?;
            if self.arrows {
                let head = arrow().kind(ArrowType::Closed).length(6.0 + w);
                let hs = heads(&path, &head, w);
                draw_heads(backend, &hs, &head, color, self.alpha.max(0.8), 1.0)?;
                backend.set_mark_part(None);
            }
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y, Aesthetic::Xend, Aesthetic::Yend]
    }
    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatIdentity)
    }
    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }
    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }
    fn name(&self) -> &str {
        "flow"
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }

    /// Project both endpoints (lon/lat → map units) before the scales train,
    /// and expose the destination as extent columns so it trains them too.
    fn setup_data(&self, data: &mut DataFrame, _warnings: &mut Vec<String>) {
        let n = data.nrows();
        let get = |c: &str| data.column(c).map(|v| v.to_vec());
        let (Some(x0), Some(y0), Some(x1), Some(y1)) =
            (get("x"), get("y"), get("xend"), get("yend"))
        else {
            return;
        };
        let mut out: [Vec<Value>; 4] = Default::default();
        for i in 0..n {
            let proj = |lon: &Value, lat: &Value| -> Option<[f64; 2]> {
                Some(self.projection.project([lon.as_f64()?, lat.as_f64()?]))
            };
            let (p, q) = (proj(&x0[i], &y0[i]), proj(&x1[i], &y1[i]));
            for (k, v) in [
                p.map(|p| p[0]),
                p.map(|p| p[1]),
                q.map(|q| q[0]),
                q.map(|q| q[1]),
            ]
            .into_iter()
            .enumerate()
            {
                out[k].push(v.map(Value::Float).unwrap_or(Value::Na));
            }
        }
        let [px0, py0, px1, py1] = out;
        for (name, vals) in [
            ("x", px0),
            ("y", py0),
            ("xend", px1.clone()),
            ("yend", py1.clone()),
        ] {
            if let Some(c) = data.column_mut(name) {
                *c = vals;
            }
        }
        if !data.has_column("xmax") {
            data.add_column("xmax".into(), px1);
        }
        if !data.has_column("ymax") {
            data.add_column("ymax".into(), py1);
        }
    }
}
