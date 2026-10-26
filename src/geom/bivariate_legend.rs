//! `bivariate_legend()` (feature `sf`): the square `n × m` guide for
//! [`ScaleFillBivariate`](crate::scale::bivariate::ScaleFillBivariate), with
//! two axis captions — biscale's own legend shape. Drawn as an ordinary
//! furniture geom (no required aesthetic, no data column), the same way
//! [`crate::geom::annotation_scale`]'s scale bar/attribution are, rather
//! than through the standard per-aesthetic discrete-legend machinery (which
//! draws one row per break, not a 2-D grid) — set
//! `.legend_position(LegendPosition::None)` to hide the ordinary Fill
//! legend `ScaleFillBivariate` would otherwise also produce.

use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle, TextAnchor, TextStyle};
use crate::render::corner_layout::stack_clear;
use crate::render::text::{text_width, wrap_text};
use crate::render::{Rect, RenderError};
use crate::scale::bivariate::bivariate_cell_color;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Which panel corner the legend sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LegendCorner {
    BottomLeft,
    #[default]
    BottomRight,
    TopLeft,
    TopRight,
}

/// The square bivariate legend: an `nx × ny` grid of swatches plus two axis
/// captions. Mirror a [`ScaleFillBivariate`](crate::scale::bivariate::ScaleFillBivariate)'s
/// `nx`/`ny`/`base`/`color_x`/`color_y` here so the swatches match the data
/// exactly (there's no live link between the scale and this geom — they're
/// independent layers, like `annotation_scale` and `geom_sf`).
pub struct GeomBivariateLegend {
    pub nx: usize,
    pub ny: usize,
    pub base: (u8, u8, u8),
    pub color_x: (u8, u8, u8),
    pub color_y: (u8, u8, u8),
    pub x_name: String,
    pub y_name: String,
    pub corner: LegendCorner,
    pub cell_size: f64,
    pub text_size: f64,
    pub text_color: (u8, u8, u8),
}

impl GeomBivariateLegend {
    /// Build directly from a [`ScaleFillBivariate`](crate::scale::bivariate::ScaleFillBivariate),
    /// so the legend's swatches are guaranteed to match the scale's.
    pub fn from_scale(scale: &crate::scale::bivariate::ScaleFillBivariate) -> Self {
        GeomBivariateLegend {
            nx: scale.nx(),
            ny: scale.ny(),
            base: scale.base(),
            color_x: scale.color_x(),
            color_y: scale.color_y(),
            x_name: scale.x_name().to_string(),
            y_name: scale.y_name().to_string(),
            ..Self::default()
        }
    }

    pub fn corner(mut self, corner: LegendCorner) -> Self {
        self.corner = corner;
        self
    }

    pub fn cell_size(mut self, px: f64) -> Self {
        self.cell_size = px.max(1.0);
        self
    }

    /// Show each cell's row count, centred in its swatch (#259;
    /// DataZooDE/anofox-visualization#506's bivariate choropleths). Indexed
    /// like the `draw` loop's own `for j in 0..ny { for i in 0..nx }` —
    /// `counts[j][i]` is the count for column `i`, row `j`. Returns a
    /// separate [`GeomBivariateLegendCounts`] rather than a new field on
    /// this struct: `GeomBivariateLegend` is fully literal-constructible
    /// from outside the crate (no private fields, not
    /// `#[non_exhaustive]`), so adding a field here would be a breaking
    /// change (0.31.0's `GeomAttribution` precedent, #250). Add the
    /// result with [`crate::plot::GGPlot::bivariate_legend_counts`]
    /// instead of [`crate::plot::GGPlot::bivariate_legend`].
    /// [`Self::from_scale`] stays count-less.
    pub fn counts(self, counts: Vec<Vec<usize>>) -> GeomBivariateLegendCounts {
        GeomBivariateLegendCounts {
            legend: self,
            counts,
        }
    }
}

impl Default for GeomBivariateLegend {
    fn default() -> Self {
        GeomBivariateLegend {
            nx: 3,
            ny: 3,
            base: (255, 255, 255),
            color_x: (220, 90, 60),
            color_y: (60, 90, 220),
            x_name: String::new(),
            y_name: String::new(),
            corner: LegendCorner::default(),
            cell_size: 14.0,
            text_size: 8.0,
            text_color: (80, 80, 80),
        }
    }
}

/// A caption no longer than this many physical lines (#273): shrink
/// toward a floor if that alone fits, else wrap, else truncate — same
/// three-tier fallback `GeomAttribution::fit` uses, capped here rather
/// than left to grow the box, so neither caption's rendered footprint
/// ever forces the box bigger than roughly twice the grid itself.
const CAPTION_MAX_LINES: usize = 2;

/// Fits `text` within `budget` px at `base_size`: unchanged if it
/// already fits; shrunk toward a floor (`(base_size * 0.7).max(5.0)`,
/// clamped to `base_size`) if that alone fits; wrapped onto up to
/// [`CAPTION_MAX_LINES`] physical lines at the floor size; truncated
/// with a trailing "…" on the last line as a last resort (#273: a
/// `GeomBivariateLegend` caption must fit the grid's own width/height,
/// not grow the box to fit itself — see [`fn@legend_layout`]). Returns
/// `(size, lines, truncated)`; `lines` is empty for empty `text`.
/// `truncated` is only ever true when content was actually dropped (a
/// wrap alone keeps every character, just across more lines) — the
/// caller uses it to decide whether a `<title>` fallback with the full
/// text is worth adding.
fn fit_caption(text: &str, budget: f64, base_size: f64) -> (f64, Vec<String>, bool) {
    if text.is_empty() {
        return (base_size, Vec::new(), false);
    }
    if budget.is_nan() || budget <= 0.0 {
        return (base_size, vec![text.to_string()], false);
    }
    let full_w = text_width(text, base_size);
    if full_w <= budget {
        return (base_size, vec![text.to_string()], false);
    }
    let min_size = (base_size * 0.7).max(5.0).min(base_size);
    let shrunk = (base_size * budget / full_w).max(min_size);
    if text_width(text, shrunk) <= budget {
        return (shrunk, vec![text.to_string()], false);
    }
    let mut lines = wrap_text(text, budget, min_size);
    if lines.len() > CAPTION_MAX_LINES {
        lines.truncate(CAPTION_MAX_LINES);
        if let Some(last) = lines.last_mut() {
            *last = force_ellipsis(last, budget, min_size);
        }
        return (min_size, lines, true);
    }
    (min_size, lines, false)
}

