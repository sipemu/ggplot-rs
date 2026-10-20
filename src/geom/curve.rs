use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::geom::arrow::{draw_heads, heads, Arrow};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineEnd, LineStyle, Linetype};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Curve geometry — curved segment via a quadratic Bézier.
pub struct GeomCurve {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
    /// Curvature of the control point, in pixel space: the control point
    /// sits at `mid + (-dy, dx) * curvature` (positive is right-handed).
    pub curvature: f64,
    /// Ignored: SVG draws a true quadratic Bézier (one control point), so
    /// there is no subdivision count to tune. Kept for API compatibility.
    pub ncp: usize,
    /// An optional arrowhead drawn at the curve's end(s).
    pub arrow: Option<Arrow>,
    /// Line cap style (R grid's `lineend`).
    pub lineend: LineEnd,
}

impl Default for GeomCurve {
    fn default() -> Self {
        GeomCurve {
            color: (0, 0, 0),
            width: 1.0,
            alpha: 1.0,
            curvature: 0.5,
            ncp: 5,
            arrow: None,
            lineend: LineEnd::Butt,
        }
    }
}

impl GeomCurve {
    /// Fixed stroke colour, used when no `color` aesthetic is mapped.
    pub fn color(mut self, color: (u8, u8, u8)) -> Self {
        self.color = color;
        self
    }

    /// Sets `width` (the stroke width in pixels).
    pub fn linewidth(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    /// Fixed opacity, used when no `alpha` aesthetic is mapped.
    pub fn alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha;
        self
    }

    /// See the [`curvature`](Self::curvature) field doc.
    pub fn curvature(mut self, curvature: f64) -> Self {
        self.curvature = curvature;
        self
    }

    /// See the [`ncp`](Self::ncp) field doc: ignored by SVG rendering.
    pub fn ncp(mut self, ncp: usize) -> Self {
        self.ncp = ncp;
        self
    }

    /// Draw an arrowhead at the end(s) — see [`arrow()`](crate::geom::arrow::arrow).
    pub fn arrow(mut self, arrow: Arrow) -> Self {
        self.arrow = Some(arrow);
        self
    }

    /// Line cap style (R grid's `lineend`).
    pub fn lineend(mut self, lineend: LineEnd) -> Self {
        self.lineend = lineend;
        self
    }

    /// The single quadratic-Bézier control point for `p0 -> p2`, in pixel
    /// space: `mid + (-dy, dx) * curvature`.
    fn control_point(p0: (f64, f64), p2: (f64, f64), curvature: f64) -> (f64, f64) {
        let mx = (p0.0 + p2.0) / 2.0;
        let my = (p0.1 + p2.1) / 2.0;
        let dx = p2.0 - p0.0;
        let dy = p2.1 - p0.1;
        (mx - dy * curvature, my + dx * curvature)
    }

    /// Flatten the quadratic Bézier to `n` points — used only to feed the
    /// arrowhead geometry ([`crate::geom::arrow::heads`]) an accurate
    /// polyline; the actual stroke is drawn with
    /// [`DrawBackend::draw_quadratic`], a true curve on the SVG backend.
    fn flatten(p0: (f64, f64), ctrl: (f64, f64), p2: (f64, f64), n: usize) -> Vec<(f64, f64)> {
        (0..n)
            .map(|i| {
                let t = i as f64 / (n - 1) as f64;
                let u = 1.0 - t;
                let x = u * u * p0.0 + 2.0 * u * t * ctrl.0 + t * t * p2.0;
                let y = u * u * p0.1 + 2.0 * u * t * ctrl.1 + t * t * p2.1;
                (x, y)
            })
            .collect()
    }
}

impl Geom for GeomCurve {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let x_col = data
            .column("x")
            .ok_or(RenderError::MissingAesthetic("x".into()))?;
        let y_col = data
            .column("y")
            .ok_or(RenderError::MissingAesthetic("y".into()))?;
        let xend_col = data
            .column("xend")
            .ok_or(RenderError::MissingAesthetic("xend".into()))?;
        let yend_col = data
            .column("yend")
            .ok_or(RenderError::MissingAesthetic("yend".into()))?;
        let color_col = data.column("color");
        let linetype_col = data.column("linetype");
        let alpha_col = data.column("alpha");
        let linewidth_col = data.column("linewidth");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let nx1 = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny1 = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let nx2 = x_scale.map(|s| s.map(&xend_col[i])).unwrap_or(0.0);
            let ny2 = y_scale.map(|s| s.map(&yend_col[i])).unwrap_or(0.0);

