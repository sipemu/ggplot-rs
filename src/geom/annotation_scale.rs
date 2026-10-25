//! `annotation_scale()` / `annotation_attribution()` (feature `sf`) —
//! ggspatial's map furniture: a metric scale bar computed from the
//! projection at the map's centre, and a small attribution text in a
//! corner. Ported from anofox-visualization's `map/mod.rs` (DataZooDE/
//! anofox-visualization#318).
//!
//! Both read the trained X position scale's domain directly rather than
//! any data column, so they add no required aesthetic and no row loop —
//! a plot adds at most one of each, like a legend. The scale bar assumes
//! the X/Y domain is in metres (any `geom_sf` projection other than
//! `SfProjection::PlateCarree`, which stays in degrees and has no single
//! metric scale) — same constraint ggspatial's own `annotation_scale()`
//! has for an unprojected CRS.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, TextAnchor, TextStyle};
use crate::render::text::{text_width, truncate_to_width, wrap_text};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// One attribution run: text, plus an optional `http(s)` link href — see
/// [`GeomAttribution::runs`].
type AttributionRun = (String, Option<String>);
/// One attribution line's run list — see [`GeomAttribution::runs`].
type AttributionRow = Vec<AttributionRun>;

/// Which panel corner a scale bar / attribution sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MapCorner {
    #[default]
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

/// What [`GeomAttribution::fit`] does once shrinking a line's text size
/// down to its floor still isn't enough to keep it inside the panel
/// (#258).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AttributionOverflow {
    /// Truncate the line with a trailing "…" (ggplot-rs's own
    /// [`crate::render::text::truncate_to_width`], same as a legend's
    /// label-truncation fallback, #159).
    #[default]
    Ellipsis,
    /// Wrap the remainder onto additional physical lines, pushing any
    /// further attribution lines further from the corner's edge.
    Wrap,
}

/// A metric scale bar (ggspatial's `annotation_scale()`): a "nice" 1/2/5 ×
/// 10ⁿ distance at most `max_width_px` long, computed from the trained X
/// scale's domain at the panel's centre height.
pub struct GeomAnnotationScale {
    pub corner: MapCorner,
    pub max_width_px: f64,
    pub color: (u8, u8, u8),
    pub text_size: f64,
    pub line_width: f64,
}

impl Default for GeomAnnotationScale {
    fn default() -> Self {
        GeomAnnotationScale {
            corner: MapCorner::default(),
            max_width_px: 90.0,
            color: (80, 80, 80),
            text_size: 8.0,
            line_width: 1.0,
        }
    }
}

impl GeomAnnotationScale {
    pub fn corner(mut self, corner: MapCorner) -> Self {
        self.corner = corner;
        self
    }

    /// The scale bar's maximum length in pixels (it picks the largest
    /// "nice" 1/2/5 × 10ⁿ distance that still fits). Default `90.0`.
    pub fn max_width_px(mut self, px: f64) -> Self {
        self.max_width_px = px.max(1.0);
        self
    }
}

impl Geom for GeomAnnotationScale {
    fn draw(
        &self,
        _data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let plot_area = backend.plot_area();
        let Some((px, label)) = scale_bar_geometry(coord, scales, &plot_area, self.max_width_px)
        else {
            return Ok(());
        };
        let (x0, y0) = corner_origin(self.corner, &plot_area, px, self.text_size);
        // A staple/I-beam: two short end-ticks joining a horizontal run,
        // like ggspatial's own scale bar.
        backend.draw_line(
            &[(x0, y0), (x0, y0 + 3.0), (x0 + px, y0 + 3.0), (x0 + px, y0)],
            &LineStyle {
                color: self.color,
                alpha: 1.0,
                width: self.line_width,
                linetype: Linetype::Solid,
                ..Default::default()
            },
        )?;
        backend.draw_text(
            &label,
            (x0 + px / 2.0, y0 - 5.0),
            &TextStyle {
                color: self.color,
                size: self.text_size,
                anchor: TextAnchor::Middle,
                angle: 0.0,
                family: None,
                face: crate::render::backend::FontFace::Plain,
                bg_colour: None,
                bg_r: 0.0,
            },
        )?;
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![]
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
        "annotation_scale"
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (80, 80, 80), el.ink) {
            self.color = ink;
        }
    }
}