/// Like [`crate::render::text::truncate_to_width`], but always appends
/// "…" even when `text` already fits `max_width` on its own — the last
/// of [`CAPTION_MAX_LINES`] wrapped lines came from [`wrap_text`], so it
/// already fits; `truncate_to_width`'s "already fits, leave it alone"
/// early-out would otherwise drop the remaining lines with no visible
/// sign of truncation (same reasoning as `geom::annotation_scale`'s own
/// `force_ellipsis_row`, #264/#273).
fn force_ellipsis(text: &str, max_width: f64, size: f64) -> String {
    let chars: Vec<char> = text.chars().collect();
    for keep in (0..=chars.len()).rev() {
        let candidate: String = chars[..keep].iter().collect::<String>() + "…";
        if text_width(&candidate, size) <= max_width {
            return candidate;
        }
    }
    "…".to_string()
}

/// The geometry shared by [`GeomBivariateLegend`]'s own `draw` and
/// [`GeomBivariateLegendCounts`]'s per-cell count overlay (#259) —
/// factored out so the two stay in lockstep rather than each recomputing
/// the swatch grid's position independently.
#[derive(Clone)]
struct LegendLayout {
    box_x: f64,
    box_y: f64,
    total_w: f64,
    total_h: f64,
    grid_x: f64,
    grid_y: f64,
    grid_w: f64,
    grid_h: f64,
    /// The x caption's own physical lines (#273: fit to `grid_w`, see
    /// [`fit_caption`]) — empty when `x_name` is empty.
    x_caption_lines: Vec<String>,
    x_caption_size: f64,
    /// Whether `x_caption_lines` dropped content (ellipsised) rather
    /// than just wrapping it — see [`fit_caption`]'s own docs.
    x_caption_truncated: bool,
    /// The y caption's own physical lines, each drawn as its own
    /// rotated column (#273: fit to `grid_h`) — empty when `y_name` is
    /// empty.
    y_caption_lines: Vec<String>,
    y_caption_size: f64,
    y_caption_truncated: bool,
}

/// One caption "slot" — the thickness a single physical line of a
/// caption (at `size`) reserves, matching the pre-#273 single-line
/// `text_size + 4.0` formula exactly, so a caption that still fits on
/// one line at its nominal size draws byte-identically to before.
fn caption_slot(size: f64) -> f64 {
    size + 4.0
}

fn legend_layout(legend: &GeomBivariateLegend, plot_area: &crate::render::Rect) -> LegendLayout {
    let (nx, ny) = (legend.nx.max(1), legend.ny.max(1));
    let grid_w = nx as f64 * legend.cell_size;
    let grid_h = ny as f64 * legend.cell_size;
    // #273: each caption fits *inside* the grid's own width/height — a
    // caption longer than that used to size the box from its thickness
    // only, drawing past the box's own edge (clipped), or (an earlier
    // version of this fix) grew the box to fit it, which could itself
    // grow tall/wide enough to crowd out every other corner item
    // regardless of how far they stacked. Fitting the caption instead
    // keeps the box tied to the grid, same as before #273.
    let (x_caption_size, x_caption_lines, x_caption_truncated) =
        fit_caption(&legend.x_name, grid_w, legend.text_size);
    let (y_caption_size, y_caption_lines, y_caption_truncated) =
        fit_caption(&legend.y_name, grid_h, legend.text_size);
    let x_caption_h = if x_caption_lines.is_empty() {
        0.0
    } else {
        x_caption_lines.len() as f64 * caption_slot(x_caption_size)
    };
    let y_caption_w = if y_caption_lines.is_empty() {
        0.0
    } else {
        y_caption_lines.len() as f64 * caption_slot(y_caption_size)
    };
    let pad = 6.0;
    let total_w = y_caption_w + grid_w + pad * 2.0;
    let total_h = grid_h + x_caption_h + pad * 2.0;

    let (box_x, box_y) = match legend.corner {
        LegendCorner::BottomLeft => (
            plot_area.x + pad,
            plot_area.y + plot_area.height - total_h - pad,
        ),
        LegendCorner::BottomRight => (
            plot_area.x + plot_area.width - total_w - pad,
            plot_area.y + plot_area.height - total_h - pad,
        ),
        LegendCorner::TopLeft => (plot_area.x + pad, plot_area.y + pad),
        LegendCorner::TopRight => (
            plot_area.x + plot_area.width - total_w - pad,
            plot_area.y + pad,
        ),
    };
    LegendLayout {
        box_x,
        box_y,
        total_w,
        total_h,
        grid_x: box_x + y_caption_w,
        grid_y: box_y,
        grid_w,
        grid_h,
        x_caption_lines,
        x_caption_size,
        x_caption_truncated,
        y_caption_lines,
        y_caption_size,
        y_caption_truncated,
    }
}

impl GeomBivariateLegend {
    /// This legend's box, shifted clear of anything already claimed in
    /// its corner so far this render pass (a scale bar/attribution drawn
    /// first, or an inside legend pre-claimed by `renderer.rs`), and
    /// registered via [`DrawBackend::claim_corner_box`] so a later corner
    /// geom sees it too (#273's shared corner layout).
    ///
    /// Shared by [`Geom::draw`] below and by
    /// [`GeomBivariateLegendCounts::draw`], which needs the *exact same*
    /// shifted layout for its count overlay to land inside the (possibly
    /// moved) swatches rather than recomputing — and re-claiming — its
    /// own: a second call here, after the first has already claimed this
    /// box, would see that claim as something to avoid and shift again.
    fn resolve_layout(
        &self,
        plot_area: &crate::render::Rect,
        backend: &mut dyn DrawBackend,
    ) -> LegendLayout {
        let natural = legend_layout(self, plot_area);
        let away_sign = match self.corner {
            LegendCorner::BottomLeft | LegendCorner::BottomRight => -1.0,
            LegendCorner::TopLeft | LegendCorner::TopRight => 1.0,
        };
        let natural_box = Rect {
            x: natural.box_x,
            y: natural.box_y,
            width: natural.total_w,
            height: natural.total_h,
        };
        let claims = backend.corner_claims();
        let shifted_box = stack_clear(natural_box.clone(), away_sign, &claims, plot_area);
        let dy = shifted_box.y - natural_box.y;
        backend.claim_corner_box(shifted_box);
        LegendLayout {
            box_y: natural.box_y + dy,
            grid_y: natural.grid_y + dy,
            ..natural
        }
    }

