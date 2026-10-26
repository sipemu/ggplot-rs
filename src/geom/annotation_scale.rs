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
use crate::render::text::{hard_break_cut, text_width};
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
    /// Multiplier on every width [`Self::fit`] estimates for this
    /// attribution — see [`Self::fit_width_scale`]. Default `1.0`: no
    /// effect, byte-identical to before this field existed (#267).
    pub fit_width_scale: f64,
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
            fit_width_scale: 1.0,
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

    /// Calibrate [`Self::fit`]'s width estimate for a caller's own deployed
    /// font (#267): `render::text::text_width`'s per-character heuristic is
    /// tuned as a general-purpose layout estimate, not pixel-perfect
    /// metrics for any one font, and this crate has no way to know what
    /// font a downstream SVG consumer's browser actually renders the
    /// attribution text in (weight, `letter-spacing`, a fallback font, …).
    /// A value above `1.0` makes [`Self::fit`] treat every line as wider
    /// than the heuristic says by that factor — shrinking, truncating or
    /// wrapping earlier — which is the right knob to reach for once a
    /// caller has *measured* their own rendering overflowing despite
    /// [`Self::fit`], rather than this crate guessing a bigger built-in
    /// margin that may still be wrong for that caller's font and either
    /// over- or under-shoots everyone else's. Default `1.0` (no effect,
    /// byte-identical to before this method existed). Clamped to a sane
    /// positive minimum; has no effect unless `.fit(..)` was called.
    pub fn fit_width_scale(mut self, scale: f64) -> Self {
        self.fit_width_scale = if scale.is_finite() {
            scale.max(0.01)
        } else {
            1.0
        };
        self
    }
}

/// Shrinks `row`'s text size toward `min_size` (clamped to `base_size`) so
/// its concatenated text fits `max_width` px, then truncates (ellipsis) or
/// wraps once shrinking alone isn't enough — see [`GeomAttribution::fit`]
/// (#258). Returns the text size to draw at, and the physical lines to
/// draw: almost always one line (`row` unchanged) — more than one only
/// when `overflow` is [`AttributionOverflow::Wrap`] and the line still
/// doesn't fit at `min_size`. `width_scale` multiplies every width used in
/// the fit decision (see [`GeomAttribution::fit_width_scale`]); `1.0`
/// keeps this byte-identical to before that hook existed (#267).
///
/// Truncating/wrapping a row that still overflows at `min_size` used to
/// flatten it to plain text, dropping its per-run link structure entirely
/// — a row that's one bare linked URL became unclickable the moment it
/// needed to shrink *and* still didn't fit (#267). [`wrap_row`] and
/// [`truncate_row_to_width`] instead keep whichever run's href covers
/// each kept (or split) piece of text.
fn fit_row(
    row: &[AttributionRun],
    base_size: f64,
    min_size: f64,
    max_width: f64,
    overflow: AttributionOverflow,
    width_scale: f64,
) -> (f64, Vec<AttributionRow>) {
    let min_size = min_size.min(base_size).max(0.1);
    // Scaling every estimated width up by `width_scale` before comparing
    // against `max_width` is equivalent to — and cheaper than — scaling
    // every `text_width` call site down by it, so apply it once here as an
    // effective (smaller) budget instead.
    let max_width = if width_scale.is_finite() && width_scale > 0.0 {
        max_width / width_scale
    } else {
        max_width
    };
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
        AttributionOverflow::Ellipsis => (
            min_size,
            vec![truncate_row_to_width(row, max_width, min_size)],
        ),
        AttributionOverflow::Wrap => {
            let mut lines = wrap_row(row, max_width, min_size);
            if lines.len() > ATTRIBUTION_MAX_WRAPPED_LINES {
                lines.truncate(ATTRIBUTION_MAX_WRAPPED_LINES);
                if let Some(last) = lines.last_mut() {
                    *last = force_ellipsis_row(last, max_width, min_size);
                }
            }
            (min_size, lines)
        }
    }
}

/// Merges adjacent runs that share the same href (`None` href included) in
/// place, dropping empty-text runs — the common tail of every helper below
/// that reassembles an [`AttributionRow`] out of pieces that may have come
/// from the same original run (so should read, and link, as one run again)
/// or from two different runs that merely happen to carry the same href.
fn merge_adjacent_same_href(runs: Vec<AttributionRun>) -> AttributionRow {
    let mut out: AttributionRow = Vec::new();
    for (text, href) in runs {
        if text.is_empty() {
            continue;
        }
        if let Some(last) = out.last_mut() {
            if last.1 == href {
                last.0.push_str(&text);
                continue;
            }
        }
        out.push((text, href));
    }
    out
}

