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
}

impl Default for RectStyle {
    fn default() -> Self {
        RectStyle {
            fill: Some((128, 128, 128)),
            stroke: None,
            stroke_width: 1.0,
            alpha: 1.0,
            clip: true,
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