    /// Draws the grid and captions at an already-resolved `layout` — see
    /// [`Self::resolve_layout`].
    fn draw_with_layout(
        &self,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
        layout: &LegendLayout,
    ) -> Result<(), RenderError> {
        let (nx, ny) = (self.nx.max(1), self.ny.max(1));
        let layout = layout.clone();

        if theme.legend_background.visible {
            if let Some(fill) = theme.legend_background.fill {
                backend.draw_rect(
                    (layout.box_x, layout.box_y),
                    (layout.box_x + layout.total_w, layout.box_y + layout.total_h),
                    &RectStyle {
                        fill: Some(fill),
                        stroke: theme.legend_background.color,
                        stroke_width: theme.legend_background.width,
                        alpha: 1.0,
                        clip: false,
                        corner_radius: 0.0,
                    },
                )?;
            }
        }

        // Row 0 (j = 0, the low-y end) is drawn at the BOTTOM, matching
        // biscale's own legend orientation (y increases upward).
        for j in 0..ny {
            for i in 0..nx {
                let color =
                    bivariate_cell_color(self.base, self.color_x, self.color_y, nx, ny, i, j);
                let cx = layout.grid_x + i as f64 * self.cell_size;
                let cy = layout.grid_y + (ny - 1 - j) as f64 * self.cell_size;
                backend.draw_rect(
                    (cx, cy),
                    (cx + self.cell_size, cy + self.cell_size),
                    &RectStyle {
                        fill: Some(color),
                        stroke: None,
                        stroke_width: 0.0,
                        alpha: 1.0,
                        clip: false,
                        corner_radius: 0.0,
                    },
                )?;
            }
        }

        let font = super::resolve_font_family("", theme);
        // The x caption's own physical lines (#273: `fit_caption` already
        // wrapped/shrunk/truncated it to fit `grid_w`), stacked top to
        // bottom below the grid — one line reads byte-identically to
        // before #273 (same position, same size).
        let n_x = layout.x_caption_lines.len();
        for (i, line) in layout.x_caption_lines.iter().enumerate() {
            let slot = caption_slot(layout.x_caption_size);
            let ly = layout.grid_y + layout.grid_h + (i as f64 + 0.5) * slot;
            let title =
                (layout.x_caption_truncated && i + 1 == n_x).then_some(self.x_name.as_str());
            backend.draw_text_with_title(
                line,
                (layout.grid_x + layout.grid_w / 2.0, ly),
                &TextStyle {
                    color: self.text_color,
                    size: layout.x_caption_size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                    family: font.clone(),
                    face: crate::render::backend::FontFace::Plain,
                    bg_colour: None,
                    bg_r: 0.0,
                },
                title,
            )?;
        }
        // The y caption's own physical lines, each its own rotated
        // column to the left of the grid, reading the standard axis-title
        // way — bottom-to-top, letter-tops to the left (`angle: -90.0`;
        // the prior `90.0` read top-to-bottom, letter-tops right, which
        // this draw loop never exposed as a visible bug on its own
        // because a single, unbroken line of text is legible either way
        // — there's no second line to be mis-ordered relative to).
        // Once a caption wraps, the column stacking direction has to
        // agree with that per-column reading direction or the two
        // columns, read in the only non-mirrored page rotation, come out
        // swapped (same family as #273's reading-order bug): with
        // bottom-to-top/letter-tops-left text, the column nearest the
        // grid ends up read *second* once rotated upright, so line 0
        // (the caption's own start) goes in the *farthest* column, the
        // continuation nearest the grid.
        let n_y = layout.y_caption_lines.len();
        for (i, line) in layout.y_caption_lines.iter().enumerate() {
            let slot = caption_slot(layout.y_caption_size);
            let col = n_y - 1 - i; // 0 = nearest the grid, highest = farthest
            let lx = layout.grid_x - (col as f64 + 0.5) * slot;
            let ly = layout.grid_y + layout.grid_h / 2.0;
            let title =
                (layout.y_caption_truncated && i + 1 == n_y).then_some(self.y_name.as_str());
            backend.draw_text_with_title(
                line,
                (lx, ly),
                &TextStyle {
                    color: self.text_color,
                    size: layout.y_caption_size,
                    anchor: TextAnchor::Middle,
                    angle: -90.0,
                    family: font.clone(),
                    face: crate::render::backend::FontFace::Plain,
                    bg_colour: None,
                    bg_r: 0.0,
                },
                title,
            )?;
        }
        Ok(())
    }
}

impl Geom for GeomBivariateLegend {
    fn draw(
        &self,
        _data: &DataFrame,
        _coord: &dyn Coord,
        _scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let plot_area = backend.plot_area();
        let layout = self.resolve_layout(&plot_area, backend);
        self.draw_with_layout(theme, backend, &layout)
    }

    fn required_aes(&self) -> Vec<crate::aes::Aesthetic> {
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
        "bivariate_legend"
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.text_color == (80, 80, 80), el.ink) {
            self.text_color = ink;
        }
    }
}