/// Splits `chars` (with `owners[i]` naming which run in `row` each
/// `chars[i]` came from) into maximal same-owner runs, resolving each to
/// that run's href.
fn owner_runs(chars: &[char], owners: &[usize], row: &[AttributionRun]) -> Vec<AttributionRun> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let owner = owners[i];
        let mut j = i + 1;
        while j < chars.len() && owners[j] == owner {
            j += 1;
        }
        out.push((chars[i..j].iter().collect(), row[owner].1.clone()));
        i = j;
    }
    out
}

/// Like [`crate::render::text::truncate_to_width`], but run-aware (#267): keeps whichever run's
/// href covers the kept prefix instead of flattening `row` to a plain
/// string, so a shortened-but-still-pointing-to-the-right-place link stays
/// clickable. Returns `row` itself (byte-identical, including its run
/// structure) when it already fits — same "leave it alone" early-out as
/// [`crate::render::text::truncate_to_width`].
fn truncate_row_to_width(row: &[AttributionRun], max_width: f64, size: f64) -> AttributionRow {
    let concat: String = row.iter().map(|(t, _)| t.as_str()).collect();
    if text_width(&concat, size) <= max_width {
        return row.to_vec();
    }
    force_ellipsis_row(row, max_width, size)
}

/// Like [`truncate_row_to_width`], but always appends "…" even when `row`
/// already fits `max_width` on its own (#264/#267: the last line kept
/// after [`ATTRIBUTION_MAX_WRAPPED_LINES`] caps a wrap already fits — it
/// came from [`wrap_row`] — so `truncate_row_to_width`'s "already fits,
/// leave it alone" early-out would silently drop the remaining lines
/// without any visible sign that the line was cut short). The href that
/// ends up covering the trailing "…" is whichever run the last kept
/// character belongs to (or none, if nothing at all fits).
fn force_ellipsis_row(row: &[AttributionRun], max_width: f64, size: f64) -> AttributionRow {
    let mut chars: Vec<char> = Vec::new();
    let mut owners: Vec<usize> = Vec::new();
    for (i, (text, _)) in row.iter().enumerate() {
        for c in text.chars() {
            chars.push(c);
            owners.push(i);
        }
    }
    for keep in (0..=chars.len()).rev() {
        let candidate: String = chars[..keep].iter().collect::<String>() + "…";
        if text_width(&candidate, size) <= max_width {
            let mut merged =
                merge_adjacent_same_href(owner_runs(&chars[..keep], &owners[..keep], row));
            match merged.last_mut() {
                Some(last) => last.0.push('…'),
                None => merged.push(("…".to_string(), None)),
            }
            return merged;
        }
    }
    vec![("…".to_string(), None)]
}