/// A small, muted attribution text (ggspatial's `annotation_scale()`
/// sibling convention: `"© Natural Earth"` etc.) in a panel corner.
///
/// `#[non_exhaustive]` (#250/#176 precedent): build via [`Self::new`] and
/// the `.corner(..)`/`.link(..)`/`.runs(..)`/`.line(..)` builders, not a
/// struct literal — this keeps a future field additive. `GeomText`/
/// `GeomLabel` stay plain `pub` structs (downstream code builds those
/// with `..Default::default()` literals, which `#[non_exhaustive]` would
/// forbid); `GeomAttribution` has no such existing convention to protect.
#[non_exhaustive]
pub struct GeomAttribution {
    pub text: String,
    pub corner: MapCorner,
    pub color: (u8, u8, u8),
    pub text_size: f64,
    /// Linked segments overriding `text` — see [`Self::link`]/[`Self::runs`].
    /// `None`: draw `text` plain, as before #196. This is row 0 — the line
    /// closest to the corner's edge.
    pub runs: Option<Vec<AttributionRun>>,
    /// Additional stacked lines beyond row 0, each its own run list — see
    /// [`Self::line`] (#250).
    pub lines: Vec<AttributionRow>,
    /// Fit-to-panel floor size in px, or `None` to disable fitting (the
    /// default, byte-identical to before #258) — see [`Self::fit`].
    pub fit_min_text_size: Option<f64>,
    /// What to do once shrinking alone can't fit a line — see
    /// [`Self::fit_overflow`]. Only consulted when `fit_min_text_size` is
    /// `Some`.
    pub fit_overflow: AttributionOverflow,
}

impl GeomAttribution {
    pub fn new(text: impl Into<String>) -> Self {
        GeomAttribution {
            text: text.into(),
            corner: MapCorner::BottomRight,
            color: (130, 130, 130),
            text_size: 7.5,
            runs: None,
            lines: Vec::new(),
            fit_min_text_size: None,
            fit_overflow: AttributionOverflow::Ellipsis,
        }
    }

    pub fn corner(mut self, corner: MapCorner) -> Self {
        self.corner = corner;
        self
    }

    /// Make the whole attribution text one hyperlink — ggspatial has no
    /// equivalent (plain text only); a source that asks for a single link
    /// (e.g. "Made with Natural Earth" → naturalearthdata.com) wants this.
    /// `url` must be `http(s)`; anything else is dropped at render time
    /// with a warning rather than drawn as a link (#196).
    pub fn link(mut self, url: impl Into<String>) -> Self {
        let text = self.text.clone();
        self.runs = Some(vec![(text, Some(url.into()))]);
        self
    }

    /// Replace `text` with several runs, each independently plain or
    /// linked (`None` href: plain) — ggspatial-style multi-segment
    /// attribution: "© BKG" linked, a licence code linked, plain
    /// connective text between them (DataZooDE/anofox-visualization#420's
    /// BKG/VG2500 source note). Overrides a prior [`Self::link`] call. A
    /// non-`http(s)` href is dropped at render time with a warning, same
    /// as [`Self::link`] (#196).
    pub fn runs(mut self, runs: Vec<(&str, Option<&str>)>) -> Self {
        self.runs = Some(
            runs.into_iter()
                .map(|(t, u)| (t.to_string(), u.map(str::to_string)))
                .collect(),
        );
        self
    }