/// Perceived-brightness (luma) threshold a cell's fill is compared
/// against to pick black-ish or white-ish count text — the standard
/// `0.299R + 0.587G + 0.114B` weighting (same one used to tell the nine
/// bivariate cells apart in [`crate::scale::bivariate`]'s own tests),
/// thresholded at the usual roughly-middle-grey cutoff.
fn contrast_text_color(fill: (u8, u8, u8)) -> (u8, u8, u8) {
    let (r, g, b) = fill;
    let luma = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
    if luma > 140.0 {
        (30, 30, 30)
    } else {
        (245, 245, 245)
    }
}

/// [`GeomBivariateLegend`] plus an optional per-cell row count, drawn
/// centred in each swatch (#259) — built with [`GeomBivariateLegend::counts`]
/// rather than a field on `GeomBivariateLegend` itself (see that method's
/// docs for why). Add it to a plot with
/// [`crate::plot::GGPlot::bivariate_legend_counts`].
///
/// `#[non_exhaustive]`: both fields are `pub` for now (there's no reason
/// yet to hide them), but marking it so from the start — rather than only
/// once a future field forces the issue, as `GeomAttribution` had to
/// (#250) — keeps a later addition here free.
#[non_exhaustive]
pub struct GeomBivariateLegendCounts {
    pub legend: GeomBivariateLegend,
    /// `counts[j][i]`: the row count for column `i`, row `j` — same
    /// indexing as [`GeomBivariateLegend::draw`]'s own
    /// `for j in 0..ny { for i in 0..nx }` loop. A cell with no entry (a
    /// short row or a short column) simply draws no count.
    pub counts: Vec<Vec<usize>>,
}

impl Geom for GeomBivariateLegendCounts {
    fn draw(
        &self,
        _data: &DataFrame,
        _coord: &dyn Coord,
        _scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let plot_area = backend.plot_area();
        let legend = &self.legend;
        // #273: resolve (and claim) the corner layout exactly once, then
        // draw the grid/captions and the count overlay from that same
        // layout — not `legend.draw(..)` followed by a second, unshifted
        // `legend_layout(..)` call here, which would draw the counts
        // against the *un*-stacked position while the swatches
        // themselves (drawn via the shifted layout) had moved.
        let layout = legend.resolve_layout(&plot_area, backend);
        legend.draw_with_layout(theme, backend, &layout)?;

        let (nx, ny) = (legend.nx.max(1), legend.ny.max(1));
        let font = super::resolve_font_family("", theme);
        // "About 7px, below the caption's text_size" (#259's default
        // text_size is 8.0) — always one px under whatever `text_size`
        // actually is, with a sane floor so a tiny custom `text_size`
        // never collapses the count to an unreadable/zero size.
        let count_size = (legend.text_size - 1.0).max(4.0);
        for (j, row) in self.counts.iter().enumerate().take(ny) {
            for (i, &count) in row.iter().enumerate().take(nx) {
                let fill =
                    bivariate_cell_color(legend.base, legend.color_x, legend.color_y, nx, ny, i, j);
                let cx = layout.grid_x + (i as f64 + 0.5) * legend.cell_size;
                let cy =
                    layout.grid_y + (ny - 1 - j) as f64 * legend.cell_size + legend.cell_size / 2.0;
                backend.draw_text(
                    &count.to_string(),
                    (cx, cy),
                    &TextStyle {
                        color: contrast_text_color(fill),
                        size: count_size,
                        anchor: TextAnchor::Middle,
                        angle: 0.0,
                        family: font.clone(),
                        face: crate::render::backend::FontFace::Plain,
                        bg_colour: None,
                        bg_r: 0.0,
                    },
                )?;
            }
        }
        Ok(())
    }