/// [`AttributionOverflow::Wrap`]'s run-aware line-breaking (#267): the same
/// word-wrap and hard-break rules as [`crate::render::text::wrap_text`] (break at whitespace;
/// hard-split an overlong word, preferring a `/ . _ - ? & =` separator),
/// but applied across `row`'s runs instead of one flattened string, so
/// each emitted word — including a hard-split piece of an overlong run,
/// such as a bare URL — keeps the href of the run it came from. A word
/// that itself straddles two runs (not a shape the `.runs()`/`.line()`
/// builders are meant to produce, but the API doesn't forbid it) is cut at
/// the run boundary as well as any width-driven boundary, so a href is
/// never applied to text that wasn't actually part of that run.
fn wrap_row(row: &[AttributionRun], max_width: f64, size: f64) -> Vec<AttributionRow> {
    let mut chars: Vec<char> = Vec::new();
    let mut owners: Vec<usize> = Vec::new();
    for (i, (text, _)) in row.iter().enumerate() {
        for c in text.chars() {
            chars.push(c);
            owners.push(i);
        }
    }
    // Tokenize into (chars, owners) "words" at whitespace boundaries, same
    // granularity as `str::split_whitespace`.
    struct Token {
        chars: Vec<char>,
        owners: Vec<usize>,
    }
    let mut tokens: Vec<Token> = Vec::new();
    let mut cur_chars: Vec<char> = Vec::new();
    let mut cur_owners: Vec<usize> = Vec::new();
    for (c, o) in chars.into_iter().zip(owners) {
        if c.is_whitespace() {
            if !cur_chars.is_empty() {
                tokens.push(Token {
                    chars: std::mem::take(&mut cur_chars),
                    owners: std::mem::take(&mut cur_owners),
                });
            }
        } else {
            cur_chars.push(c);
            cur_owners.push(o);
        }
    }
    if !cur_chars.is_empty() {
        tokens.push(Token {
            chars: cur_chars,
            owners: cur_owners,
        });
    }

    let mut lines: Vec<AttributionRow> = Vec::new();
    // The physical line currently being built, as text (for width checks,
    // mirroring `wrap_text`'s `cur: String`) and as pieces (to reassemble
    // into runs once the line is complete).
    let mut cur_text = String::new();
    let mut cur_pieces: Vec<AttributionRun> = Vec::new();

    fn flush_cur(
        cur_text: &mut String,
        cur_pieces: &mut Vec<AttributionRun>,
        lines: &mut Vec<AttributionRow>,
    ) {
        if !cur_pieces.is_empty() {
            lines.push(merge_adjacent_same_href(std::mem::take(cur_pieces)));
        }
        cur_text.clear();
    }

    for token in tokens {
        let mut word_chars = token.chars;
        let mut word_owners = token.owners;
        // A single word that alone overflows max_width: hard-break it,
        // same as `wrap_text`, each forced piece becoming its own
        // complete physical line.
        while text_width(&word_chars.iter().collect::<String>(), size) > max_width
            && word_chars.len() > 1
        {
            let cut = hard_break_cut(&word_chars, max_width, size);
            flush_cur(&mut cur_text, &mut cur_pieces, &mut lines);
            lines.push(merge_adjacent_same_href(owner_runs(
                &word_chars[..cut],
                &word_owners[..cut],
                row,
            )));
            word_chars = word_chars[cut..].to_vec();
            word_owners = word_owners[cut..].to_vec();
        }
        let word_str: String = word_chars.iter().collect();
        if cur_text.is_empty() {
            cur_text = word_str;
            cur_pieces = owner_runs(&word_chars, &word_owners, row);
        } else {
            let candidate = format!("{cur_text} {word_str}");
            if text_width(&candidate, size) <= max_width {
                cur_pieces.push((" ".to_string(), None));
                cur_pieces.extend(owner_runs(&word_chars, &word_owners, row));
                cur_text = candidate;
            } else {
                flush_cur(&mut cur_text, &mut cur_pieces, &mut lines);
                cur_text = word_str;
                cur_pieces = owner_runs(&word_chars, &word_owners, row);
            }
        }
    }
    if !cur_pieces.is_empty() || lines.is_empty() {
        flush_cur(&mut cur_text, &mut cur_pieces, &mut lines);
    }
    lines
}

/// Safety margin applied to [`GeomAttribution::fit`]'s width budget (#264):
/// `text_width`'s per-character heuristic (`render::text`'s em table) is
/// tuned for layout decisions generally, not pixel-perfect metrics for one
/// particular font — a character mix heavy on digits, `/`/`.`/`_` and
/// mixed-case prose (a URL plus a trailing note, e.g. #264's repro) can
/// render a few percent wider in the actual font than the estimate says.
/// Shrinking the budget by this fraction before fitting keeps a line that
/// "just" fits by the estimate from actually overflowing the panel once
/// real glyphs are drawn, without needing the shared, crate-wide
/// `text_width` table itself recalibrated per caller.
///
/// This is deliberately *not* raised further for #267: measuring the exact
/// repro string against both of this crate's own real, non-synthetic fonts
/// — the bundled DejaVu Sans (`render::pixel_backend`'s canvas backend) and
/// a fetched Hanken Grotesk (anofox-visualization's own UI font; see the
/// `fit_against_a_real_font_*` tests) — finds the real glyph-advance sum
/// consistently *narrower* than this heuristic's estimate for that text,
/// in both fonts, not wider; this margin already has headroom to spare
/// against either. A caller whose own deployed rendering (a heavier
/// font-weight, extra `letter-spacing`, a fallback font, …) measurably
/// needs more should reach for [`GeomAttribution::fit_width_scale`]
/// instead of this crate guessing a bigger margin for a case it can't
/// reproduce.
const ATTRIBUTION_FIT_SAFETY_MARGIN: f64 = 0.92;