    /// Append another stacked line, with its own run list (same
    /// plain/linked-segment semantics as [`Self::runs`]) — e.g. a data
    /// licence note that needs several lines, such as BKG's dl-de/by-2-0
    /// note for German boundaries (DataZooDE/anofox-visualization#420).
    /// `.runs(..)` (or plain `text`, if `.runs(..)` was never called) is
    /// row 0; the first `.line(..)` call appends row 1, the next row 2,
    /// and so on. Rows stack away from the corner's edge, bottom corners
    /// upward and top corners downward, each row `text_size * 1.3` px
    /// from the last — the same line-height multiple a single-line
    /// attribution already uses implicitly. A single line (no `.line(..)`
    /// calls) draws byte-identically to before #250 (#196).
    pub fn line(mut self, runs: Vec<(&str, Option<&str>)>) -> Self {
        self.lines.push(
            runs.into_iter()
                .map(|(t, u)| (t.to_string(), u.map(str::to_string)))
                .collect(),
        );
        self
    }

    /// Fit each attribution line inside the final panel width: once the
    /// panel is narrow enough that a line would overflow at `text_size`
    /// (e.g. a `Right` legend eating into the panel), shrink that line's
    /// text size down to `min_size`, then — if it still doesn't fit —
    /// truncate or wrap it (see [`Self::fit_overflow`]). Default: `None`
    /// (fitting off), which keeps output byte-identical to before #258;
    /// call this to opt in. `min_size` is clamped to `text_size` itself
    /// (shrinking never enlarges a line) and to a sane positive floor.
    pub fn fit(mut self, min_size: f64) -> Self {
        self.fit_min_text_size = Some(min_size.max(0.1));
        self
    }

    /// Once [`Self::fit`]'s shrinking alone isn't enough, choose
    /// truncate-with-ellipsis (the default) or wrap onto additional
    /// physical lines (#258). Has no effect unless `.fit(..)` was called.
    pub fn fit_overflow(mut self, overflow: AttributionOverflow) -> Self {
        self.fit_overflow = overflow;
        self
    }
}

/// Shrinks `row`'s text size toward `min_size` (clamped to `base_size`) so
/// its concatenated text fits `max_width` px, then truncates (ellipsis) or
/// wraps once shrinking alone isn't enough — see [`GeomAttribution::fit`]
/// (#258). Returns the text size to draw at, and the physical lines to
/// draw: almost always one line (`row` unchanged) — more than one only
/// when `overflow` is [`AttributionOverflow::Wrap`] and the line still
/// doesn't fit at `min_size`. Mangling a row into a truncated/wrapped
/// plain line drops its per-run link structure (the href no longer
/// matches a shortened substring), so that only happens as a last resort.
fn fit_row(
    row: &[AttributionRun],
    base_size: f64,
    min_size: f64,
    max_width: f64,
    overflow: AttributionOverflow,
) -> (f64, Vec<AttributionRow>) {
    let min_size = min_size.min(base_size).max(0.1);
    if !(max_width.is_finite()) || max_width <= 0.0 {
        return (base_size, vec![row.to_vec()]);
    }
    let concat: String = row.iter().map(|(t, _)| t.as_str()).collect();
    let full_width = text_width(&concat, base_size);
    if full_width <= max_width {
        return (base_size, vec![row.to_vec()]);
    }
    // `text_width` scales linearly with size, so this lands close to the
    // exact fitting size in one step; the `<=` check below covers any
    // rounding slack.
    let shrunk_size = (base_size * max_width / full_width).max(min_size);
    if text_width(&concat, shrunk_size) <= max_width {
        return (shrunk_size, vec![row.to_vec()]);
    }
    match overflow {
        AttributionOverflow::Ellipsis => {
            let truncated = truncate_to_width(&concat, max_width, min_size).unwrap_or(concat);
            (min_size, vec![vec![(truncated, None)]])
        }
        AttributionOverflow::Wrap => {
            let lines = wrap_text(&concat, max_width, min_size);
            (
                min_size,
                lines.into_iter().map(|l| vec![(l, None)]).collect(),
            )
        }
    }
}

