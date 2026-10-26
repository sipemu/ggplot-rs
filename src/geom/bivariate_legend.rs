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
use crate::render::text::text_width;
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

/// The geometry shared by [`GeomBivariateLegend`]'s own `draw` and
/// [`GeomBivariateLegendCounts`]'s per-cell count overlay (#259) —
/// factored out so the two stay in lockstep rather than each recomputing
/// the swatch grid's position independently.
#[derive(Clone, Copy)]
struct LegendLayout {
    box_x: f64,
    box_y: f64,
    total_w: f64,
    total_h: f64,
    grid_x: f64,
    grid_y: f64,
    grid_w: f64,
    grid_h: f64,
    y_caption_w: f64,
    x_caption_h: f64,
    /// Extra vertical gap between the grid's bottom edge and the x
    /// caption's own position, beyond the usual `x_caption_h/2` half-step
    /// — see [`fn@legend_layout`]'s `y_caption_overflow` (#273).
    x_caption_gap: f64,
}

fn legend_layout(legend: &GeomBivariateLegend, plot_area: &crate::render::Rect) -> LegendLayout {
    let (nx, ny) = (legend.nx.max(1), legend.ny.max(1));
    let grid_w = nx as f64 * legend.cell_size;
    let grid_h = ny as f64 * legend.cell_size;
    // Room for the y caption (rotated, to the left) and the x caption
    // (below), plus a little padding all round.
    let y_caption_w = if legend.y_name.is_empty() {
        0.0
    } else {
        legend.text_size + 4.0
    };
    let x_caption_h = if legend.x_name.is_empty() {
        0.0
    } else {
        legend.text_size + 4.0
    };
    // #273: the rotated y caption's own text extent (its *length*, which
    // becomes its vertical span once rotated 90°) — previously only its
    // *thickness* (`y_caption_w`, the font's height) sized the box, so a
    // caption longer than the grid's own height (`grid_h`) drew past the
    // box's top/bottom edge, uncentred and clipped (e.g. placed
    // `TopRight`, clipped off the top). The caption is always drawn
    // centred on the grid's vertical centre (`grid_y + grid_h / 2.0`), so
    // fitting it needs `y_caption_overflow` of extra room split evenly
    // above the grid (`grid_y` shifts down by half of it) and below the
    // grid (`x_caption_gap`, consumed before the x caption's own
    // position) — zero, and this is byte-identical to before #273, for a
    // caption no longer than `grid_h`, the overwhelmingly common case.
    let y_caption_len = if legend.y_name.is_empty() {
        0.0
    } else {
        text_width(&legend.y_name, legend.text_size)
    };
    let y_caption_overflow = (y_caption_len - grid_h).max(0.0);
    let pad = 6.0;
    let total_w = y_caption_w + grid_w + pad * 2.0;
    let total_h = grid_h + y_caption_overflow + x_caption_h + pad * 2.0;

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
    let x_caption_gap = y_caption_overflow / 2.0;
    LegendLayout {
        box_x,
        box_y,
        total_w,
        total_h,
        grid_x: box_x + y_caption_w,
        grid_y: box_y + x_caption_gap,
        grid_w,
        grid_h,
        y_caption_w,
        x_caption_h,
        x_caption_gap,
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
        let layout = *layout;

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
        if !self.x_name.is_empty() {
            backend.draw_text(
                &self.x_name,
                (
                    layout.grid_x + layout.grid_w / 2.0,
                    layout.grid_y + layout.grid_h + layout.x_caption_gap + layout.x_caption_h / 2.0,
                ),
                &TextStyle {
                    color: self.text_color,
                    size: self.text_size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                    family: font.clone(),
                    face: crate::render::backend::FontFace::Plain,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
        if !self.y_name.is_empty() {
            backend.draw_text(
                &self.y_name,
                (
                    layout.box_x + layout.y_caption_w / 2.0,
                    layout.grid_y + layout.grid_h / 2.0,
                ),
                &TextStyle {
                    color: self.text_color,
                    size: self.text_size,
                    anchor: TextAnchor::Middle,
                    angle: 90.0,
                    family: font,
                    face: crate::render::backend::FontFace::Plain,
                    bg_colour: None,
                    bg_r: 0.0,
                },
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
        // overflow path — same total_h/grid_y/x_caption_gap as before.
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
        assert_eq!(layout.x_caption_gap, 0.0);
        assert_eq!(layout.grid_y, layout.box_y);
        assert_eq!(layout.total_h, layout.grid_h + 6.0 * 2.0); // no x_name here
    }

    #[test]
    fn y_caption_longer_than_the_grid_grows_the_box_to_fit_it() {
        // #273's repro: a rotated y caption longer than the grid's own
        // height used to draw past the box's top/bottom edge — clipped —
        // because only the caption's *thickness* sized the box. The box
        // must now be tall enough that the caption (always centred on
        // the grid's vertical centre) fits entirely inside it.
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
        let caption_centre = layout.grid_y + layout.grid_h / 2.0;
        let caption_top = caption_centre - caption_len / 2.0;
        let caption_bottom = caption_centre + caption_len / 2.0;
        assert!(
            caption_top >= layout.box_y - 1e-6,
            "caption_top={caption_top} box_y={}",
            layout.box_y
        );
        assert!(
            caption_bottom <= layout.box_y + layout.total_h + 1e-6,
            "caption_bottom={caption_bottom} box bottom={}",
            layout.box_y + layout.total_h
        );
    }

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
