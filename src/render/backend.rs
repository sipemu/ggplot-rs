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
///
/// The named variants are ggplot2's fixed patterns; [`Linetype::Custom`]
/// takes a caller-supplied dash/gap sequence (#220), e.g.
/// `Linetype::Custom(vec![(0.0, 10.0)])` together with
/// [`LineEnd::Round`] draws a row of round dots every 10 px (zero-length
/// dashes are drawn as round caps), as flow-diagram edges use.
///
/// `Linetype` is no longer `Copy` (a `Custom` pattern owns its `Vec`) —
/// clone it where a copy was taken before.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Linetype {
    Solid,
    Dashed,
    Dotted,
    DashDot,
    LongDash,
    TwoDash,
    /// Arbitrary `(dash, gap)` pairs in px, rendered through the same
    /// `stroke-dasharray` / dashed-stroking path as the named variants.
    ///
    /// Degenerate input never panics: pairs with a non-finite value are
    /// dropped, negative lengths are clamped to `0`, and a pattern with no
    /// usable pairs (empty, all-NaN, or a cycle of zero total length) draws
    /// solid. See [`Linetype::dash_pattern`].
    Custom(Vec<(f64, f64)>),
}

impl Linetype {
    /// All named linetypes in order (for discrete scale mapping).
    /// [`Linetype::Custom`] is not part of the cycle.
    pub const ALL: &[Linetype] = &[
        Linetype::Solid,
        Linetype::Dashed,
        Linetype::Dotted,
        Linetype::DashDot,
        Linetype::LongDash,
        Linetype::TwoDash,
    ];

    /// Get the dash pattern as (draw_len, gap_len) pairs in pixels, exactly
    /// as stored. For [`Linetype::Custom`] this is the caller's raw input;
    /// renderers use the sanitised [`Linetype::dash_pattern`] instead.
    pub fn pattern(&self) -> &[(f64, f64)] {
        match self {
            Linetype::Solid => &[],
            Linetype::Dashed => &[(6.0, 3.0)],
            Linetype::Dotted => &[(2.0, 2.0)],
            Linetype::DashDot => &[(6.0, 2.0), (2.0, 2.0)],
            Linetype::LongDash => &[(10.0, 4.0)],
            Linetype::TwoDash => &[(8.0, 3.0), (3.0, 3.0)],
            Linetype::Custom(pairs) => pairs,
        }
    }