/// `url` is an absolute `http://`/`https://` URL (case-insensitive scheme),
/// not `javascript:`, `data:`, a bare path or anything else a backend
/// might otherwise be asked to wrap a hyperlink around (#196: "http(s)
/// URLs only").
fn is_http_url(url: &str) -> bool {
    let lower = url.trim_start().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

impl Geom for GeomAttribution {
    fn draw(
        &self,
        _data: &DataFrame,
        _coord: &dyn Coord,
        _scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        // `runs` (set by `.link()`/`.runs()`) overrides plain `text` for
        // row 0; a non-http(s) href is dropped — a warning, not a
        // silently-wrong link (#196).
        let validate_runs = |backend: &mut dyn DrawBackend, runs: &[AttributionRun]| {
            runs.iter()
                .map(|(t, u)| match u {
                    Some(url) if !is_http_url(url) => {
                        backend.warn(format!(
                            "annotation_attribution: \"{url}\" is not an http(s) URL; drawing \"{t}\" as plain text"
                        ));
                        (t.clone(), None)
                    }
                    other => (t.clone(), other.clone()),
                })
                .collect::<Vec<AttributionRun>>()
        };
        let row0: AttributionRow = match &self.runs {
            Some(r) => validate_runs(backend, r),
            None => vec![(self.text.clone(), None)],
        };
        let extra_rows: Vec<AttributionRow> = self
            .lines
            .iter()
            .map(|r| validate_runs(backend, r))
            .collect();
        let all_empty = row0.iter().all(|(t, _)| t.is_empty())
            && extra_rows
                .iter()
                .all(|row| row.iter().all(|(t, _)| t.is_empty()));
        if all_empty {
            return Ok(());
        }
        let plot_area = backend.plot_area();
        let (anchor, x) = match self.corner {
            MapCorner::BottomLeft | MapCorner::TopLeft => (TextAnchor::Start, plot_area.x + 4.0),
            MapCorner::BottomRight | MapCorner::TopRight => {
                (TextAnchor::End, plot_area.x + plot_area.width - 4.0)
            }
        };
        // Row 0's baseline — unchanged from before #250. Additional rows
        // stack away from the corner's own edge: upward (smaller y) for
        // the bottom corners, downward (larger y) for the top corners, so
        // a map adding more lines never needs to know any other line's
        // position.
        let base_y = match self.corner {
            MapCorner::BottomLeft | MapCorner::BottomRight => plot_area.y + plot_area.height - 4.0,
            MapCorner::TopLeft | MapCorner::TopRight => plot_area.y + 4.0 + self.text_size / 2.0,
        };
        // Spacing between physical lines is always based on the nominal
        // `text_size`, not any one line's shrunk size — a safe upper
        // bound that never lets a shrunk (smaller) line collide with its
        // neighbour.
        let line_height = self.text_size * 1.3;
        // `.fit(min_size)` (#258): each logical row (row0, then each
        // `.line(..)`) may expand into more than one physical line when
        // wrapped, so collect the physical lines up front rather than
        // enumerate the logical rows directly.
        let logical_rows = std::iter::once(row0).chain(extra_rows);
        let physical_lines: Vec<(f64, AttributionRow)> =
            if let Some(min_size) = self.fit_min_text_size {
                let max_width = (plot_area.width - 8.0).max(0.0);
                logical_rows
                    .flat_map(|row| {
                        let (size, lines) =
                            fit_row(&row, self.text_size, min_size, max_width, self.fit_overflow);
                        lines
                            .into_iter()
                            .map(move |l| (size, l))
                            .collect::<Vec<_>>()
                    })
                    .collect()
            } else {
                logical_rows.map(|r| (self.text_size, r)).collect()
            };
        for (i, (size, row)) in physical_lines.into_iter().enumerate() {
            if row.iter().all(|(t, _)| t.is_empty()) {
                continue;
            }
            let y = match self.corner {
                MapCorner::BottomLeft | MapCorner::BottomRight => base_y - (i as f64) * line_height,
                MapCorner::TopLeft | MapCorner::TopRight => base_y + (i as f64) * line_height,
            };
            let run_refs: Vec<(&str, Option<&str>)> = row
                .iter()
                .map(|(t, u)| (t.as_str(), u.as_deref()))
                .collect();
            let style = TextStyle {
                color: self.color,
                size,
                anchor: anchor.clone(),
                angle: 0.0,
                family: None,
                face: crate::render::backend::FontFace::Plain,
                bg_colour: None,
                bg_r: 0.0,
            };
            backend.draw_text_runs(&run_refs, (x, y), &style)?;
        }
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![]
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
        "attribution"
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (130, 130, 130), el.ink) {
            self.color = ink;
        }
    }
}