            let p0 = coord.transform((nx1, ny1), &plot_area);
            let p2 = coord.transform((nx2, ny2), &plot_area);
            let ctrl = Self::control_point(p0, p2, self.curvature);

            let line_color = color_col
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[i]))
                .unwrap_or(self.color);
            let lt = linetype_col
                .and_then(|lc| scales.map_linetype(&lc[i]))
                .unwrap_or(Linetype::Solid);
            let alpha = alpha_col
                .and_then(|ac| scales.map_alpha(&ac[i]).or_else(|| ac[i].as_f64()))
                .filter(|a| a.is_finite())
                .unwrap_or(self.alpha);
            let width = linewidth_col
                .and_then(|lc| scales.map_linewidth(&lc[i]))
                .unwrap_or(self.width);
            super::set_row_extras(backend, data, i);

            backend.draw_quadratic(
                p0,
                ctrl,
                p2,
                &LineStyle {
                    color: line_color,
                    alpha,
                    width,
                    linetype: lt,
                    lineend: self.lineend,
                },
            )?;

            if let Some(arrow) = &self.arrow {
                let path = Self::flatten(p0, ctrl, p2, 64);
                let hs = heads(&path, arrow, width);
                draw_heads(backend, &hs, arrow, line_color, alpha, width)?;
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
        "curve"
    }

    fn consumes_tooltips(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}

#[cfg(test)]
mod tests {
    use crate::data::Value;
    use crate::geom::arrow::arrow;
    use crate::prelude::*;