    /// The dash pattern a renderer should draw: [`Linetype::pattern`] with
    /// non-finite pairs dropped and negative lengths clamped to `0`. Returns
    /// an empty vector (solid) when nothing usable is left or one full
    /// dash+gap cycle is shorter than 0.01 px (or overflows to infinity), so no backend ever sees a
    /// NaN, a negative length, or a zero-length cycle (which would never
    /// advance a dashed stroker).
    pub fn dash_pattern(&self) -> Vec<(f64, f64)> {
        let pairs: Vec<(f64, f64)> = self
            .pattern()
            .iter()
            .filter(|(d, g)| d.is_finite() && g.is_finite())
            .map(|&(d, g)| (d.max(0.0), g.max(0.0)))
            .collect();
        let cycle: f64 = pairs.iter().map(|(d, g)| d + g).sum();
        // No gap at all is a solid line too: emit nothing rather than a
        // dasharray with only zero gaps.
        let gaps: f64 = pairs.iter().map(|(_, g)| g).sum();
        if pairs.is_empty() || !cycle.is_finite() || cycle < 0.01 || gaps <= 0.0 {
            Vec::new()
        } else {
            pairs
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

/// A pattern fill for an area that has no single colour to draw — today
/// just a scale's `na_pattern` (#206): `ScaleFillBinned::na_pattern`,
/// drawn instead of `na_value`'s flat colour by `geom_sf`/`geom_polygon`/
/// `geom_rect`/`geom_tile` and the legend's "no data" key, so a classed
/// choropleth's no-data cells read as "we have no reading here" rather
/// than a greyed-out *value* easily mistaken for one.
///
/// `fg` stripes run at `angle_deg` (0 = horizontal, measured the usual
/// screen way, clockwise from the x axis), each `size_px` wide, over a
/// `bg` field — `fg` fills the first 30% of each cycle, `bg` the rest, a
/// thin hatch rather than an even 50/50 stripe (ggplot2/R's own
/// `pattern_stripe(density = 0.3)` default). More pattern kinds can join
/// this enum later without breaking callers that match it today only via
/// [`FillPattern::bg`]/[`FillPattern::color_at`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FillPattern {
    Stripe {
        fg: (u8, u8, u8),
        bg: (u8, u8, u8),
        /// The width of one fg+bg cycle, in output pixels. `<= 0.0` is
        /// treated as `1.0` (an unreasonably fine hatch rather than a
        /// division by zero/negative-period wraparound).
        size_px: f64,
        /// Stripe direction in degrees (0 = horizontal).
        angle_deg: f64,
    },
}

impl FillPattern {
    /// The fraction of each cycle `fg` covers (the rest is `bg`) — a thin
    /// hatch, not a 50/50 stripe.
    const STRIPE_FG_FRACTION: f64 = 0.3;

    /// A reasonable default stripe: `fg`/`bg` as given, 4px period, 45°.
    pub fn stripe(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> FillPattern {
        FillPattern::Stripe {
            fg,
            bg,
            size_px: 4.0,
            angle_deg: 45.0,
        }
    }

    /// The flat colour a backend with no pattern primitive falls back to
    /// (`plotters` — "falls back to bg", #206) — the pattern's background,
    /// so a no-data cell still reads as *muted*, just without the hatch.
    pub fn bg(&self) -> (u8, u8, u8) {
        match self {
            FillPattern::Stripe { bg, .. } => *bg,
        }
    }

    /// This pattern's colour at screen point `(x, y)` — a raster backend
    /// (pixel/`canvas`) fills per pixel with this ("raster stripes", #206)
    /// rather than approximating with [`FillPattern::bg`].
    pub fn color_at(&self, x: f64, y: f64) -> (u8, u8, u8) {
        match self {
            FillPattern::Stripe {
                fg,
                bg,
                size_px,
                angle_deg,
            } => {
                let (s, c) = angle_deg.to_radians().sin_cos();
                let rx = x * c + y * s;
                let size = if *size_px > 0.0 { *size_px } else { 1.0 };
                let frac = (rx / size).rem_euclid(1.0);
                if frac < Self::STRIPE_FG_FRACTION {
                    *fg
                } else {
                    *bg
                }
            }
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
    fn negative_radius_also_falls_back_to_a_plain_rectangle() {
        // `radius.max(0.0)` guards a caller passing a negative radius
        // (e.g. a miscomputed value) — it must not panic or produce a
        // garbage polygon, just the same plain-rect fallback as 0.0.
        let pts = rounded_rect_points(0.0, 0.0, 10.0, 20.0, -5.0);
        assert_eq!(
            pts,
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 20.0), (0.0, 20.0)]
        );
    }

    #[test]
    fn x_and_y_are_order_independent() {
        // Callers may pass (x1, y1) before (x0, y0) (e.g. a box whose
        // ymax < ymin before min/max normalisation elsewhere) — the
        // function itself must normalise, not assume sorted input.
        let forward = rounded_rect_points(0.0, 0.0, 10.0, 20.0, 3.0);
        let swapped = rounded_rect_points(10.0, 20.0, 0.0, 0.0, 3.0);
        assert_eq!(forward, swapped);
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
    /// Text halo / outline colour (#229, ggrepel's `bg.colour` /
    /// `shadowtext`'s `bg.col`): `None` (the default) draws no halo, so
    /// existing output is byte-identical. SVG: `stroke` on the `<text>`
    /// itself (`stroke-linejoin="round"`, `paint-order="stroke"`, so the
    /// stroke paints under the fill rather than over it). Raster: an
    /// outline pass in this colour before the normal fill pass.
    pub bg_colour: Option<(u8, u8, u8)>,
    /// Text halo radius in px (#229, ggrepel's `bg.r`) — the SVG
    /// `stroke-width` is `2 * bg_r`. Only matters when `bg_colour` is
    /// `Some`; `0.0` (the default) draws no halo even if `bg_colour` is
    /// set.
    pub bg_r: f64,
}

/// Font face / weight (R's `element_text(face = ...)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
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
            bg_colour: None,
            bg_r: 0.0,
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

    /// Draw a polygon with holes: `rings[0]` is the exterior, every other
    /// ring a hole cut out of it under the even-odd fill rule (`geom_sf`'s
    /// `Polygon`, OGC/`sf` convention — see
    /// [`spatial::Geometry::Polygon`](crate::spatial::Geometry::Polygon)).
    /// One `stroke`/`stroke_width` draws every ring's outline, exterior and
    /// holes alike (as `ggplot2`'s `geom_sf` does).
    ///
    /// Default: no real hole cut-out (for backends that pre-date this
    /// method) — the exterior ring filled and stroked, every other ring
    /// stroked only. Backends with a resolution-independent fill (SVG) or a
    /// scanline rasteriser (pixel/plotters) should override this to use
    /// their fill rule across every ring at once.
    fn draw_polygon_rings(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        for (i, ring) in rings.iter().enumerate() {
            let ring_style = if i == 0 {
                style.clone()
            } else {
                RectStyle {
                    fill: None,
                    ..style.clone()
                }
            };
            self.draw_polygon(ring, &ring_style)?;
        }
        Ok(())
    }

    /// Like [`draw_rect`](Self::draw_rect), but filled with `pattern`
    /// instead of `style.fill` (`geom_rect`/`geom_tile`'s no-data cells,
    /// #206) — `style.stroke`/`stroke_width`/`clip`/`corner_radius` still
    /// apply, only the fill differs.
    ///
    /// Default: flattens `pattern` to its [`FillPattern::bg`] and draws an
    /// ordinary [`draw_rect`](Self::draw_rect) — the `plotters` backend
    /// has no pattern-fill primitive and keeps this default ("falls back
    /// to bg"). The native SVG backend overrides this to register a
    /// `<defs><pattern>` (deduped, this backend's own id prefix) and fill
    /// with its `url(#…)` reference; the pixel/`canvas` backend overrides
    /// it to rasterise the actual stripes per pixel ("raster stripes").
    fn draw_rect_pattern(
        &mut self,
        top_left: (f64, f64),
        bottom_right: (f64, f64),
        pattern: &FillPattern,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        self.draw_rect(
            top_left,
            bottom_right,
            &RectStyle {
                fill: Some(pattern.bg()),
                ..style.clone()
            },
        )
    }

    /// Like [`draw_polygon`](Self::draw_polygon), but filled with
    /// `pattern` instead of `style.fill` (#206). Default/override split:
    /// see [`draw_rect_pattern`](Self::draw_rect_pattern).
    fn draw_polygon_pattern(
        &mut self,
        points: &[(f64, f64)],
        pattern: &FillPattern,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        self.draw_polygon(
            points,
            &RectStyle {
                fill: Some(pattern.bg()),
                ..style.clone()
            },
        )
    }

    /// Like [`draw_polygon_rings`](Self::draw_polygon_rings), but filled
    /// with `pattern` instead of `style.fill` (`geom_sf`/`geom_polygon`'s
    /// no-data features, #206) — holes still cut out under the even-odd
    /// rule, exactly as a flat fill would. Default/override split: see
    /// [`draw_rect_pattern`](Self::draw_rect_pattern).
    fn draw_polygon_rings_pattern(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        pattern: &FillPattern,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        self.draw_polygon_rings(
            rings,
            &RectStyle {
                fill: Some(pattern.bg()),
                ..style.clone()
            },
        )
    }

    /// Like [`draw_polygon_rings`](Self::draw_polygon_rings)/
    /// [`draw_polygon_rings_pattern`](Self::draw_polygon_rings_pattern),
    /// but lets the caller ask for a compact path encoding at a given
    /// decimal `precision` — relative commands after the first `M`
    /// instead of absolute `L` per point, geometrically identical within
    /// that precision (#214). `geom_sf` ties `precision` to its own
    /// `simplify_px` (more simplification tolerates coarser precision);
    /// every other caller of the two methods above is untouched, so their
    /// output doesn't move.
    ///
    /// Default: `precision` is ignored and this just dispatches to
    /// [`draw_polygon_rings`](Self::draw_polygon_rings)/
    /// [`draw_polygon_rings_pattern`](Self::draw_polygon_rings_pattern) —
    /// correct (if not byte-compact) for every backend, including ones
    /// that pre-date this method. Only the native SVG backend, which
    /// actually writes path text, overrides it to shrink the encoding
    /// when `precision` is `Some`.
    fn draw_polygon_rings_precise(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        pattern: Option<&FillPattern>,
        precision: Option<u8>,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let _ = precision;
        match pattern {
            Some(p) => self.draw_polygon_rings_pattern(rings, p, style),
            None => self.draw_polygon_rings(rings, style),
        }
    }

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

    /// Draw one line as consecutive runs, each optionally an `http(s)`
    /// hyperlink (ggspatial-style attribution: "© BKG" linked, a licence
    /// code linked, plain connective text between them —
    /// `annotation_attribution`'s `.link()`/`.runs()`, #196). `pos`/`style`
    /// place and style the line as a whole, same as [`draw_text`](Self::draw_text)
    /// — `style.anchor` aligns the *total* run sequence, not each run.
    ///
    /// Default: concatenates the runs' text and draws it plain with
    /// [`draw_text`](Self::draw_text), dropping every link — the sensible
    /// fallback for a backend with no link primitive (pixel/`canvas`,
    /// `plotters`). The native SVG backend overrides this to wrap each
    /// linked run in `<a href="…" target="_blank" rel="noopener">` with an
    /// underline, inside one `<text>` element so the anchor still aligns
    /// the whole line.
    fn draw_text_runs(
        &mut self,
        runs: &[(&str, Option<&str>)],
        pos: (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        let joined: String = runs.iter().map(|(t, _)| *t).collect();
        self.draw_text(&joined, pos, style)
    }
    /// Draw an encoded raster image (PNG or JPEG bytes) stretched over
    /// `rect`, clipped to `clip`, at opacity `alpha` — a map tile (#325).
    /// Default: draws nothing (a backend without raster support).
    fn draw_image(
        &mut self,
        _bytes: &[u8],
        _rect: &Rect,
        _clip: &Rect,
        _alpha: f64,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn plot_area(&self) -> Rect;
    fn total_area(&self) -> Rect;

    /// Every box already claimed in a panel corner via
    /// [`Self::claim_corner_box`] so far this render pass — the scale
    /// bar, attribution text, bivariate legend, or an inside ordinary
    /// legend already placed (`render::corner_layout`, #273). Default:
    /// always empty — a backend that never overrides this (anything that
    /// pre-dates #273, including a third-party `impl DrawBackend`) keeps
    /// every corner item positioned exactly as before, byte-identical.
    fn corner_claims(&self) -> Vec<Rect> {
        Vec::new()
    }

    /// Register `rect` as claimed in a panel corner, for a later corner
    /// geom in the same render pass to see via [`Self::corner_claims`]
    /// (#273). Default: no-op, paired with that method's empty default.
    fn claim_corner_box(&mut self, _rect: Rect) {}

    /// Clear every box claimed via [`Self::claim_corner_box`] — called
    /// once per panel (`renderer.rs`), so a facet's second panel doesn't
    /// see the first panel's claims (#273). Default: no-op, paired with
    /// the above.
    fn clear_corner_claims(&mut self) {}

    /// Start one panel of a fixed-aspect `facet_wrap` (e.g. `coord_sf` small
    /// multiples): marks until [`end_panel`](Self::end_panel) belong to
    /// panel `index` (strip label `label`), clipped to `clip`; `data` is the
    /// rect the coord maps the data window onto. The SVG backend wraps them
    /// in a clipped `<g data-facet-panel>` and lists every panel on the root
    /// as `data-panels`. Default: no-op.
    fn begin_panel(
        &mut self,
        _index: usize,
        _label: &str,
        _clip: &crate::render::Rect,
        _data: &crate::render::Rect,
    ) {
    }

    /// End the panel started by [`begin_panel`](Self::begin_panel).
    /// Default: no-op.
    fn end_panel(&mut self) {}

    /// The rect a single-panel plot's coord maps the data window onto, when
    /// it differs from the plot area (a letterboxing coord such as
    /// `coord_sf`). The SVG backend writes it as the root `data-viewport`.
    /// Default: no-op.
    fn set_viewport(&mut self, _rect: &crate::render::Rect) {}

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

    /// Extra CSS class tokens for subsequently-drawn marks (#221's
    /// `Aes::class`), space-separated and already validated by the caller;
    /// `None` clears them, per row. The SVG backend appends them to the
    /// mark's own `class="gg-mark gg-<kind>"` (re-validating every token as
    /// defence in depth). Default: no-op.
    fn set_mark_class(&mut self, _class: Option<String>) {}

    /// Put subsequently-drawn marks into the caller-named group
    /// `(attribute name, value)` — e.g. `("data-of", "node-7")` — until
    /// cleared with `None` (#221's `Aes::group_wrap`). The SVG backend
    /// collects every mark with the same pair into one `<g name="value">`,
    /// placed where the group's first mark was drawn. Default: no-op (a
    /// raster backend has no grouping).
    fn set_mark_group(&mut self, _group: Option<(String, String)>) {}

    /// Per-layer SVG attributes for every mark the current layer draws
    /// (#221's [`GGPlot::layer_svg_attr`](crate::plot::GGPlot::layer_svg_attr)):
    /// `class` tokens, `aria-hidden`, `pointer-events`, `data-*`, already
    /// validated by the caller. An empty `Vec` clears them (set before a
    /// layer draws, cleared after). Default: no-op.
    fn set_layer_attrs(&mut self, _attrs: Vec<(String, String)>) {}

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
    /// One `draw_polygon` call's recorded points and fill colour.
    type RecordedPolygon = (Vec<(f64, f64)>, Option<(u8, u8, u8)>);
    /// One `draw_rect` call's corners and fill colour.
    type RecordedRect = ((f64, f64), (f64, f64), Option<(u8, u8, u8)>);

    #[derive(Default)]
    struct RecordingBackend {
        lines: RefCell<Vec<Vec<(f64, f64)>>>,
        circles: RefCell<Vec<((f64, f64), f64)>>,
        polygons: RefCell<Vec<RecordedPolygon>>,
        rects: RefCell<Vec<RecordedRect>>,
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
            top_left: (f64, f64),
            bottom_right: (f64, f64),
            style: &RectStyle,
        ) -> Result<(), RenderError> {
            self.rects
                .borrow_mut()
                .push((top_left, bottom_right, style.fill));
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
            points: &[(f64, f64)],
            style: &RectStyle,
        ) -> Result<(), RenderError> {
            self.polygons
                .borrow_mut()
                .push((points.to_vec(), style.fill));
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

    /// The default `draw_polygon_rings` (for a backend pre-dating #178, or
    /// any third-party `DrawBackend` that never overrides it) falls back to
    /// one `draw_polygon` call per ring: the exterior filled, every other
    /// ring (a hole) with its fill cleared — the old approximation, not a
    /// real cut-out, but it draws *something* rather than nothing.
    #[test]
    fn default_draw_polygon_rings_fills_only_the_exterior() {
        let mut b = RecordingBackend::default();
        let exterior = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let hole = vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)];
        b.draw_polygon_rings(
            &[exterior.clone(), hole.clone()],
            &RectStyle {
                fill: Some((200, 50, 50)),
                stroke: Some((0, 0, 0)),
                stroke_width: 1.0,
                alpha: 1.0,
                clip: true,
                corner_radius: 0.0,
            },
        )
        .unwrap();
        let polys = b.polygons.borrow();
        assert_eq!(polys.len(), 2, "one draw_polygon call per ring");
        assert_eq!(polys[0], (exterior, Some((200, 50, 50))), "exterior filled");
        assert_eq!(polys[1].0, hole);
        assert_eq!(polys[1].1, None, "hole not filled (no real cut-out)");
    }

    // #206: the default `draw_*_pattern` methods (a backend with no
    // pattern-fill primitive — `plotters` is the real-world case, this
    // mock stands in for it) flatten to the pattern's own `bg` and call
    // the plain draw method, unchanged otherwise.

    fn stripe() -> FillPattern {
        FillPattern::stripe((10, 20, 30), (200, 210, 220))
    }

    #[test]
    fn default_draw_rect_pattern_flattens_to_bg() {
        let mut b = RecordingBackend::default();
        b.draw_rect_pattern(
            (0.0, 0.0),
            (10.0, 10.0),
            &stripe(),
            &RectStyle {
                stroke: Some((0, 0, 0)),
                ..Default::default()
            },
        )
        .unwrap();
        let rects = b.rects.borrow();
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0], ((0.0, 0.0), (10.0, 10.0), Some(stripe().bg())));
    }

    #[test]
    fn default_draw_polygon_pattern_flattens_to_bg() {
        let mut b = RecordingBackend::default();
        let pts = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        b.draw_polygon_pattern(&pts, &stripe(), &RectStyle::default())
            .unwrap();
        let polys = b.polygons.borrow();
        assert_eq!(polys.len(), 1);
        assert_eq!(polys[0], (pts, Some(stripe().bg())));
    }

    #[test]
    fn default_draw_polygon_rings_pattern_flattens_to_bg_per_ring() {
        let mut b = RecordingBackend::default();
        let exterior = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let hole = vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)];
        b.draw_polygon_rings_pattern(&[exterior.clone(), hole], &stripe(), &RectStyle::default())
            .unwrap();
        let polys = b.polygons.borrow();
        // Same fallback shape as draw_polygon_rings's own default: the
        // exterior filled (with the pattern's bg), the hole not.
        assert_eq!(polys[0], (exterior, Some(stripe().bg())));
        assert_eq!(polys[1].1, None);
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
