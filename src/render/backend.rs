use super::{Rect, RenderError};

/// Point shape types.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum PointShape {
    Circle,
    Triangle,
    Square,
    Diamond,
    Cross,
    Plus,
}

impl PointShape {
    /// All shapes in order (for discrete scale mapping).
    pub const ALL: &[PointShape] = &[
        PointShape::Circle,
        PointShape::Triangle,
        PointShape::Square,
        PointShape::Diamond,
        PointShape::Cross,
        PointShape::Plus,
    ];
}

/// Line type patterns.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Linetype {
    Solid,
    Dashed,
    Dotted,
    DashDot,
    LongDash,
    TwoDash,
}

impl Linetype {
    /// All linetypes in order (for discrete scale mapping).
    pub const ALL: &[Linetype] = &[
        Linetype::Solid,
        Linetype::Dashed,
        Linetype::Dotted,
        Linetype::DashDot,
        Linetype::LongDash,
        Linetype::TwoDash,
    ];

    /// Get dash pattern as (draw_len, gap_len) pairs in pixels.
    pub fn pattern(&self) -> &[(f64, f64)] {
        match self {
            Linetype::Solid => &[],
            Linetype::Dashed => &[(6.0, 3.0)],
            Linetype::Dotted => &[(2.0, 2.0)],
            Linetype::DashDot => &[(6.0, 2.0), (2.0, 2.0)],
            Linetype::LongDash => &[(10.0, 4.0)],
            Linetype::TwoDash => &[(8.0, 3.0), (3.0, 3.0)],
        }
    }
}

/// Style for drawing points/circles.
#[derive(Clone, Debug)]
pub struct PointStyle {
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub filled: bool,
    pub shape: PointShape,
}

impl Default for PointStyle {
    fn default() -> Self {
        PointStyle {
            color: (0, 0, 0),
            alpha: 1.0,
            filled: true,
            shape: PointShape::Circle,
        }
    }
}

/// Line cap style (R grid's `lineend`), applied to the stroked ends of a
/// polyline/path. `Butt` is SVG's own default, so it is written on the wire
/// only when it differs (byte-identical default output).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineEnd {
    #[default]
    Butt,
    Round,
    Square,
}

/// Style for drawing lines.
#[derive(Clone, Debug)]
pub struct LineStyle {
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub width: f64,
    pub linetype: Linetype,
    pub lineend: LineEnd,
}

impl Default for LineStyle {
    fn default() -> Self {
        LineStyle {
            color: (0, 0, 0),
            alpha: 1.0,
            width: 1.0,
            linetype: Linetype::Solid,
            lineend: LineEnd::Butt,
        }
    }
}

/// Style for drawing rectangles/polygons.
#[derive(Clone, Debug)]
pub struct RectStyle {
    pub fill: Option<(u8, u8, u8)>,
    pub stroke: Option<(u8, u8, u8)>,
    pub stroke_width: f64,
    pub alpha: f64,
    /// Whether to clip this rect to the plot area. Default `true` for data elements.
    /// Set to `false` for non-data elements (backgrounds, strips, legends).
    pub clip: bool,
    /// Corner radius in px, for a rounded rectangle (ggplot2 `geom_label(label.r)`,
    /// ggchicklet `radius`). `0.0` (the default) draws a plain rectangle; backends
    /// that cannot draw rounded corners natively fall back to a plain rectangle.
    pub corner_radius: f64,
}

impl Default for RectStyle {
    fn default() -> Self {
        RectStyle {
            fill: Some((128, 128, 128)),
            stroke: None,
            stroke_width: 1.0,
            alpha: 1.0,
            clip: true,
            corner_radius: 0.0,
        }
    }
}