    fn plot() -> GGPlot {
        GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(0.2)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("xend".into(), vec![Value::Float(1.0), Value::Float(0.8)]),
            ("yend".into(), vec![Value::Float(0.5), Value::Float(0.3)]),
        ])
        .aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
    }

    #[test]
    fn curve_emits_a_native_quadratic_path() {
        let svg = plot()
            .geom_curve_with(GeomCurve::default())
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert!(svg.contains("<path"));
        assert!(svg.contains(" Q"));
        assert!(!svg.contains("NaN"));
    }

    #[test]
    fn curve_arrow_closed_emits_tagged_polygons() {
        let svg = plot()
            .geom_curve_with(GeomCurve::default().arrow(arrow().closed()))
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert_eq!(svg.matches("data-part=\"arrow\"").count(), 2);
    }

    #[test]
    fn lineend_round_gives_stroke_linecap_round_on_the_path() {
        let svg = plot()
            .geom_curve_with(GeomCurve::default().lineend(crate::render::backend::LineEnd::Round))
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert!(svg.contains("stroke-linecap=\"round\""));
    }

    #[test]
    fn control_point_matches_the_documented_formula() {
        let p0 = (0.0, 0.0);
        let p2 = (10.0, 0.0);
        let ctrl = GeomCurve::control_point(p0, p2, 0.2);
        // mid = (5, 0); dx = 10, dy = 0 -> ctrl = (5 - 0*0.2, 0 + 10*0.2) = (5, 2)
        assert!((ctrl.0 - 5.0).abs() < 1e-9);
        assert!((ctrl.1 - 2.0).abs() < 1e-9);
    }

    /// `.color()`/`.alpha()`/`.linewidth()`/`.curvature()`/`.ncp()` builders
    /// actually change the rendered output, not just the struct's fields.
    #[test]
    fn builders_change_the_rendered_curve() {
        let flat = plot()
            .geom_curve_with(GeomCurve::default().curvature(0.0))
            .render_svg_native_with_size(200, 200)
            .unwrap();
        let bent = plot()
            .geom_curve_with(GeomCurve::default().curvature(0.8))
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert_ne!(flat, bent, "curvature should change the control point");

        let styled = plot()
            .geom_curve_with(
                GeomCurve::default()
                    .color((10, 20, 30))
                    .alpha(0.4)
                    .linewidth(5.0)
                    // `ncp` is ignored by SVG rendering (documented), but
                    // the builder itself must still work.
                    .ncp(12),
            )
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert!(styled.contains("stroke=\"#0A141E\""));
        assert!(styled.contains("stroke-opacity=\"0.400\""));
        assert!(styled.contains("stroke-width=\"5.00\""));
    }

    // ─── B1-7: interactive aesthetics ──────────────────────────────────

    fn plot_with_extras() -> GGPlot {
        GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(0.2)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("xend".into(), vec![Value::Float(1.0), Value::Float(0.8)]),
            ("yend".into(), vec![Value::Float(0.5), Value::Float(0.3)]),
            (
                "orig".into(),
                vec![Value::Str("Berlin".into()), Value::Str("Paris & Co".into())],
            ),
            (
                "tip".into(),
                vec![
                    Value::Str("A tooltip".into()),
                    Value::Str("<script>".into()),
                ],
            ),
        ])
        .aes(
            Aes::new()
                .x("x")
                .y("y")
                .xend("xend")
                .yend("yend")
                .data_attr("orig", "orig")
                .tooltip("tip"),
        )
    }

    #[test]
    fn data_attr_and_tooltip_both_emit_and_arrow_heads_carry_the_attr_but_no_title() {
        let svg = plot_with_extras()
            .geom_curve_with(GeomCurve::default().arrow(crate::geom::arrow::arrow().closed()))
            .render_svg_native_with_size(200, 200)
            .unwrap();

        assert!(svg.contains("data-orig=\"Berlin\""));
        // `&` is escaped.
        assert!(svg.contains("data-orig=\"Paris &amp; Co\""));
        assert!(svg.contains("<title>A tooltip</title>"));
        // `<script>` is escaped, never a literal second element.
        assert!(svg.contains("&lt;script&gt;"));
        assert!(!svg.contains("<script>"));

        // Arrow heads are decorative marks: they carry the row's data-orig
        // (set before the arrow is drawn) but no `<title>`. Each is a
        // self-closing `<polygon .../>` element, so bound the check to it.
        for part in svg.split("<polygon").skip(1) {
            let element = part.split("/>").next().unwrap_or("");
            if element.contains("data-part=\"arrow\"") {
                assert!(element.contains("data-orig="));
                assert!(!element.contains("<title>"));
            }
        }
    }

    #[test]
    fn data_attr_x_gives_a_single_data_x_not_two() {
        // GeomPoint sets the built-in `data-x` via `set_mark_axis`;
        // `data_attr("x", "xlabel")` maps to the same attribute name and
        // should replace it, not duplicate it.
        let svg = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            (
                "xlabel".into(),
                vec![Value::Str("first".into()), Value::Str("second".into())],
            ),
        ])
        .aes(Aes::new().x("x").y("y").data_attr("x", "xlabel"))
        .geom_point()
        .render_svg_native_with_size(200, 200)
        .unwrap();

        assert!(svg.contains("data-x=\"first\""));
        assert!(svg.contains("data-x=\"second\""));
        for tag_open in svg.split('<').skip(1) {
            let attrs_part = tag_open.split('>').next().unwrap_or("");
            let count = attrs_part.matches("data-x=\"").count();
            assert!(count <= 1, "data-x duplicated in: <{tag_open}");
        }
    }

    #[test]
    fn layers_without_interactive_aes_are_byte_identical() {
        let plain = |p: GGPlot| {
            p.geom_curve_with(GeomCurve::default())
                .render_svg_native_with_size(200, 200)
                .unwrap()
        };
        let a = plain(plot());
        let b = plain(plot());
        assert_eq!(a, b);
        assert!(!a.contains("data-orig"));
    }

    /// Review finding #3: `data_attr("part", ...)` must never produce a
    /// second `data-part` alongside the arrowhead's own — a duplicate
    /// attribute is an XML parse error, and the duplicate would shadow
    /// `data-part="arrow"`, breaking the `[data-part="arrow"]` selector.
    #[test]
    fn data_attr_named_part_is_rejected_so_arrowheads_never_get_a_duplicate_data_part() {
        let plot = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(0.2)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("xend".into(), vec![Value::Float(1.0), Value::Float(0.8)]),
            ("yend".into(), vec![Value::Float(0.5), Value::Float(0.3)]),
            (
                "p".into(),
                vec![Value::Str("mine".into()), Value::Str("mine".into())],
            ),
        ])
        .aes(
            Aes::new()
                .x("x")
                .y("y")
                .xend("xend")
                .yend("yend")
                .data_attr("part", "p"),
        );
        // The invalid/reserved name is rejected with a build warning, not
        // silently accepted.
        assert!(plot
            .warnings
            .iter()
            .any(|w| w.contains("part") && w.contains("reserved")));

        let svg = plot
            .geom_curve_with(GeomCurve::default().arrow(crate::geom::arrow::arrow().closed()))
            .render_svg_native_with_size(200, 200)
            .unwrap();

        // Exactly one `data-part` per arrowhead element, always "arrow".
        for part in svg.split("<polygon").skip(1) {
            let element = part.split("/>").next().unwrap_or("");
            if element.contains("data-part") {
                assert_eq!(element.matches("data-part=").count(), 1);
                assert!(element.contains("data-part=\"arrow\""));
            }
        }
        assert!(!svg.contains("data-part=\"mine\""));
    }
}