/// `(bar length in px, "50 km"/"300 m" label)`, or `None` when the X scale
/// isn't trained, has a degenerate domain, or the nicest bar would be
/// under 12px (too small to show). Computed from two domain points mapped
/// through the full scale → coord pipeline at the panel's mid-height, so
/// it reflects the actual on-screen scale at the map's centre — correct
/// for a non-conformal projection (Equal Earth, Albers) too, not just a
/// uniform one.
fn scale_bar_geometry(
    coord: &dyn Coord,
    scales: &ScaleSet,
    plot_area: &crate::render::Rect,
    max_width_px: f64,
) -> Option<(f64, String)> {
    let x_scale = scales.get(&Aesthetic::X)?;
    let (d0, d1) = x_scale.domain()?;
    if !(d0.is_finite() && d1.is_finite()) || d1 <= d0 {
        return None;
    }
    let y_scale = scales.get(&Aesthetic::Y);
    let ny = y_scale
        .and_then(|s| s.domain())
        .map(|(y0, y1)| {
            let mid = Value::Float((y0 + y1) / 2.0);
            y_scale.unwrap().map(&mid)
        })
        .unwrap_or(0.5);
    let p0 = coord.transform((x_scale.map(&Value::Float(d0)), ny), plot_area);
    let p1 = coord.transform((x_scale.map(&Value::Float(d1)), ny), plot_area);
    let total_px = (p1.0 - p0.0).hypot(p1.1 - p0.1);
    if !(total_px.is_finite()) || total_px <= 0.0 {
        return None;
    }
    let px_per_unit = total_px / (d1 - d0);
    // px per km, assuming the domain unit is metres.
    let px_per_km = px_per_unit * 1000.0;
    if !(px_per_km.is_finite()) || px_per_km <= 0.0 {
        return None;
    }
    let max_km = max_width_px / px_per_km;
    if !(max_km.is_finite()) || max_km <= 0.0 {
        return None;
    }
    let mag = 10f64.powf(max_km.log10().floor());
    let km = [5.0, 2.0, 1.0]
        .iter()
        .map(|f| f * mag)
        .find(|k| *k <= max_km)
        .unwrap_or(mag);
    let px = km * px_per_km;
    if !px.is_finite() || px < 12.0 {
        return None;
    }
    let label = if km >= 1.0 {
        format!("{} km", km.round() as i64)
    } else {
        format!("{} m", (km * 1000.0).round() as i64)
    };
    Some((px, label))
}