/// Once [`AttributionOverflow::Wrap`] still doesn't fit a logical row at
/// the fit floor, the most physical lines it may expand into (#264: an
/// unbounded wrap on a bare URL — no whitespace to break the old
/// char-by-char fallback — produced a 13-line ladder that ran up into the
/// map). Any remainder beyond this count is dropped, with the last kept
/// line ellipsised to show truncation happened.
const ATTRIBUTION_MAX_WRAPPED_LINES: usize = 3;

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
                // Panel width minus the 4px corner inset on each side, then
                // a safety margin against the width estimate's slack
                // against real font metrics (#264).
                let max_width = ((plot_area.width - 8.0) * ATTRIBUTION_FIT_SAFETY_MARGIN).max(0.0);
                logical_rows
                    .flat_map(|row| {
                        let (size, lines) = fit_row(
                            &row,
                            self.text_size,
                            min_size,
                            max_width,
                            self.fit_overflow,
                            self.fit_width_scale,
                        );
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

    /// `text` with every whitespace character removed — for comparing a
    /// [`wrap_row`]/[`wrap_text`]-wrapped block back against its
    /// unwrapped source: a line break at a word boundary legitimately
    /// consumes the space it broke on (same reasoning as `wrap_text`'s own
    /// tests comparing against `lines.join(" ")`, not `lines.concat()`),
    /// so a byte-for-byte reconstruction isn't the right invariant once
    /// more than one physical line is involved (#267).
    fn strip_ws(text: &str) -> String {
        text.chars().filter(|c| !c.is_whitespace()).collect()
    }

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
        let (size, lines) = fit_row(&row, 8.0, 4.0, 1000.0, AttributionOverflow::Ellipsis, 1.0);
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
        let (size, lines) = fit_row(
            &row,
            8.0,
            4.0,
            max_width,
            AttributionOverflow::Ellipsis,
            1.0,
        );
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
        let (size, lines) = fit_row(
            &row,
            8.0,
            4.0,
            max_width,
            AttributionOverflow::Ellipsis,
            1.0,
        );
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
        let (size, lines) = fit_row(&row, 8.0, 4.0, max_width, AttributionOverflow::Wrap, 1.0);
        assert_eq!(size, 4.0);
        assert!(lines.len() > 1, "expected multiple wrapped lines");
        for line in &lines {
            assert_eq!(line.len(), 1);
            assert!(text_width(&line[0].0, size) <= max_width + 1e-6);
        }
    }

    #[test]
    fn fit_row_wrap_caps_the_line_count_and_ellipsises_the_last() {
        // No whitespace at all (#264's repro shape): the old char-by-char
        // fallback would otherwise ladder on indefinitely.
        let row = vec![(
            "https://sgx.geodatenzentrum.de/web_public/gdz/datenquellen/datenquellen_vg_nuts.pdf"
                .to_string(),
            None,
        )];
        let max_width = 15.0; // narrow enough to need far more than 3 lines uncapped
        let (size, lines) = fit_row(&row, 8.0, 4.0, max_width, AttributionOverflow::Wrap, 1.0);
        assert_eq!(size, 4.0);
        assert_eq!(lines.len(), ATTRIBUTION_MAX_WRAPPED_LINES);
        for line in &lines {
            assert_eq!(line.len(), 1);
            assert!(
                text_width(&line[0].0, size) <= max_width + 1e-6,
                "line {:?} overflows",
                line[0].0
            );
        }
        assert!(
            lines.last().unwrap()[0].0.ends_with('…'),
            "last kept line should be ellipsised: {:?}",
            lines.last().unwrap()[0].0
        );
    }

    #[test]
    fn fit_row_wrap_under_the_cap_is_untouched() {
        // Fewer physical lines than the cap: no truncation, no ellipsis.
        let row = vec![(
            "https://example.com/sources geometry modified simplified".to_string(),
            None,
        )];
        let max_width = 40.0;
        let (_, lines) = fit_row(&row, 8.0, 4.0, max_width, AttributionOverflow::Wrap, 1.0);
        assert!(lines.len() <= ATTRIBUTION_MAX_WRAPPED_LINES);
        assert!(!lines.last().unwrap()[0].0.ends_with('…'));
    }

    // --- #267: Wrap splits *within* a run, and both Wrap and Ellipsis keep
    // each kept/split piece's href instead of flattening the whole row to
    // plain text the moment shrinking alone isn't enough. ---

    #[test]
    fn wrap_row_splits_a_linked_run_keeping_its_href_on_every_resulting_line() {
        // The bare-URL half of #267's BKG repro, alone: narrow enough that
        // even just this one linked run needs several physical lines once
        // hard-split.
        let href =
            "https://sgx.geodatenzentrum.de/web_public/gdz/datenquellen/datenquellen_vg_nuts.pdf";
        let row = vec![(href.to_string(), Some(href.to_string()))];
        let lines = wrap_row(&row, 60.0, 6.0);
        assert!(
            lines.len() > 1,
            "expected the URL to split across lines, got {lines:?}"
        );
        let mut rebuilt = String::new();
        for line in &lines {
            assert_eq!(
                line.len(),
                1,
                "single-run input should stay a single run per line: {line:?}"
            );
            assert_eq!(
                line[0].1.as_deref(),
                Some(href),
                "a split piece lost its href: {line:?}"
            );
            rebuilt.push_str(&line[0].0);
        }
        assert_eq!(rebuilt, href, "splitting must not lose or duplicate text");
    }

    #[test]
    fn wrap_row_keeps_a_too_wide_linked_run_separate_from_a_later_plain_run() {
        // #267's full BKG repro shape: a bare linked URL run, then a plain
        // (unlinked) note run.
        let href =
            "https://sgx.geodatenzentrum.de/web_public/gdz/datenquellen/datenquellen_vg_nuts.pdf";
        let note = " · geometry modified (simplified)";
        let row = vec![
            (href.to_string(), Some(href.to_string())),
            (note.to_string(), None),
        ];
        let lines = wrap_row(&row, 60.0, 6.0);
        assert!(lines.len() > 1, "expected wrapping, got {lines:?}");
        let rebuilt: String = lines.iter().flatten().map(|(t, _)| t.as_str()).collect();
        // A line break at a word boundary consumes the space it broke on
        // (same as `wrap_text`'s own `lines.join(" ")`, not
        // `lines.concat()`, in its tests) — compare with whitespace
        // stripped out of both sides rather than demanding the exact
        // separator back.
        assert_eq!(strip_ws(&rebuilt), strip_ws(&format!("{href}{note}")));
        // Every linked piece, concatenated in order, is a prefix of the
        // URL run alone — the note's text never ends up carrying the
        // URL's href, and the URL's href is never dropped partway through.
        let linked: String = lines
            .iter()
            .flatten()
            .filter(|(_, h)| h.is_some())
            .map(|(t, _)| t.as_str())
            .collect();
        assert!(!linked.is_empty());
        assert!(
            href.starts_with(&linked),
            "linked text {linked:?} is not a prefix of the url run {href:?}"
        );
        // And the note itself did make it into an unlinked run somewhere.
        assert!(lines.iter().flatten().any(|(_, h)| h.is_none()));
    }

    #[test]
    fn fit_row_wrap_preserves_run_boundaries_across_widths_and_floors() {
        // Property check (not just the one hand-picked width above): for
        // a grid of panel widths and floor sizes, Wrap must never degrade
        // to #267's old behaviour of dropping every run's href the moment
        // shrinking alone isn't enough.
        let url =
            "https://sgx.geodatenzentrum.de/web_public/gdz/datenquellen/datenquellen_vg_nuts.pdf";
        let href = url;
        let note = " · geometry modified (simplified)";
        let row = vec![
            (url.to_string(), Some(href.to_string())),
            (note.to_string(), None),
        ];
        let concat = format!("{url}{note}");
        for max_width in [15.0, 40.0, 80.0, 150.0, 200.0, 307.0, 1000.0] {
            for &(base, floor) in &[(7.0, 4.0), (7.0, 7.0), (8.0, 6.0)] {
                for overflow in [AttributionOverflow::Ellipsis, AttributionOverflow::Wrap] {
                    let (_, lines) = fit_row(&row, base, floor, max_width, overflow, 1.0);
                    let out: String = lines.iter().flatten().map(|(t, _)| t.as_str()).collect();
                    // Whitespace-insensitive, same reasoning as
                    // `wrap_row_keeps_a_too_wide_linked_run_separate_from_a_later_plain_run`:
                    // a Wrap line break at a word boundary legitimately
                    // consumes the space it broke on.
                    if let Some(prefix) = out.strip_suffix('…') {
                        assert!(
                            strip_ws(&concat).starts_with(&strip_ws(prefix)),
                            "max_width={max_width} base={base} floor={floor} overflow={overflow:?}: \
                             {prefix:?} is not a prefix of {concat:?}"
                        );
                    } else {
                        assert_eq!(
                            strip_ws(&out), strip_ws(&concat),
                            "max_width={max_width} base={base} floor={floor} overflow={overflow:?}: \
                             lost or changed text without an ellipsis"
                        );
                    }
                    let linked: String = lines
                        .iter()
                        .flatten()
                        .filter(|(_, h)| h.is_some())
                        .map(|(t, _)| t.as_str())
                        .collect();
                    if out != "…" && !out.is_empty() {
                        assert!(
                            !linked.is_empty(),
                            "max_width={max_width} base={base} floor={floor} overflow={overflow:?}: \
                             output {out:?} kept none of the url run's href at all"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn truncate_row_to_width_keeps_a_fitting_row_byte_identical() {
        let row = vec![
            ("short".to_string(), Some("https://x.example".to_string())),
            (" note".to_string(), None),
        ];
        assert_eq!(truncate_row_to_width(&row, 1000.0, 10.0), row);
    }

    #[test]
    fn force_ellipsis_row_keeps_the_href_on_the_ellipsised_tail() {
        let href = "https://example.com/a/very/long/path/that/needs/truncating";
        let row = vec![(href.to_string(), Some(href.to_string()))];
        let out = force_ellipsis_row(&row, 40.0, 7.0);
        assert_eq!(out.len(), 1);
        assert!(out[0].0.ends_with('…'));
        assert_eq!(out[0].1.as_deref(), Some(href));
        assert!(text_width(&out[0].0, 7.0) <= 40.0 + 1e-6);
    }

    #[test]
    fn fit_row_ellipsis_preserves_the_href_on_the_truncated_prefix() {
        let href = "https://example.com/a/very/long/path/that/needs/truncating/at/the/floor/size";
        let row = vec![(href.to_string(), Some(href.to_string()))];
        let (size, lines) = fit_row(&row, 8.0, 4.0, 40.0, AttributionOverflow::Ellipsis, 1.0);
        assert_eq!(size, 4.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 1);
        assert!(lines[0][0].0.ends_with('…'));
        assert_eq!(
            lines[0][0].1.as_deref(),
            Some(href),
            "the truncated prefix should still link to the full, untruncated href"
        );
    }

    // --- #267: `fit_width_scale` — an opt-in calibration hook for a
    // caller's own deployed font/rendering, since this crate can't measure
    // it itself. ---

    #[test]
    fn fit_width_scale_default_is_one() {
        assert_eq!(GeomAttribution::new("x").fit_width_scale, 1.0);
    }

    #[test]
    fn fit_width_scale_builder_clamps_non_positive_and_non_finite() {
        assert_eq!(
            GeomAttribution::new("x")
                .fit_width_scale(0.0)
                .fit_width_scale,
            0.01
        );
        assert_eq!(
            GeomAttribution::new("x")
                .fit_width_scale(-5.0)
                .fit_width_scale,
            0.01
        );
        assert_eq!(
            GeomAttribution::new("x")
                .fit_width_scale(f64::NAN)
                .fit_width_scale,
            1.0
        );
        assert_eq!(
            GeomAttribution::new("x")
                .fit_width_scale(2.5)
                .fit_width_scale,
            2.5
        );
    }

    #[test]
    fn fit_row_width_scale_one_matches_the_no_scale_behaviour() {
        let row = vec![("a moderately long attribution line".to_string(), None)];
        let full_width = text_width(&row[0].0, 8.0);
        let max_width = full_width * 0.7;
        let a = fit_row(
            &row,
            8.0,
            4.0,
            max_width,
            AttributionOverflow::Ellipsis,
            1.0,
        );
        let b = fit_row(
            &row,
            8.0,
            4.0,
            max_width,
            AttributionOverflow::Ellipsis,
            1.000_000_1, // "no effective scaling" should also cover ~1.0
        );
        assert!(
            (a.0 - b.0).abs() < 1e-6,
            "a negligible scale delta shouldn't visibly change the fitted size: {} vs {}",
            a.0,
            b.0
        );
    }

    #[test]
    fn fit_row_width_scale_above_one_shrinks_the_effective_budget() {
        // A row that fits untouched at the base size with no scaling...
        let row = vec![("a moderately long attribution line".to_string(), None)];
        let full_width = text_width(&row[0].0, 8.0);
        let max_width = full_width * 1.05;
        let (size1, lines1) = fit_row(
            &row,
            8.0,
            4.0,
            max_width,
            AttributionOverflow::Ellipsis,
            1.0,
        );
        assert_eq!(size1, 8.0);
        assert_eq!(lines1, vec![row.clone()]);
        // ...but needs to shrink once a caller says their real font
        // renders meaningfully wider than the heuristic assumes.
        let (size2, lines2) = fit_row(
            &row,
            8.0,
            4.0,
            max_width,
            AttributionOverflow::Ellipsis,
            1.2,
        );
        assert!(size2 < 8.0, "size2 = {size2}");
        assert_eq!(lines2, vec![row]);
        assert!(text_width(&lines2[0][0].0, size2) <= max_width / 1.2 + 1e-6);
    }

    #[test]
    fn fit_width_scale_wires_through_the_full_draw_pipeline() {
        let area = crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        };
        let text = "a moderately long attribution line of text";
        let coord = CoordCartesian::default();
        let scales = ScaleSet::new();

        let mut backend1 = RecordingBackend::new(area.clone());
        GeomAttribution::new(text)
            .fit(4.0)
            .draw(
                &DataFrame::new(),
                &coord,
                &scales,
                &Theme::default(),
                &mut backend1,
            )
            .unwrap();

        let mut backend2 = RecordingBackend::new(area.clone());
        GeomAttribution::new(text)
            .fit(4.0)
            .fit_width_scale(3.0)
            .draw(
                &DataFrame::new(),
                &coord,
                &scales,
                &Theme::default(),
                &mut backend2,
            )
            .unwrap();

        assert_eq!(backend1.calls.len(), 1);
        assert_eq!(backend2.calls.len(), 1);
        assert!(
            backend2.calls[0].2 <= backend1.calls[0].2,
            "a larger fit_width_scale should never draw at a *bigger* size \
             ({} vs {})",
            backend2.calls[0].2,
            backend1.calls[0].2
        );
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

    #[test]
    fn fit_applies_a_safety_margin_below_the_raw_inset_budget() {
        // #264's repro shape: a right-anchored line that just barely fits
        // the estimate at the floor size should still have slack against
        // the *raw* inset budget (panel width minus the 4px corner inset
        // each side) — the whole point of the margin is that the estimate
        // alone isn't trusted to be exact.
        let area = crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 342.0,
            height: 300.0,
        };
        let mut backend = RecordingBackend::new(area.clone());
        let g = GeomAttribution::new(
            "https://sgx.geodatenzentrum.de/web_public/gdz/datenquellen/datenquellen_vg_nuts.pdf · geometry modified",
        )
        .fit(7.0);
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
        let (text, _, size, _) = &backend.calls[0];
        let raw_budget = area.width - 8.0;
        let w = text_width(text, *size);
        assert!(w <= raw_budget, "line {text:?} overflows raw budget");
        assert!(
            w <= raw_budget * ATTRIBUTION_FIT_SAFETY_MARGIN + 1e-6,
            "line {text:?} (w={w}) leaves no safety margin below the raw budget ({raw_budget})"
        );
    }

    #[test]
    fn fit_keeps_every_line_inside_the_panel_across_widths_and_sizes() {
        // Sweeps panel widths 100..=400px and several base/floor text
        // sizes — every emitted line's estimated width must stay within
        // the panel, with the safety margin's slack against the raw inset
        // budget, for both the ellipsis and wrap overflow modes (#264).
        let text = "https://sgx.geodatenzentrum.de/web_public/gdz/datenquellen/datenquellen_vg_nuts.pdf · geometry modified";
        let coord = CoordCartesian::default();
        let scales = ScaleSet::new();
        for width in (100..=400).step_by(25) {
            let area = crate::render::Rect {
                x: 0.0,
                y: 0.0,
                width: width as f64,
                height: 300.0,
            };
            let raw_budget = area.width - 8.0;
            for &(base_size, min_size) in &[(7.0, 7.0), (7.0, 4.0), (10.0, 5.0), (8.0, 8.0)] {
                for overflow in [AttributionOverflow::Ellipsis, AttributionOverflow::Wrap] {
                    let mut backend = RecordingBackend::new(area.clone());
                    let mut g = GeomAttribution::new(text).fit(min_size);
                    g.text_size = base_size;
                    g = g.fit_overflow(overflow);
                    g.draw(
                        &DataFrame::new(),
                        &coord,
                        &scales,
                        &Theme::default(),
                        &mut backend,
                    )
                    .unwrap();
                    for (line_text, pos, size, anchor) in &backend.calls {
                        let w = text_width(line_text, *size);
                        let frac = anchor_fraction(anchor.clone());
                        let x0 = pos.0 - w * frac;
                        let x1 = x0 + w;
                        assert!(
                            x0 >= area.x - 1e-6 && x1 <= area.x + area.width + 1e-6,
                            "width={width} base={base_size} min={min_size} overflow={overflow:?}: \
                             line {line_text:?} overflowed [x0={x0}, x1={x1}] vs panel [{}, {}]",
                            area.x,
                            area.x + area.width
                        );
                        assert!(
                            w <= raw_budget * ATTRIBUTION_FIT_SAFETY_MARGIN + 1e-6,
                            "width={width} base={base_size} min={min_size} overflow={overflow:?}: \
                             line {line_text:?} (w={w}) leaves no safety margin below raw budget {raw_budget}"
                        );
                    }
                }
            }
        }
    }

    // #267: the panel-width/text-size sweep above only checks
    // `text_width`'s own heuristic estimate against itself — it can't
    // catch the estimate being wrong relative to an actual font. These
    // repeat it measuring with real glyph advances from two genuine,
    // non-synthetic fonts instead: the DejaVu Sans this crate already
    // bundles for its own canvas/plotters text rendering, and a fetched
    // Hanken Grotesk — anofox-visualization's own UI/chart font, and the
    // font #267's real production overflow was observed in. `ab_glyph` is
    // a plain dev-dependency here (not the optional `canvas`-feature one),
    // so this runs under plain `--features sf` same as every other
    // `GeomAttribution` test, without needing `canvas` too.
    mod real_font_267 {
        use super::*;
        use ab_glyph::{Font, FontRef, PxScale, ScaleFont};

        const DEJAVU: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");
        const HANKEN: &[u8] = include_bytes!("../../assets/fonts/test/HankenGrotesk-wght.ttf");

        /// Sums each character's real glyph advance (ab_glyph `h_advance`
        /// — the same metric `render::pixel_backend`'s canvas backend
        /// actually draws with). No kerning, same simplification
        /// `text_width`'s heuristic makes — but read from the font's own
        /// tables instead of a per-character average.
        fn real_width(font_bytes: &[u8], text: &str, size: f64) -> f64 {
            let font = FontRef::try_from_slice(font_bytes).expect("bundled test font parses");
            let scaled = font.as_scaled(PxScale::from(size as f32));
            text.chars()
                .map(|c| scaled.h_advance(font.glyph_id(c)) as f64)
                .sum()
        }

        fn real_font_sweep(font_bytes: &[u8]) {
            let text = "https://sgx.geodatenzentrum.de/web_public/gdz/datenquellen/datenquellen_vg_nuts.pdf · geometry modified";
            let coord = CoordCartesian::default();
            let scales = ScaleSet::new();
            for width in (100..=400).step_by(25) {
                let area = crate::render::Rect {
                    x: 0.0,
                    y: 0.0,
                    width: width as f64,
                    height: 300.0,
                };
                // Floors 6-8px, as #267's acceptance criteria asks.
                for &(base_size, min_size) in &[(7.0, 6.0), (7.0, 7.0), (8.0, 7.0), (8.0, 8.0)] {
                    for overflow in [AttributionOverflow::Ellipsis, AttributionOverflow::Wrap] {
                        let mut backend = RecordingBackend::new(area.clone());
                        let mut g = GeomAttribution::new(text).fit(min_size);
                        g.text_size = base_size;
                        g = g.fit_overflow(overflow);
                        g.draw(
                            &DataFrame::new(),
                            &coord,
                            &scales,
                            &Theme::default(),
                            &mut backend,
                        )
                        .unwrap();
                        for (line_text, pos, size, anchor) in &backend.calls {
                            let w = real_width(font_bytes, line_text, *size);
                            let frac = anchor_fraction(anchor.clone());
                            let x0 = pos.0 - w * frac;
                            let x1 = x0 + w;
                            assert!(
                                x0 >= area.x - 1e-6 && x1 <= area.x + area.width + 1e-6,
                                "width={width} base={base_size} min={min_size} overflow={overflow:?}: \
                                 real-glyph line {line_text:?} overflowed [x0={x0}, x1={x1}] vs panel [{}, {}]",
                                area.x,
                                area.x + area.width
                            );
                        }
                    }
                }
            }
        }

        #[test]
        fn fit_against_real_dejavu_glyph_metrics_never_overflows_the_panel() {
            real_font_sweep(DEJAVU);
        }

        #[test]
        fn fit_against_real_hanken_grotesk_glyph_metrics_never_overflows_the_panel() {
            real_font_sweep(HANKEN);
        }
    }
}