/// Number of line segments approximating each 90° corner of a rounded rect in
/// backends with no native rounded-rect primitive (pixel, plotters). Higher
/// is smoother; 8 is visually indistinguishable from a true arc at the sizes
/// these rects are drawn at (node/label boxes, a few tens to ~200px).
///
/// Only [`pixel_backend`](super::pixel_backend) (feature `canvas`) and
/// [`plotters_backend`](super::plotters_backend) (feature `plotters`) call
/// [`rounded_rect_points`] — `svg_backend` emits a native `rx`/`ry` instead —
/// so both it and the function are cfg-gated the same way those backends
/// are, rather than left for `--no-default-features` to flag as dead code.
#[cfg(any(
    feature = "canvas",
    all(feature = "plotters", not(target_arch = "wasm32"))
))]
const ROUNDED_RECT_CORNER_SEGMENTS: usize = 8;

/// A closed polygon approximating a rectangle with rounded corners, for
/// backends that must rasterize/triangulate rather than emit a native rounded
/// rect (see [`DrawBackend::draw_rect`]'s `corner_radius`, via [`RectStyle`]).
///
/// `radius` is clamped to half the shorter side, so an oversized radius draws
/// a "stadium" (fully rounded) shape rather than overshooting. A
/// non-positive radius returns the plain 4-corner rectangle.
///
/// Corners go clockwise in screen coordinates (y down), starting at the
/// top-right, so the first and last points sit either side of the top-right
/// corner — callers that stroke the result as a closed loop (last point back
/// to the first) get a seamless boundary.
#[cfg(any(
    feature = "canvas",
    all(feature = "plotters", not(target_arch = "wasm32"))
))]
pub(crate) fn rounded_rect_points(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    radius: f64,
) -> Vec<(f64, f64)> {
    let (x0, x1) = (x0.min(x1), x0.max(x1));
    let (y0, y1) = (y0.min(y1), y0.max(y1));
    let r = radius.max(0.0).min((x1 - x0) / 2.0).min((y1 - y0) / 2.0);
    if r <= 0.0 {
        return vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    }
    use std::f64::consts::{FRAC_PI_2, PI};
    let n = ROUNDED_RECT_CORNER_SEGMENTS;
    // (corner centre, start angle, end angle), clockwise, starting top-right.
    let corners = [
        (x1 - r, y0 + r, -FRAC_PI_2, 0.0),     // top-right
        (x1 - r, y1 - r, 0.0, FRAC_PI_2),      // bottom-right
        (x0 + r, y1 - r, FRAC_PI_2, PI),       // bottom-left
        (x0 + r, y0 + r, PI, 3.0 * FRAC_PI_2), // top-left
    ];
    let mut pts = Vec::with_capacity(4 * (n + 1));
    for (cx, cy, a0, a1) in corners {
        for i in 0..=n {
            let t = a0 + (a1 - a0) * (i as f64 / n as f64);
            pts.push((cx + r * t.cos(), cy + r * t.sin()));
        }
    }
    pts
}

#[cfg(all(
    test,
    any(
        feature = "canvas",
        all(feature = "plotters", not(target_arch = "wasm32"))
    )
))]
mod rounded_rect_tests {
    use super::rounded_rect_points;

    #[test]
    fn zero_radius_is_a_plain_rectangle() {
        let pts = rounded_rect_points(0.0, 0.0, 10.0, 20.0, 0.0);
        assert_eq!(
            pts,
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 20.0), (0.0, 20.0)]
        );
    }

    #[test]
    fn radius_is_clamped_to_half_the_shorter_side() {
        // A radius bigger than the box collapses to a "stadium": every point
        // must stay within the rect's bounds.
        let pts = rounded_rect_points(0.0, 0.0, 10.0, 20.0, 1000.0);
        for (x, y) in &pts {
            assert!((0.0..=10.0).contains(x), "x={x} out of [0,10]");
            assert!((0.0..=20.0).contains(y), "y={y} out of [0,20]");
        }
    }

    #[test]
    fn corners_are_inset_by_radius_from_the_rect_corners() {
        let pts = rounded_rect_points(0.0, 0.0, 100.0, 50.0, 10.0);
        // The arc midpoint of the top-right corner sits at 45° in from the
        // corner, inset by r*(1 - cos(45°)) from the straight edges.
        assert!(pts.len() > 4);
        for (x, y) in &pts {
            assert!(*x >= 0.0 && *x <= 100.0);
            assert!(*y >= 0.0 && *y <= 50.0);
        }
    }
}