    fn required_aes(&self) -> Vec<crate::aes::Aesthetic> {
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
        "bivariate_legend"
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        self.legend.apply_element_geom(el);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aes::Aesthetic;
    use crate::data::Value;
    use crate::scale::bivariate::ScaleFillBivariate;

    #[test]
    fn from_scale_copies_grid_shape_and_colors() {
        let s = ScaleFillBivariate::tertile(
            Aesthetic::Fill,
            (217, 95, 2),
            (117, 112, 179),
            "Income",
            "Education",
        );
        let g = GeomBivariateLegend::from_scale(&s);
        assert_eq!(g.nx, 3);
        assert_eq!(g.ny, 3);
        assert_eq!(g.color_x, (217, 95, 2));
        assert_eq!(g.color_y, (117, 112, 179));
        assert_eq!(g.x_name, "Income");
        assert_eq!(g.y_name, "Education");
    }

    #[test]
    fn default_corner_is_bottom_right() {
        assert_eq!(LegendCorner::default(), LegendCorner::BottomRight);
    }

    #[test]
    fn cell_size_builder_sets_the_field() {
        let g = GeomBivariateLegend::default().cell_size(20.0);
        assert_eq!(g.cell_size, 20.0);
        // Clamped to a sane minimum rather than a zero/negative swatch.
        assert_eq!(
            GeomBivariateLegend::default().cell_size(-5.0).cell_size,
            1.0
        );
    }

    fn two_region_df() -> DataFrame {
        let mut df = DataFrame::new();
        df.add_column(
            "geometry".into(),
            vec![
                Value::Str("POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))".into()),
                Value::Str("POLYGON ((10 0, 20 0, 20 10, 10 10, 10 0))".into()),
            ],
        );
        df
    }

    #[test]
    fn empty_captions_draw_the_grid_without_axis_text() {
        let legend = GeomBivariateLegend {
            x_name: String::new(),
            y_name: String::new(),
            ..GeomBivariateLegend::default()
        };
        let svg = crate::plot::GGPlot::new(two_region_df())
            .aes(crate::aes::Aes::new())
            .bivariate_legend(legend)
            .render_svg_native()
            .expect("render");
        assert!(
            svg.contains("data-geom=\"bivariate_legend\""),
            "grid still draws: {svg}"
        );
        assert_eq!(svg.matches("<text").count(), 0, "no caption text: {svg}");
    }

    #[test]
    fn legend_background_draws_when_the_theme_enables_it() {
        let mut theme = crate::theme::Theme::default();
        theme.legend_background.visible = true;
        theme.legend_background.fill = Some((40, 40, 44));
        let legend = GeomBivariateLegend::from_scale(&ScaleFillBivariate::tertile(
            Aesthetic::Fill,
            (217, 95, 2),
            (117, 112, 179),
            "x",
            "y",
        ));
        let svg = crate::plot::GGPlot::new(two_region_df())
            .aes(crate::aes::Aes::new())
            .bivariate_legend(legend)
            .theme(theme)
            .render_svg_native()
            .expect("render");
        assert!(svg.contains("#28282C"), "{svg}");
    }

    #[test]
    fn ink_theme_hook_recolours_the_default_text_color() {
        let mut g = GeomBivariateLegend::default();
        let el = crate::theme::elements::ElementGeom {
            ink: Some((10, 20, 30)),
            ..Default::default()
        };
        g.apply_element_geom(&el);
        assert_eq!(g.text_color, (10, 20, 30));
        // An already-customised text_color is left alone.
        let mut g2 = GeomBivariateLegend {
            text_color: (1, 2, 3),
            ..GeomBivariateLegend::default()
        };
        g2.apply_element_geom(&el);
        assert_eq!(g2.text_color, (1, 2, 3));
    }

    #[test]
    fn contrast_text_color_picks_light_on_dark_and_dark_on_light() {
        assert_eq!(contrast_text_color((0, 0, 0)), (245, 245, 245));
        assert_eq!(contrast_text_color((255, 255, 255)), (30, 30, 30));
        // The scale's own darkest "high-high" swatch tone.
        assert_eq!(contrast_text_color((40, 40, 80)), (245, 245, 245));
    }

    #[test]
    fn counts_builder_moves_the_legend_and_stores_counts_indexed_by_j_then_i() {
        let g = GeomBivariateLegend::default().counts(vec![vec![1, 2, 3], vec![4, 5, 6]]);
        assert_eq!(g.legend.nx, 3);
        assert_eq!(g.counts, vec![vec![1, 2, 3], vec![4, 5, 6]]);
    }

    /// Records every `draw_rect`/`draw_text` call — enough to check that
    /// each cell's count lands centred in that cell's own rect, with a
    /// contrasting color (#259).
    #[derive(Default)]
    #[allow(clippy::type_complexity)] // test-only recorder of raw draw calls
    struct RecordingBackend {
        rects: Vec<((f64, f64), (f64, f64), Option<(u8, u8, u8)>)>,
        texts: Vec<(String, (f64, f64), f64, (u8, u8, u8))>,
        /// #273: real corner-claim tracking, for the stacking tests below.
        corner_claims: Vec<crate::render::Rect>,
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
        fn draw_line(
            &mut self,
            _p: &[(f64, f64)],
            _s: &crate::render::backend::LineStyle,
        ) -> Result<(), RenderError> {
            Ok(())
        }
        fn draw_rect(
            &mut self,
            tl: (f64, f64),
            br: (f64, f64),
            style: &RectStyle,
        ) -> Result<(), RenderError> {
            self.rects.push((tl, br, style.fill));
            Ok(())
        }
        fn draw_text(
            &mut self,
            text: &str,
            pos: (f64, f64),
            style: &TextStyle,
        ) -> Result<(), RenderError> {
            self.texts
                .push((text.to_string(), pos, style.size, style.color));
            Ok(())
        }
        fn draw_polygon(&mut self, _p: &[(f64, f64)], _s: &RectStyle) -> Result<(), RenderError> {
            Ok(())
        }
        fn plot_area(&self) -> crate::render::Rect {
            crate::render::Rect {
                x: 0.0,
                y: 0.0,
                width: 400.0,
                height: 300.0,
            }
        }
        fn total_area(&self) -> crate::render::Rect {
            self.plot_area()
        }
        fn corner_claims(&self) -> Vec<crate::render::Rect> {
            self.corner_claims.clone()
        }
        fn claim_corner_box(&mut self, rect: crate::render::Rect) {
            self.corner_claims.push(rect);
        }
        fn clear_corner_claims(&mut self) {
            self.corner_claims.clear();
        }
    }

    #[test]
    fn counts_are_drawn_centred_in_their_own_cell_with_a_contrasting_color() {
        // Blank captions here (unlike `from_scale`'s named ones) so
        // `backend.texts` holds only the 9 count labels under test.
        let g = GeomBivariateLegend {
            color_x: (217, 95, 2),
            color_y: (117, 112, 179),
            ..GeomBivariateLegend::default()
        }
        .counts(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
        let mut backend = RecordingBackend::default();
        let coord = crate::coord::cartesian::CoordCartesian::default();
        let scales = ScaleSet::new();
        // Blank the legend background panel so `backend.rects` holds only
        // the 9 swatches — the default theme's own white legend panel
        // would otherwise add a 10th rect ahead of them.
        let mut theme = Theme::default();
        theme.legend_background = crate::theme::elements::ElementRect::blank();
        g.draw(&DataFrame::new(), &coord, &scales, &theme, &mut backend)
            .unwrap();

        // 9 swatch rects (the legend's own grid) plus 9 count labels.
        assert_eq!(backend.rects.len(), 9);
        assert_eq!(backend.texts.len(), 9);

        for (text, pos, size, color) in &backend.texts {
            let n: usize = text.parse().expect("count text is a plain integer");
            // Find the swatch whose count this label is — same j/i order
            // `counts` was given in, matched against the rect it labels
            // by expected cell count: row j, col i -> value j*3 + i + 1.
            let j = (n - 1) / 3;
            let i = (n - 1) % 3;
            let (tl, br, fill) = backend.rects[j * 3 + i];
            let cx = (tl.0 + br.0) / 2.0;
            let cy = (tl.1 + br.1) / 2.0;
            assert!(
                (pos.0 - cx).abs() < 1e-6 && (pos.1 - cy).abs() < 1e-6,
                "count {n} at {pos:?}, expected cell centre ({cx}, {cy})"
            );
            assert_eq!(*color, contrast_text_color(fill.unwrap()));
            // "about 7px, below the caption's text_size" — default
            // text_size is 8.0.
            assert_eq!(*size, 7.0);
        }
    }

    #[test]
    fn counts_out_of_range_indices_are_skipped_not_panicking() {
        // Fewer rows/columns than nx/ny: the missing cells simply draw no
        // count rather than panicking.
        let g = GeomBivariateLegend::default().counts(vec![vec![1]]);
        let mut backend = RecordingBackend::default();
        let coord = crate::coord::cartesian::CoordCartesian::default();
        let scales = ScaleSet::new();
        g.draw(
            &DataFrame::new(),
            &coord,
            &scales,
            &Theme::default(),
            &mut backend,
        )
        .unwrap();
        assert_eq!(backend.texts.len(), 1);
        assert_eq!(backend.texts[0].0, "1");
    }

    #[test]
    fn no_counts_builder_call_draws_no_count_text() {
        // Byte-identical-by-default (#259): plain `GeomBivariateLegend`
        // (no `.counts(..)` call at all) never draws count text. Blank
        // captions so `backend.texts` isn't muddied by caption text.
        let g = GeomBivariateLegend {
            color_x: (217, 95, 2),
            color_y: (117, 112, 179),
            ..GeomBivariateLegend::default()
        };
        let mut backend = RecordingBackend::default();
        let coord = crate::coord::cartesian::CoordCartesian::default();
        let scales = ScaleSet::new();
        let mut theme = Theme::default();
        theme.legend_background = crate::theme::elements::ElementRect::blank();
        g.draw(&DataFrame::new(), &coord, &scales, &theme, &mut backend)
            .unwrap();
        assert_eq!(backend.rects.len(), 9);
        assert_eq!(backend.texts.len(), 0);
    }

    #[test]
    fn from_scale_stays_count_less() {
        // `from_scale` returns a plain `GeomBivariateLegend`, not
        // `GeomBivariateLegendCounts` — enforced by the type system, but
        // asserted here so a future change can't slip past review.
        let g: GeomBivariateLegend = GeomBivariateLegend::from_scale(&ScaleFillBivariate::tertile(
            Aesthetic::Fill,
            (217, 95, 2),
            (117, 112, 179),
            "x",
            "y",
        ));
        let _ = g; // just the type annotation matters
    }

    #[test]
    fn bivariate_legend_counts_renders_through_the_plot_builder() {
        let g = GeomBivariateLegend::from_scale(&ScaleFillBivariate::tertile(
            Aesthetic::Fill,
            (217, 95, 2),
            (117, 112, 179),
            "x",
            "y",
        ))
        .counts(vec![vec![3, 1, 0], vec![0, 2, 0], vec![0, 0, 5]]);
        let svg = crate::plot::GGPlot::new(two_region_df())
            .aes(crate::aes::Aes::new())
            .bivariate_legend_counts(g)
            .render_svg_native()
            .expect("render");
        assert!(svg.contains("data-geom=\"bivariate_legend\""), "{svg}");
        assert!(svg.contains(">3<"), "count 3 missing: {svg}");
        assert!(svg.contains(">5<"), "count 5 missing: {svg}");
    }

    // --- #273: the rotated y caption is sized by its full extent, and
    // the legend stacks clear of anything already claimed in its corner.

    #[test]
    fn short_y_caption_leaves_the_box_byte_identical_to_before_273() {
        // A caption no longer than the grid itself never triggers the
        // fit path (one line, at the full `text_size`) — same
        // total_h/grid_y as before #273.
        let legend = GeomBivariateLegend {
            y_name: "y".to_string(),
            ..GeomBivariateLegend::default()
        };
        let plot_area = crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 400.0,
        };
        let layout = legend_layout(&legend, &plot_area);
        assert_eq!(layout.y_caption_lines, vec!["y".to_string()]);
        assert_eq!(layout.y_caption_size, legend.text_size);
        assert!(!layout.y_caption_truncated);
        assert_eq!(layout.grid_y, layout.box_y);
        assert_eq!(layout.total_h, layout.grid_h + 6.0 * 2.0); // no x_name here
    }

    #[test]
    fn y_caption_longer_than_the_grid_is_fit_to_the_grid_height_not_the_box() {
        // #273's repro: a rotated y caption longer than the grid's own
        // height used to draw past the box's top/bottom edge (sized by
        // the caption's *thickness* only), then — an earlier fix pass —
        // grew the box tall enough to fit the caption's full extent,
        // which could itself grow large enough to crowd out every other
        // corner item regardless of how far up they'd stacked
        // (DataZooDE/anofox-visualization#549). The caption must instead
        // *fit* the grid's own height — wrap, then shrink to a floor,
        // then ellipsis — so the box stays tied to the grid, same as a
        // short caption.
        let legend = GeomBivariateLegend {
            nx: 2,
            ny: 2,
            cell_size: 10.0, // a small grid (20px tall) next to a long caption
            y_name: "Customers per 10,000 inhabitants".to_string(),
            x_name: String::new(),
            ..GeomBivariateLegend::default()
        };
        let plot_area = crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 400.0,
        };
        let layout = legend_layout(&legend, &plot_area);
        let caption_len = text_width(&legend.y_name, legend.text_size);
        assert!(
            caption_len > layout.grid_h,
            "test setup: caption ({caption_len}) should exceed the grid height ({})",
            layout.grid_h
        );
        // The box is sized from the grid plus at most CAPTION_MAX_LINES
        // caption slots — never from the caption's own raw length.
        assert!(layout.y_caption_lines.len() <= CAPTION_MAX_LINES);
        assert_eq!(
            layout.total_h,
            layout.grid_h + 6.0 * 2.0,
            "the y caption must never grow total_h — only total_w"
        );
        // Every emitted line actually fits inside the grid's own height.
        for line in &layout.y_caption_lines {
            let w = text_width(line, layout.y_caption_size);
            assert!(
                w <= layout.grid_h + 1e-6,
                "y caption line {line:?} (w={w}) overflows the grid height ({})",
                layout.grid_h
            );
        }
        // A caption this much longer than a 20px-tall grid can't survive
        // wrap+shrink alone — some content had to be dropped.
        assert!(layout.y_caption_truncated);
    }