fn corner_origin(
    corner: MapCorner,
    plot_area: &crate::render::Rect,
    bar_px: f64,
    text_size: f64,
) -> (f64, f64) {
    let pad_x = 6.0;
    let pad_y = 8.0 + text_size;
    match corner {
        MapCorner::BottomLeft => (plot_area.x + pad_x, plot_area.y + plot_area.height - pad_y),
        MapCorner::BottomRight => (
            plot_area.x + plot_area.width - pad_x - bar_px,
            plot_area.y + plot_area.height - pad_y,
        ),
        MapCorner::TopLeft => (plot_area.x + pad_x, plot_area.y + pad_y),
        MapCorner::TopRight => (
            plot_area.x + plot_area.width - pad_x - bar_px,
            plot_area.y + pad_y,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::cartesian::CoordCartesian;
    use crate::scale::continuous::ScaleContinuous;
    use crate::scale::Scale;

    fn trained_scales(d0: f64, d1: f64) -> ScaleSet {
        let mut scales = ScaleSet::new();
        let mut x = ScaleContinuous::new().for_aesthetic(Aesthetic::X);
        x.train(&[Value::Float(d0), Value::Float(d1)]);
        scales.add(Box::new(x));
        let mut y = ScaleContinuous::new().for_aesthetic(Aesthetic::Y);
        y.train(&[Value::Float(0.0), Value::Float(1000.0)]);
        scales.add(Box::new(y));
        scales
    }

    fn panel() -> crate::render::Rect {
        crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        }
    }

    #[test]
    fn picks_a_nice_round_distance() {
        // 400,000 m across a 400px-wide-ish panel: ~1000 m/px before
        // margins; a 90px max bar should land on a tidy 1/2/5 figure.
        let scales = trained_scales(0.0, 400_000.0);
        let coord = CoordCartesian::default();
        let (px, label) = scale_bar_geometry(&coord, &scales, &panel(), 90.0).unwrap();
        assert!((12.0..=90.0).contains(&px), "px = {px}");
        assert!(
            label.ends_with(" km") || label.ends_with(" m"),
            "label = {label}"
        );
        // The label's leading number should be one of the "nice" values.
        let n: f64 = label.split_whitespace().next().unwrap().parse().unwrap();
        let mag = 10f64.powf(n.log10().floor());
        assert!(
            [1.0, 2.0, 5.0].contains(&(n / mag).round()),
            "not a nice 1/2/5 figure: {label}"
        );
    }

    #[test]
    fn too_small_a_bar_is_omitted() {
        // A huge domain over a tiny max width collapses the nicest bar
        // under 12px.
        let scales = trained_scales(0.0, 50_000_000.0);
        let coord = CoordCartesian::default();
        assert!(scale_bar_geometry(&coord, &scales, &panel(), 5.0).is_none());
    }

    #[test]
    fn untrained_scale_has_no_bar() {
        let scales = ScaleSet::new();
        let coord = CoordCartesian::default();
        assert!(scale_bar_geometry(&coord, &scales, &panel(), 90.0).is_none());
    }

    #[test]
    fn corner_placement_matches_each_corner() {
        let p = panel();
        let (x_bl, y_bl) = corner_origin(MapCorner::BottomLeft, &p, 50.0, 8.0);
        let (x_br, y_br) = corner_origin(MapCorner::BottomRight, &p, 50.0, 8.0);
        let (x_tl, y_tl) = corner_origin(MapCorner::TopLeft, &p, 50.0, 8.0);
        let (x_tr, y_tr) = corner_origin(MapCorner::TopRight, &p, 50.0, 8.0);
        assert!(x_bl < x_br, "left < right x");
        assert!(y_tl < y_bl, "top < bottom y");
        assert_eq!(x_tl, x_bl);
        assert_eq!(x_tr, x_br);
        assert_eq!(y_tl, y_tr);
        assert_eq!(y_bl, y_br);
    }

    #[test]
    fn default_corner_is_bottom_left() {
        assert_eq!(MapCorner::default(), MapCorner::BottomLeft);
    }

    #[test]
    fn is_http_url_accepts_only_http_and_https() {
        assert!(is_http_url("https://www.bkg.bund.de"));
        assert!(is_http_url("http://example.com"));
        assert!(is_http_url("  https://example.com")); // leading whitespace
        assert!(is_http_url("HTTPS://EXAMPLE.COM"));
        assert!(!is_http_url("javascript:alert(1)"));
        assert!(!is_http_url("data:text/html,<script>"));
        assert!(!is_http_url("//example.com"));
        assert!(!is_http_url("example.com"));
        assert!(!is_http_url(""));
    }

    #[test]
    fn link_sets_a_single_run_with_the_whole_text() {
        let g = GeomAttribution::new("Natural Earth").link("https://www.naturalearthdata.com");
        assert_eq!(
            g.runs,
            Some(vec![(
                "Natural Earth".to_string(),
                Some("https://www.naturalearthdata.com".to_string())
            )])
        );
    }

    #[test]
    fn runs_overrides_text_with_several_segments() {
        let g = GeomAttribution::new("ignored")
            .runs(vec![("© ", None), ("BKG", Some("https://www.bkg.bund.de"))]);
        assert_eq!(
            g.runs,
            Some(vec![
                ("© ".to_string(), None),
                (
                    "BKG".to_string(),
                    Some("https://www.bkg.bund.de".to_string())
                ),
            ])
        );
    }

    #[test]
    fn fit_is_off_by_default() {
        let g = GeomAttribution::new("x");
        assert_eq!(g.fit_min_text_size, None);
        assert_eq!(g.fit_overflow, AttributionOverflow::Ellipsis);
    }

    #[test]
    fn fit_row_leaves_a_fitting_row_untouched_at_full_size() {
        let row = vec![("short".to_string(), None)];
        let (size, lines) = fit_row(&row, 8.0, 4.0, 1000.0, AttributionOverflow::Ellipsis);
        assert_eq!(size, 8.0);
        assert_eq!(lines, vec![row]);
    }

    #[test]
    fn fit_row_shrinks_toward_the_floor_when_that_alone_fits() {
        let row = vec![("a moderately long attribution line".to_string(), None)];
        let full_width = text_width(&row[0].0, 8.0);
        // A width the full-size line overflows, but that a size still
        // above the floor can satisfy.
        let max_width = full_width * 0.7;
        let (size, lines) = fit_row(&row, 8.0, 4.0, max_width, AttributionOverflow::Ellipsis);
        assert!((4.0..8.0).contains(&size), "size = {size}");
        assert_eq!(lines, vec![row]); // unchanged text — only size shrank
        assert!(text_width(&lines[0][0].0, size) <= max_width + 1e-6);
    }

    #[test]
    fn fit_row_truncates_with_ellipsis_once_the_floor_still_overflows() {
        let row = vec![(
            "https://example.com/sources · geometry modified (simplified)".to_string(),
            None,
        )];
        let max_width = 40.0; // too narrow even at the 4px floor
        let (size, lines) = fit_row(&row, 8.0, 4.0, max_width, AttributionOverflow::Ellipsis);
        assert_eq!(size, 4.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 1);
        assert!(lines[0][0].0.ends_with('…'), "line = {:?}", lines[0][0].0);
        assert!(text_width(&lines[0][0].0, size) <= max_width + 1e-6);
    }

    #[test]
    fn fit_row_wraps_once_the_floor_still_overflows() {
        let row = vec![(
            "https://example.com/sources geometry modified simplified".to_string(),
            None,
        )];
        let max_width = 40.0;
        let (size, lines) = fit_row(&row, 8.0, 4.0, max_width, AttributionOverflow::Wrap);
        assert_eq!(size, 4.0);
        assert!(lines.len() > 1, "expected multiple wrapped lines");
        for line in &lines {
            assert_eq!(line.len(), 1);
            assert!(text_width(&line[0].0, size) <= max_width + 1e-6);
        }
    }

    /// Records every `draw_text`/`draw_text_runs` call's text, position and
    /// size, with a configurable `plot_area` — enough to check an
    /// attribution line's bbox against a narrow panel.
    struct RecordingBackend {
        area: crate::render::Rect,
        calls: Vec<(String, (f64, f64), f64, TextAnchor)>,
    }

    impl RecordingBackend {
        fn new(area: crate::render::Rect) -> Self {
            RecordingBackend {
                area,
                calls: Vec::new(),
            }
        }
    }

    impl DrawBackend for RecordingBackend {
        fn draw_circle(
            &mut self,
            _c: (f64, f64),
            _r: f64,
            _s: &crate::render::backend::PointStyle,
        ) -> Result<(), RenderError> {
            Ok(())
        }
        fn draw_line(&mut self, _p: &[(f64, f64)], _s: &LineStyle) -> Result<(), RenderError> {
            Ok(())
        }
        fn draw_rect(
            &mut self,
            _tl: (f64, f64),
            _br: (f64, f64),
            _s: &crate::render::backend::RectStyle,
        ) -> Result<(), RenderError> {
            Ok(())
        }
        fn draw_text(
            &mut self,
            text: &str,
            pos: (f64, f64),
            style: &TextStyle,
        ) -> Result<(), RenderError> {
            self.calls
                .push((text.to_string(), pos, style.size, style.anchor.clone()));
            Ok(())
        }
        fn draw_polygon(
            &mut self,
            _p: &[(f64, f64)],
            _s: &crate::render::backend::RectStyle,
        ) -> Result<(), RenderError> {
            Ok(())
        }
        fn plot_area(&self) -> crate::render::Rect {
            self.area.clone()
        }
        fn total_area(&self) -> crate::render::Rect {
            self.area.clone()
        }
    }

    fn anchor_fraction(anchor: TextAnchor) -> f64 {
        match anchor {
            TextAnchor::Start => 0.0,
            TextAnchor::Middle => 0.5,
            TextAnchor::End => 1.0,
        }
    }

    #[test]
    fn fit_keeps_every_line_inside_a_narrow_panel_with_a_right_legend() {
        // A 390px-wide page minus a ~90px Right legend, same shape as
        // DataZooDE/anofox-visualization#505/#258.
        let area = crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 300.0,
        };
        let mut backend = RecordingBackend::new(area.clone());
        let g = GeomAttribution::new(
            "https://sg.geodatenzentrum.de/web_public/gdz/lizenz · geometry modified (simplified)",
        )
        .fit(4.0);
        let scales = ScaleSet::new();
        let coord = CoordCartesian::default();
        g.draw(
            &DataFrame::new(),
            &coord,
            &scales,
            &Theme::default(),
            &mut backend,
        )
        .unwrap();
        assert_eq!(backend.calls.len(), 1);
        let (text, pos, size, anchor) = &backend.calls[0];
        let w = text_width(text, *size);
        let frac = anchor_fraction(anchor.clone());
        let x0 = pos.0 - w * frac;
        let x1 = x0 + w;
        assert!(
            x0 >= area.x - 1e-6 && x1 <= area.x + area.width + 1e-6,
            "line overflowed the panel: x0={x0} x1={x1} area=[{},{}]",
            area.x,
            area.x + area.width
        );
    }

    #[test]
    fn fit_off_by_default_leaves_a_wide_panel_unchanged() {
        let area = crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        };
        let mut backend = RecordingBackend::new(area.clone());
        let g = GeomAttribution::new("a short note"); // no `.fit(..)` call
        let scales = ScaleSet::new();
        let coord = CoordCartesian::default();
        g.draw(
            &DataFrame::new(),
            &coord,
            &scales,
            &Theme::default(),
            &mut backend,
        )
        .unwrap();
        assert_eq!(backend.calls.len(), 1);
        assert_eq!(backend.calls[0].2, g.text_size); // size unchanged
    }

    #[test]
    fn fit_off_by_default_also_leaves_a_narrow_panel_unchanged() {
        // Byte-identical-by-default (#258): without `.fit(..)`, even a
        // panel too narrow for the line draws at the nominal size, same
        // as before this feature existed.
        let area = crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 300.0,
        };
        let mut backend = RecordingBackend::new(area.clone());
        let g = GeomAttribution::new(
            "https://sg.geodatenzentrum.de/web_public/gdz/lizenz · geometry modified (simplified)",
        );
        let scales = ScaleSet::new();
        let coord = CoordCartesian::default();
        g.draw(
            &DataFrame::new(),
            &coord,
            &scales,
            &Theme::default(),
            &mut backend,
        )
        .unwrap();
        assert_eq!(backend.calls[0].2, g.text_size);
    }
}