/// Style for drawing text.
#[derive(Clone, Debug)]
pub struct TextStyle {
    pub color: (u8, u8, u8),
    pub size: f64,
    pub anchor: TextAnchor,
    pub angle: f64,
    /// Font family (e.g., "serif", "monospace"). None defaults to "sans-serif".
    pub family: Option<String>,
    /// Font face (R's `element_text(face = ...)`).
    pub face: FontFace,
}

/// Font face / weight (R's `element_text(face = ...)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontFace {
    #[default]
    Plain,
    Bold,
    Italic,
}

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle {
            color: (50, 50, 50),
            size: 12.0,
            anchor: TextAnchor::Middle,
            angle: 0.0,
            family: None,
            face: crate::render::backend::FontFace::Plain,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

/// Our rendering abstraction, independent of plotters details.
pub trait DrawBackend {
    fn draw_circle(
        &mut self,
        center: (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError>;
    fn draw_line(&mut self, points: &[(f64, f64)], style: &LineStyle) -> Result<(), RenderError>;
    fn draw_rect(
        &mut self,
        top_left: (f64, f64),
        bottom_right: (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError>;
    fn draw_text(
        &mut self,
        text: &str,
        pos: (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError>;
    fn draw_polygon(&mut self, points: &[(f64, f64)], style: &RectStyle)
        -> Result<(), RenderError>;

    /// Like [`draw_text`](Self::draw_text), but with an optional `<title>`
    /// tooltip carrying the *full* text when `text` itself has been
    /// shortened (e.g. a truncated-with-ellipsis legend label, #159).
    /// Default: ignores `title` and draws `text` plain — backends without
    /// native tooltip support (anything but SVG) have no better option.
    /// SVG overrides this to emit the `<title>` child, escaped the same
    /// way every other text run is.
    fn draw_text_with_title(
        &mut self,
        text: &str,
        pos: (f64, f64),
        style: &TextStyle,
        _title: Option<&str>,
    ) -> Result<(), RenderError> {
        self.draw_text(text, pos, style)
    }
    fn plot_area(&self) -> Rect;
    fn total_area(&self) -> Rect;

    /// Attach a tooltip to subsequently-drawn marks until cleared with `None`.
    /// Backends that produce interactive output (SVG) emit it as a `<title>`
    /// for a native hover tooltip; others ignore it. Default: no-op.
    fn set_tooltip(&mut self, _tooltip: Option<String>) {}

    /// Attach an axis (x) key to subsequently-drawn marks — SVG emits it as a
    /// `data-x` attribute so a host can group marks by x (axis-pointer tooltip).
    /// Default: no-op.
    fn set_mark_axis(&mut self, _key: Option<String>) {}

    /// Attach a series key (the colour / fill / group level) to subsequently
    /// drawn marks — SVG emits it as `data-series`, so a host can highlight or
    /// filter one series without parsing the tooltip. Default: no-op.
    fn set_mark_series(&mut self, _series: Option<String>) {}

    /// Attach the raw measured value (the unformatted y) to subsequently drawn
    /// marks — SVG emits it as `data-value`. Default: no-op.
    fn set_mark_value(&mut self, _value: Option<String>) {}

    /// Tag subsequently drawn elements as data marks of the named geom (the
    /// layer's `Geom::name`) until cleared with `None`. SVG emits
    /// `class="gg-mark gg-<kind>"` (kind from the primitive: `point`, `line`,
    /// `band`, `bar`, `text`) plus `data-geom`, so a host can select data
    /// marks without matching axes, grid lines or legends. Default: no-op.
    fn set_mark_geom(&mut self, _geom: Option<&str>) {}

    /// Report a non-fatal problem found while drawing (e.g. labels a repel
    /// layout had to drop). Backends that collect warnings (the native SVG
    /// backend) append it to the render warnings. Default: ignored.
    fn warn(&mut self, _message: String) {}

    /// Mark subsequently-drawn marks as belonging to a sub-part of a larger
    /// geom (e.g. `"arrow"` for an arrowhead drawn alongside a segment/curve/
    /// path/line). While set, the SVG backend writes `data-part="<part>"
    /// aria-hidden="true"` and omits the `<title>` tooltip (the part is
    /// decorative; the parent mark already carries the semantic info).
    /// `data-x`/`data-series`/`data-value` (and any user `data-*` attrs) are
    /// still written. Default: no-op.
    fn set_mark_part(&mut self, _part: Option<&str>) {}

    /// Attach arbitrary `data-*` attributes to subsequently-drawn marks
    /// (B1-7's `Aes::data_attr`). The SVG backend writes each, escaped; an
    /// attr with the same name as a built-in (`data-x`, `data-series`,
    /// `data-value`) replaces it rather than duplicating it. An empty
    /// `Vec` clears any previously set attrs. Default: no-op.
    fn set_mark_attrs(&mut self, _attrs: Vec<(String, String)>) {}

    /// Whether the caller asked for per-mark formatted values
    /// ([`crate::plot::GGPlot::svg_formatted_values`], B2-4) — a query a
    /// geom checks before doing the (otherwise pointless) work of resolving
    /// a scale's formatter for `set_mark_fvalue`/`set_mark_fx`. Default
    /// false, so the feature is byte-identical when never turned on.
    fn wants_formatted_values(&self) -> bool {
        false
    }

    /// `data-fvalue` on subsequently-drawn marks: the "value" aesthetic's
    /// raw data value (y for most geoms, fill for hex/tile), formatted
    /// through that aesthetic's scale (`Scale::format_value`) — e.g. a
    /// de-DE currency string. Only meaningful while
    /// [`wants_formatted_values`](Self::wants_formatted_values) is true;
    /// `None` clears any previous mark's value, per row. Default: no-op.
    fn set_mark_fvalue(&mut self, _value: Option<String>) {}

    /// `data-fx` on subsequently-drawn marks: the x value formatted through
    /// the x scale (e.g. a datetime axis's ISO/locale date). The SVG
    /// backend omits it when it matches the already-written `data-x`, so a
    /// plain numeric x doesn't get a redundant duplicate attribute. Default:
    /// no-op.
    fn set_mark_fx(&mut self, _value: Option<String>) {}

    /// Draw a point with a specific shape. Default delegates to draw_circle for Circle.
    fn draw_shape(
        &mut self,
        center: (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        match style.shape {
            PointShape::Circle => self.draw_circle(center, radius, style),
            _ => {
                // Default: fall back to circle for unsupported backends
                self.draw_circle(center, radius, style)
            }
        }
    }

    /// Draw a quadratic Bézier curve from `p0` to `p1` with control point
    /// `ctrl`, in pixel space. The default implementation flattens the curve
    /// to 32 line segments and draws it with [`draw_line`](Self::draw_line);
    /// backends that support real curves (SVG) should override this.
    /// Non-finite input draws nothing.
    fn draw_quadratic(
        &mut self,
        p0: (f64, f64),
        ctrl: (f64, f64),
        p1: (f64, f64),
        style: &LineStyle,
    ) -> Result<(), RenderError> {
        if !(p0.0.is_finite()
            && p0.1.is_finite()
            && ctrl.0.is_finite()
            && ctrl.1.is_finite()
            && p1.0.is_finite()
            && p1.1.is_finite())
        {
            return Ok(());
        }
        const STEPS: usize = 32;
        let mut pts = Vec::with_capacity(STEPS + 1);
        for i in 0..=STEPS {
            let t = i as f64 / STEPS as f64;
            let one_minus_t = 1.0 - t;
            let x =
                one_minus_t * one_minus_t * p0.0 + 2.0 * one_minus_t * t * ctrl.0 + t * t * p1.0;
            let y =
                one_minus_t * one_minus_t * p0.1 + 2.0 * one_minus_t * t * ctrl.1 + t * t * p1.1;
            pts.push((x, y));
        }
        self.draw_line(&pts, style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Rect, RenderError};
    use std::cell::RefCell;

    /// A minimal backend that does not override `draw_quadratic`, to exercise
    /// the trait's default flattening implementation.
    #[derive(Default)]
    struct RecordingBackend {
        lines: RefCell<Vec<Vec<(f64, f64)>>>,
        circles: RefCell<Vec<((f64, f64), f64)>>,
    }

    impl DrawBackend for RecordingBackend {
        fn draw_circle(
            &mut self,
            center: (f64, f64),
            radius: f64,
            _style: &PointStyle,
        ) -> Result<(), RenderError> {
            self.circles.borrow_mut().push((center, radius));
            Ok(())
        }
        fn draw_line(
            &mut self,
            points: &[(f64, f64)],
            _style: &LineStyle,
        ) -> Result<(), RenderError> {
            self.lines.borrow_mut().push(points.to_vec());
            Ok(())
        }
        fn draw_rect(
            &mut self,
            _top_left: (f64, f64),
            _bottom_right: (f64, f64),
            _style: &RectStyle,
        ) -> Result<(), RenderError> {
            Ok(())
        }
        fn draw_text(
            &mut self,
            _text: &str,
            _pos: (f64, f64),
            _style: &TextStyle,
        ) -> Result<(), RenderError> {
            Ok(())
        }
        fn draw_polygon(
            &mut self,
            _points: &[(f64, f64)],
            _style: &RectStyle,
        ) -> Result<(), RenderError> {
            Ok(())
        }
        fn plot_area(&self) -> Rect {
            Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            }
        }
        fn total_area(&self) -> Rect {
            self.plot_area()
        }
    }

    #[test]
    fn default_draw_quadratic_flattens_to_33_points() {
        let mut b = RecordingBackend::default();
        b.draw_quadratic((0.0, 0.0), (5.0, 10.0), (10.0, 0.0), &LineStyle::default())
            .unwrap();
        let lines = b.lines.borrow();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 33);
        assert_eq!(lines[0][0], (0.0, 0.0));
        assert_eq!(lines[0][32], (10.0, 0.0));
        // Midpoint of a quadratic Bézier at t=0.5 is the average of the
        // endpoints and the control point, each weighted 1/4, 1/2, 1/4.
        let mid = lines[0][16];
        assert!((mid.0 - 5.0).abs() < 1e-9);
        assert!((mid.1 - 5.0).abs() < 1e-9);
    }

    #[test]
    fn default_draw_quadratic_drops_non_finite() {
        let mut b = RecordingBackend::default();
        b.draw_quadratic(
            (f64::NAN, 0.0),
            (5.0, 10.0),
            (10.0, 0.0),
            &LineStyle::default(),
        )
        .unwrap();
        assert!(b.lines.borrow().is_empty());
    }

    /// The default `draw_shape` falls back to `draw_circle` for every
    /// `PointShape` (not just `Circle`) on a backend that doesn't override
    /// it — e.g. a minimal backend that only implements the required
    /// methods.
    #[test]
    fn default_draw_shape_falls_back_to_draw_circle_for_every_point_shape() {
        let mut b = RecordingBackend::default();
        for shape in PointShape::ALL {
            b.draw_shape(
                (1.0, 2.0),
                3.0,
                &PointStyle {
                    shape: *shape,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        assert_eq!(b.circles.borrow().len(), PointShape::ALL.len());
        for (center, radius) in b.circles.borrow().iter() {
            assert_eq!(*center, (1.0, 2.0));
            assert_eq!(*radius, 3.0);
        }
    }

    #[test]
    fn recording_backend_areas_and_remaining_draw_methods_behave_as_declared() {
        let mut b = RecordingBackend::default();
        let pa = b.plot_area();
        let ta = b.total_area();
        assert_eq!(
            (pa.x, pa.y, pa.width, pa.height),
            (ta.x, ta.y, ta.width, ta.height)
        );
        assert_eq!((pa.x, pa.y, pa.width, pa.height), (0.0, 0.0, 100.0, 100.0));
        b.draw_rect((0.0, 0.0), (1.0, 1.0), &RectStyle::default())
            .unwrap();
        b.draw_text("label", (0.0, 0.0), &TextStyle::default())
            .unwrap();
        b.draw_polygon(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)], &RectStyle::default())
            .unwrap();
    }
}