    #[test]
    fn x_caption_longer_than_the_grid_is_fit_to_the_grid_width_not_the_box() {
        // Same fix, the other axis: a long x caption fits `grid_w`
        // rather than growing `total_w`.
        let legend = GeomBivariateLegend {
            nx: 2,
            ny: 2,
            cell_size: 10.0,
            x_name: "Road km to the nearest regional hospital".to_string(),
            y_name: String::new(),
            ..GeomBivariateLegend::default()
        };
        let plot_area = crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 400.0,
        };
        let layout = legend_layout(&legend, &plot_area);
        let caption_len = text_width(&legend.x_name, legend.text_size);
        assert!(
            caption_len > layout.grid_w,
            "test setup: caption ({caption_len}) should exceed the grid width ({})",
            layout.grid_w
        );
        assert!(layout.x_caption_lines.len() <= CAPTION_MAX_LINES);
        assert_eq!(
            layout.total_w,
            layout.grid_w + 6.0 * 2.0,
            "the x caption must never grow total_w — only total_h"
        );
        for line in &layout.x_caption_lines {
            let w = text_width(line, layout.x_caption_size);
            assert!(
                w <= layout.grid_w + 1e-6,
                "x caption line {line:?} (w={w}) overflows the grid width ({})",
                layout.grid_w
            );
        }
    }

    #[test]
    fn wrapped_y_caption_lines_stack_with_line_0_farthest_from_the_grid() {
        // The rotated y caption reads bottom-to-top, letter-tops-left
        // (`angle: -90.0`) — the standard axis-title orientation. Once it
        // wraps onto several columns, the column nearest the grid is the
        // one read *last* once a reader rotates the page upright to read
        // it (the only rotation that doesn't mirror the glyphs), so the
        // caption's own first physical line must sit in the *farthest*
        // column (smaller x) and each later line nearer the grid (larger
        // x) — same family as #273's reading-order bug, just along the
        // rotated axis instead of the vertical stack a horizontal
        // attribution row uses.
        let legend = GeomBivariateLegend {
            nx: 2,
            ny: 2,
            cell_size: 10.0,
            y_name: "Customers per 10,000 people".to_string(),
            x_name: String::new(),
            ..GeomBivariateLegend::default()
        };
        let mut backend = RecordingBackend::default();
        let coord = crate::coord::cartesian::CoordCartesian::default();
        let scales = ScaleSet::new();
        legend
            .draw(
                &DataFrame::new(),
                &coord,
                &scales,
                &Theme::default(),
                &mut backend,
            )
            .unwrap();

        let layout = legend_layout(
            &legend,
            &crate::render::Rect {
                x: 0.0,
                y: 0.0,
                width: 400.0,
                height: 400.0,
            },
        );
        assert!(
            layout.y_caption_lines.len() >= 2,
            "test setup: expected this caption to wrap onto at least 2 lines, got {:?}",
            layout.y_caption_lines
        );
        let x_of = |text: &str| {
            backend
                .texts
                .iter()
                .find(|(t, ..)| t == text)
                .unwrap_or_else(|| {
                    panic!("no draw_text call for line {text:?}: {:?}", backend.texts)
                })
                .1
                 .0
        };
        for pair in layout.y_caption_lines.windows(2) {
            let (line1, line2) = (&pair[0], &pair[1]);
            let (x1, x2) = (x_of(line1), x_of(line2));
            assert!(
                x1 < x2,
                "line {line1:?} (x={x1}) must sit farther from the grid (smaller x) than the \
                 following line {line2:?} (x={x2})"
            );
        }
    }

    /// Reconstructs every rect [`GeomBivariateLegend::draw_with_layout`]
    /// actually draws into from a resolved `layout` — the grid itself,
    /// each x-caption line's row, each y-caption line's rotated column —
    /// so self-intersection can be checked geometrically rather than
    /// trusting the layout numbers alone.
    fn legend_part_rects(layout: &LegendLayout) -> Vec<crate::render::Rect> {
        let mut rects = vec![crate::render::Rect {
            x: layout.grid_x,
            y: layout.grid_y,
            width: layout.grid_w,
            height: layout.grid_h,
        }];
        let n_x = layout.x_caption_lines.len();
        for (i, line) in layout.x_caption_lines.iter().enumerate() {
            let slot = caption_slot(layout.x_caption_size);
            let ly = layout.grid_y + layout.grid_h + (i as f64 + 0.5) * slot;
            let w = text_width(line, layout.x_caption_size);
            rects.push(crate::render::Rect {
                x: layout.grid_x + layout.grid_w / 2.0 - w / 2.0,
                y: ly - slot / 2.0,
                width: w,
                height: slot,
            });
            let _ = n_x;
        }
        let n_y = layout.y_caption_lines.len();
        for (i, line) in layout.y_caption_lines.iter().enumerate() {
            let slot = caption_slot(layout.y_caption_size);
            let col = n_y - 1 - i; // matches draw_with_layout's column order
            let lx = layout.grid_x - (col as f64 + 0.5) * slot;
            let h = text_width(line, layout.y_caption_size); // rotated 90°
            rects.push(crate::render::Rect {
                x: lx - slot / 2.0,
                y: layout.grid_y + layout.grid_h / 2.0 - h / 2.0,
                width: slot,
                height: h,
            });
            let _ = n_y;
        }
        rects
    }

    /// Like [`crate::render::corner_layout::rects_intersect`], but
    /// tolerant of exact, by-design zero-gap touching between two parts
    /// (e.g. two stacked lines of the same wrapped caption) — deflating
    /// each rect by a tiny epsilon first absorbs floating-point noise at
    /// a shared boundary (the two halves of the same `lx ± slot/2`
    /// computed via different arithmetic can land a `f64` ULP apart)
    /// without hiding a real overlap, which in this bug's repro is on
    /// the order of several pixels, not `1e-9`.
    fn really_intersects(a: &crate::render::Rect, b: &crate::render::Rect) -> bool {
        const EPS: f64 = 1e-6;
        let deflate = |r: &crate::render::Rect| crate::render::Rect {
            x: r.x + EPS,
            y: r.y + EPS,
            width: (r.width - 2.0 * EPS).max(0.0),
            height: (r.height - 2.0 * EPS).max(0.0),
        };
        crate::render::corner_layout::rects_intersect(&deflate(a), &deflate(b))
    }

    #[test]
    fn legend_is_self_intersection_free_over_caption_lengths_and_grid_sizes() {
        // #273: the bug was exactly two of these parts overlapping (the
        // rotated y caption running down across the x caption) — sweep
        // grid shapes and caption lengths and check every pair of drawn
        // parts stays disjoint (beyond the by-design zero-gap touch
        // between stacked lines of the same caption, see
        // `really_intersects`), and the whole thing stays inside the box
        // `legend_layout` claims for it.
        let captions = [
            "",
            "y",
            "Income",
            "Customers per 10,000 people",
            "Road km to the nearest regional hospital or clinic",
        ];
        let plot_area = crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 400.0,
        };
        for &(nx, ny, cell_size) in &[
            (2usize, 2usize, 8.0),
            (3, 3, 20.0),
            (4, 2, 14.0),
            (2, 5, 6.0),
        ] {
            for &x_name in &captions {
                for &y_name in &captions {
                    let legend = GeomBivariateLegend {
                        nx,
                        ny,
                        cell_size,
                        x_name: x_name.to_string(),
                        y_name: y_name.to_string(),
                        ..GeomBivariateLegend::default()
                    };
                    let layout = legend_layout(&legend, &plot_area);
                    let parts = legend_part_rects(&layout);
                    for (i, a) in parts.iter().enumerate() {
                        for b in &parts[i + 1..] {
                            assert!(
                                !really_intersects(a, b),
                                "nx={nx} ny={ny} cell_size={cell_size} x_name={x_name:?} \
                                 y_name={y_name:?}: parts intersect: {a:?} vs {b:?}"
                            );
                        }
                        let box_rect = crate::render::Rect {
                            x: layout.box_x,
                            y: layout.box_y,
                            width: layout.total_w,
                            height: layout.total_h,
                        };
                        assert!(
                            a.x >= box_rect.x - 1e-6
                                && a.y >= box_rect.y - 1e-6
                                && a.x + a.width <= box_rect.x + box_rect.width + 1e-6
                                && a.y + a.height <= box_rect.y + box_rect.height + 1e-6,
                            "nx={nx} ny={ny} cell_size={cell_size} x_name={x_name:?} \
                             y_name={y_name:?}: part {a:?} escapes claimed box {box_rect:?}"
                        );
                    }
                }
            }
        }
    }

    // Note: the full attribution + scale bar + bivariate legend corner
    // sweep (100..=400px, no intersections, every claim inside the panel)
    // is already covered by
    // `annotation_scale::tests::corner_sweep_attribution_scale_bar_and_bivariate_legend_never_overlap`
    // — extended below (see that test) to use the long, real-world
    // captions from DataZooDE/anofox-visualization#549's repro rather
    // than short placeholders, so it actually exercises this fix's
    // caption-fit path rather than only the pre-existing box-vs-box
    // stacking.

    #[test]
    fn bivariate_legend_stacks_clear_of_an_already_claimed_box_in_its_corner() {
        let mut backend = RecordingBackend::default();
        let plot_area = backend.plot_area();
        let claim = crate::render::Rect {
            x: plot_area.width - 80.0,
            y: plot_area.height - 60.0,
            width: 80.0,
            height: 40.0,
        };
        backend.claim_corner_box(claim.clone());

        let legend = GeomBivariateLegend::default(); // BottomRight, same as `claim`
        let coord = crate::coord::cartesian::CoordCartesian::default();
        let scales = ScaleSet::new();
        legend
            .draw(
                &DataFrame::new(),
                &coord,
                &scales,
                &Theme::default(),
                &mut backend,
            )
            .unwrap();

        assert_eq!(backend.corner_claims.len(), 2);
        assert!(!crate::render::corner_layout::rects_intersect(
            &backend.corner_claims[0],
            &backend.corner_claims[1]
        ));
    }

    #[test]
    fn counts_overlay_lands_on_the_same_stacked_layout_as_the_swatches() {
        // #273: `GeomBivariateLegendCounts` must position its count labels
        // against the *same* (possibly stacked/shifted) layout the
        // swatches themselves drew at — not recompute an unstacked one.
        let mut backend = RecordingBackend::default();
        let plot_area = backend.plot_area();
        let claim = crate::render::Rect {
            x: plot_area.width - 80.0,
            y: plot_area.height - 60.0,
            width: 80.0,
            height: 40.0,
        };
        backend.claim_corner_box(claim.clone());

        let g = GeomBivariateLegend {
            color_x: (217, 95, 2),
            color_y: (117, 112, 179),
            ..GeomBivariateLegend::default()
        }
        .counts(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
        let coord = crate::coord::cartesian::CoordCartesian::default();
        let scales = ScaleSet::new();
        let mut theme = Theme::default();
        theme.legend_background = crate::theme::elements::ElementRect::blank();
        g.draw(&DataFrame::new(), &coord, &scales, &theme, &mut backend)
            .unwrap();

        assert_eq!(backend.rects.len(), 9);
        assert_eq!(backend.texts.len(), 9);
        for (text, pos, ..) in &backend.texts {
            let n: usize = text.parse().unwrap();
            let j = (n - 1) / 3;
            let i = (n - 1) % 3;
            let (tl, br, _) = backend.rects[j * 3 + i];
            let cx = (tl.0 + br.0) / 2.0;
            let cy = (tl.1 + br.1) / 2.0;
            assert!(
                (pos.0 - cx).abs() < 1e-6 && (pos.1 - cy).abs() < 1e-6,
                "count {n} at {pos:?}, expected the (stacked) cell centre ({cx}, {cy})"
            );
        }
        // And the legend's own box (claimed second, after the
        // pre-existing `claim`) really did move clear of it.
        assert_eq!(backend.corner_claims.len(), 2);
        assert!(!crate::render::corner_layout::rects_intersect(
            &backend.corner_claims[0],
            &backend.corner_claims[1]
        ));
    }
}
