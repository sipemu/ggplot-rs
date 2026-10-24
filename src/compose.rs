//! Plot composition for the native SVG path — a patchwork-style [`PlotGrid`].
//!
//! ```
//! use ggplot_rs::prelude::*;
//! use ggplot_rs::data::Value;
//! let data = || vec![
//!     ("x".to_string(), (0..10).map(|i| Value::Float(i as f64)).collect::<Vec<_>>()),
//!     ("y".to_string(), (0..10).map(|i| Value::Float((i * i) as f64)).collect::<Vec<_>>()),
//! ];
//! let p = || GGPlot::new(data()).aes(Aes::new().x("x").y("y"));
//! // a | b puts plots side by side, a / b stacks them; `/` binds tighter.
//! let svg = ((p().geom_point() | p().geom_line()) / p().geom_col())
//!     .title("Diagnostics")
//!     .tag_levels(TagLevels::Lower)
//!     .render_svg_native_with_size(900, 700)
//!     .unwrap();
//! assert!(svg.contains("data-panel=\"a\"") && svg.contains("data-panel=\"c\""));
//! ```
//!
//! Every sub-plot is rendered as a positioned nested `<svg>` fragment (see
//! [`GGPlot::render_svg_native_at`]) that keeps all of its host attributes
//! (`data-plot`, `data-domain`, marks' `data-x`/`data-series`/`data-value`)
//! and additionally carries `data-panel="<tag or index>"`. A host maps a
//! pointer to data coordinates by first finding the enclosing sub-`<svg>`.

use std::ops::{BitOr, Div};

use crate::aes::Aesthetic;
use crate::guide::legend;
use crate::plot::{GGError, GGPlot, RenderMeta};
use crate::render::backend::{
    DrawBackend, LineStyle, PointStyle, RectStyle, TextAnchor, TextStyle,
};
use crate::render::renderer::PlotRenderer;
use crate::render::svg_backend::{root_data_attrs, validate_root_attrs, SvgBackend};
use crate::render::{Rect, RenderError};
use crate::theme::elements::{ElementLine, ElementText};
use crate::theme::{AxisLabelFit, LegendPosition, Theme};

/// Panel-tag sequence (patchwork's `tag_levels`).
#[derive(Clone, Debug, PartialEq)]
pub enum TagLevels {
    /// `a`, `b`, … `z`, `aa`, …
    Lower,
    /// `A`, `B`, …
    Upper,
    /// `1`, `2`, …
    Numeric,
    /// `i`, `ii`, `iii`, …
    LowerRoman,
    /// `I`, `II`, …
    UpperRoman,
    /// Explicit tags; panels beyond the list are numbered.
    Custom(Vec<String>),
}

impl TagLevels {
    /// The tag of the `i`-th (0-based) panel.
    pub fn tag(&self, i: usize) -> String {
        match self {
            TagLevels::Lower => alpha(i, b'a'),
            TagLevels::Upper => alpha(i, b'A'),
            TagLevels::Numeric => (i + 1).to_string(),
            TagLevels::LowerRoman => roman(i + 1).to_lowercase(),
            TagLevels::UpperRoman => roman(i + 1),
            TagLevels::Custom(v) => v.get(i).cloned().unwrap_or_else(|| (i + 1).to_string()),
        }
    }
}

fn alpha(mut i: usize, base: u8) -> String {
    // Bijective base-26: a..z, aa..az, ba..
    let mut out = Vec::new();
    loop {
        out.push(base + (i % 26) as u8);
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

fn roman(mut n: usize) -> String {
    if n >= 4000 {
        return n.to_string();
    }
    const T: [(usize, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut s = String::new();
    for (v, r) in T {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}

/// How a [`PlotGrid`] aligns its own direct cells' plot areas and x scale
/// within each column (patchwork's `plot_layout(axes = ...)`, applied to
/// the x axis and plot-area alignment specifically — sipemu/
/// ggplot-rs#175, tracked from DataZooDE/anofox-visualization#318). Only a
/// column of [`GridCell::Plot`] cells participates; a [`GridCell::Spacer`]
/// or a nested [`PlotGrid`] cell keeps its own layout, and a column with
/// fewer than two plot cells is a no-op. Set via [`PlotGrid::axis_collect`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AxisCollect {
    /// Each cell keeps its own margins, x axis and x domain (default).
    #[default]
    Keep,
    /// Equal left/right plot-area insets within each column, from the
    /// widest y-axis label requirement in that column — the same idea as
    /// anofox-visualization's `track.rs` `left = max(...)` fallback, done
    /// here as one layout pass (`GGPlot::layout_built`'s own arithmetic)
    /// rather than a render-measure-render loop. Every cell in the column
    /// also switches to [`AxisLabelFit::Shift`](crate::theme::AxisLabelFit::Shift)
    /// so an overflowing outer tick label can never perturb the alignment.
    /// A cell set thinner than the plotting engine's 50px panel-height
    /// floor via [`PlotGrid::heights`](Self::heights) (an
    /// anofox-visualization `::TRACK` strip, typically 16-30px) gets a
    /// centred `aspect_ratio` instead, so its panel comes out exactly as
    /// tall as the cell rather than overflowing it — `track.rs`'s own
    /// `Fit::correct` `aspect` trick, computed from the same layout pass.
    /// Domains and axis text are otherwise untouched.
    Align,
    /// [`Align`](Self::Align), plus one x domain shared across the column
    /// — the union of every cell's own trained x domain — with x-axis
    /// ticks/title/labels drawn only on the bottom cell. Skipped (with a
    /// warning), for a column whose x scale is discrete or untrained on
    /// any cell, in which case the column still gets [`Align`](Self::Align)'s
    /// plot-area alignment.
    CollectX,
}

/// Where a [`PlotGrid`] puts its collected legends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridLegendPosition {
    /// A column to the right of the grid (default).
    #[default]
    Right,
    /// A row below the grid.
    Bottom,
}

/// How a grid was built by the `|` / `/` operators, so chains flatten
/// (`a | b | c` is one row) while mixed operators nest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Row,
    Col,
}

/// One grid cell.
pub enum GridCell {
    Plot(Box<GGPlot>),
    Grid(Box<PlotGrid>),
    /// An empty cell (patchwork's `plot_spacer()`).
    Spacer,
}

impl From<GGPlot> for GridCell {
    fn from(p: GGPlot) -> Self {
        GridCell::Plot(Box::new(p))
    }
}

impl From<PlotGrid> for GridCell {
    fn from(g: PlotGrid) -> Self {
        GridCell::Grid(Box::new(g))
    }
}

/// A patchwork-style arrangement of plots rendered into **one** SVG on the
/// plotters-free native path.
///
/// - cells fill row-major (or column-major with [`byrow(false)`](Self::byrow))
///   across [`ncol`](Self::ncol) / [`nrow`](Self::nrow) (default: a near-square
///   grid; `a | b` is one row, `a / b` one column);
/// - [`widths`](Self::widths) / [`heights`](Self::heights) set relative
///   column widths / row heights;
/// - [`title`](Self::title) / [`subtitle`](Self::subtitle) /
///   [`caption`](Self::caption) annotate the whole grid;
/// - [`tag_levels`](Self::tag_levels) tags panels `a, b, …` (`A`, `1`, roman,
///   custom) in reading order, through nested grids;
/// - [`collect_legends`](Self::collect_legends) removes the sub-plots'
///   legends and draws each distinct legend once beside the grid (identical
///   legends are de-duplicated).
///
/// Nested grids keep their own layout and annotations; tagging and legend
/// collection are controlled by the outermost grid.
pub struct PlotGrid {
    cells: Vec<GridCell>,
    ncol: Option<usize>,
    nrow: Option<usize>,
    byrow: bool,
    widths: Vec<f64>,
    heights: Vec<f64>,
    title: Option<String>,
    subtitle: Option<String>,
    caption: Option<String>,
    tags: Option<TagLevels>,
    tag_prefix: String,
    tag_suffix: String,
    collect_legends: bool,
    legend_position: GridLegendPosition,
    spacing: f64,
    theme: Theme,
    axis_collect: AxisCollect,
    op: Option<Op>,
    /// Extra root `<svg>` attributes requested via [`PlotGrid::svg_attr`]/
    /// [`PlotGrid::svg_attrs`], validated and applied at render time.
    root_attrs: Vec<(String, String)>,
    /// Whether to emit a `data-warnings` JSON array of the panels'
    /// combined build warnings on the root `<svg>`
    /// ([`PlotGrid::svg_warnings_attr`]). Default off; byte-identical
    /// output when off.
    svg_warnings_attr: bool,
}

impl Default for PlotGrid {
    fn default() -> Self {
        PlotGrid {
            cells: Vec::new(),
            ncol: None,
            nrow: None,
            byrow: true,
            widths: Vec::new(),
            heights: Vec::new(),
            title: None,
            subtitle: None,
            caption: None,
            tags: None,
            tag_prefix: String::new(),
            tag_suffix: String::new(),
            collect_legends: false,
            legend_position: GridLegendPosition::Right,
            spacing: 0.0,
            theme: Theme::default(),
            axis_collect: AxisCollect::Keep,
            op: None,
            root_attrs: Vec::new(),
            svg_warnings_attr: false,
        }
    }
}

/// Upper bound on grid columns/rows — a guard against absurd user input.
const MAX_TRACKS: usize = 10_000;

impl PlotGrid {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a plot (or a nested grid) as the next cell.
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, cell: impl Into<GridCell>) -> Self {
        self.cells.push(cell.into());
        self
    }

    /// Append several plots.
    pub fn add_all(mut self, plots: impl IntoIterator<Item = GGPlot>) -> Self {
        self.cells.extend(plots.into_iter().map(GridCell::from));
        self
    }

    /// Append an empty cell (patchwork's `plot_spacer()`).
    pub fn add_spacer(mut self) -> Self {
        self.cells.push(GridCell::Spacer);
        self
    }

    /// Number of columns.
    pub fn ncol(mut self, n: usize) -> Self {
        self.ncol = (n > 0).then_some(n.min(MAX_TRACKS));
        self
    }

    /// Number of rows.
    pub fn nrow(mut self, n: usize) -> Self {
        self.nrow = (n > 0).then_some(n.min(MAX_TRACKS));
        self
    }

    /// Fill cells row by row (default) or column by column.
    pub fn byrow(mut self, byrow: bool) -> Self {
        self.byrow = byrow;
        self
    }

    /// Relative column widths (recycled; non-positive / non-finite = 1).
    pub fn widths(mut self, widths: &[f64]) -> Self {
        self.widths = widths.to_vec();
        self
    }

    /// Relative row heights (recycled; non-positive / non-finite = 1).
    pub fn heights(mut self, heights: &[f64]) -> Self {
        self.heights = heights.to_vec();
        self
    }

    /// Title over the whole grid.
    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Subtitle over the whole grid.
    pub fn subtitle(mut self, subtitle: &str) -> Self {
        self.subtitle = Some(subtitle.to_string());
        self
    }

    /// Caption under the whole grid.
    pub fn caption(mut self, caption: &str) -> Self {
        self.caption = Some(caption.to_string());
        self
    }

    /// Tag the panels (`labs(tag)` of each sub-plot is replaced).
    pub fn tag_levels(mut self, levels: TagLevels) -> Self {
        self.tags = Some(levels);
        self
    }

    /// Text around each tag, e.g. `("(", ")")` for `(a)`.
    pub fn tag_affixes(mut self, prefix: &str, suffix: &str) -> Self {
        self.tag_prefix = prefix.to_string();
        self.tag_suffix = suffix.to_string();
        self
    }

    /// Collect the sub-plots' legends into one strip (identical legends are
    /// drawn once). Sub-plots with an inside or hidden legend keep it.
    pub fn collect_legends(mut self, collect: bool) -> Self {
        self.collect_legends = collect;
        self
    }

    /// Where collected legends go.
    pub fn legend_position(mut self, pos: GridLegendPosition) -> Self {
        self.legend_position = pos;
        self
    }

    /// Align this grid's own direct cells' plot areas and/or x scale,
    /// one column at a time — see [`AxisCollect`] (sipemu/ggplot-rs#175).
    /// Default [`AxisCollect::Keep`] (no change, byte-identical output).
    pub fn axis_collect(mut self, mode: AxisCollect) -> Self {
        self.axis_collect = mode;
        self
    }

    /// Gap between cells, in px (default 0).
    pub fn spacing(mut self, px: f64) -> Self {
        self.spacing = if px.is_finite() { px.max(0.0) } else { 0.0 };
        self
    }

    /// Theme for the grid's own title/subtitle/caption and the collected
    /// legends' text.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Add one extra attribute to the root `<svg>` element — see
    /// [`GGPlot::svg_attr`](crate::plot::GGPlot::svg_attr), same allow-list
    /// (`data-*`, `aria-*`, `role`, `id`, `class`, `lang`) and the same
    /// "ignored with a warning" rule for anything else or a collision with
    /// what ggplot-rs already writes (`data-grid`, or `data-warnings` when
    /// [`svg_warnings_attr`](Self::svg_warnings_attr) is also on).
    pub fn svg_attr(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.root_attrs.push((name.into(), value.into()));
        self
    }

    /// [`svg_attr`](Self::svg_attr) for several attributes at once.
    pub fn svg_attrs(mut self, attrs: impl IntoIterator<Item = (String, String)>) -> Self {
        self.root_attrs.extend(attrs);
        self
    }

    /// Emit a `data-warnings` attribute on the root `<svg>` — a JSON array
    /// of every panel's combined build warnings, `[]` when there are none.
    /// Default off, and omitted entirely when off.
    pub fn svg_warnings_attr(mut self, on: bool) -> Self {
        self.svg_warnings_attr = on;
        self
    }

    /// Number of cells (plots, nested grids and spacers).
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether the grid has no cells.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Render at 800×600.
    pub fn render_svg_native(self) -> Result<String, GGError> {
        self.render_svg_native_with_size(800, 600)
    }

    /// Render the grid to one standalone SVG document of `w`×`h` px. The root
    /// carries `data-grid="<rows> <cols>"` (and no `data-plot`: each sub-`<svg>`
    /// has its own).
    pub fn render_svg_native_with_size(self, w: u32, h: u32) -> Result<String, GGError> {
        Ok(self.render_page(w, h)?.0.finish())
    }

    /// Like [`render_svg_native_with_size`](Self::render_svg_native_with_size),
    /// also returning every sub-plot's warnings, prefixed `panel <tag>: `.
    pub fn render_svg_native_with_warnings(
        self,
        w: u32,
        h: u32,
    ) -> Result<(String, Vec<String>), GGError> {
        let (page, warnings) = self.render_page(w, h)?;
        Ok((page.finish(), warnings))
    }

    /// Render as a nested `<svg>` fragment positioned at `(x, y)` — for
    /// composing the grid into a larger SVG (cf. [`GGPlot::render_svg_native_at`]).
    pub fn render_svg_native_at(self, x: f64, y: f64, w: u32, h: u32) -> Result<String, GGError> {
        Ok(self.render_page(w, h)?.0.finish_fragment(x, y))
    }

    fn render_page(self, w: u32, h: u32) -> Result<(SvgBackend, Vec<String>), GGError> {
        let (w, h) = (w.max(1), h.max(1));
        let mut theme = self.theme.clone();
        theme.resolve_inheritance();
        let collect = self.collect_legends;
        let legend_pos = self.legend_position;
        let root_attrs = self.root_attrs.clone();
        let svg_warnings_attr = self.svg_warnings_attr;

        // Phase 1 (size-independent): build every plot, assign tags and
        // capture legends to collect.
        let mut ctx = PrepCtx {
            tags: self
                .tags
                .clone()
                .map(|t| (t, self.tag_prefix.clone(), self.tag_suffix.clone())),
            next_tag: 0,
            next_index: 0,
            collect: collect.then_some(legend_pos),
            legends: Vec::new(),
            page_w: w as f64,
            page_h: h as f64,
            legend_warnings: Vec::new(),
        };
        let prepared = prepare_grid(self, &mut ctx)?;
        let legends = ctx.legends;
        let legend_warnings = ctx.legend_warnings;

        // Phase 2: page layout.
        let full = Rect {
            x: 0.0,
            y: 0.0,
            width: w as f64,
            height: h as f64,
        };
        let mut page = SvgBackend::new(w, h, full.clone());
        page.without_plot_attr();
        let (rows, cols) = prepared.dims();
        let data_grid_attr = ("data-grid".to_string(), format!("{rows} {cols}"));
        page.set_root_attrs(vec![data_grid_attr.clone()]);
        if let Some(fill) = theme
            .plot_background
            .fill
            .filter(|_| theme.plot_background.visible)
        {
            page.draw_rect(
                (0.0, 0.0),
                (w as f64, h as f64),
                &RectStyle {
                    fill: Some(fill),
                    stroke: None,
                    stroke_width: 0.0,
                    alpha: 1.0,
                    clip: false,
                    corner_radius: 0.0,
                },
            )
            .map_err(GGError::Render)?;
        }

        let mut body = full;
        if !legends.is_empty() {
            let gap = theme.legend_spacing.max(0.0) * 2.0;
            let pad = 8.0;
            match legend_pos {
                GridLegendPosition::Right => {
                    let strip_w = legends.iter().map(|l| l.w).fold(0.0, f64::max) + 2.0 * pad;
                    let strip_w = strip_w.min(body.width * 0.4);
                    body.width -= strip_w;
                    let total_h =
                        legends.iter().map(|l| l.h).sum::<f64>() + gap * (legends.len() - 1) as f64;
                    let mut y = (body.y + (body.height - total_h) / 2.0).max(body.y + pad);
                    for l in &legends {
                        place_legend(&mut page, l, body.x + body.width + pad, y);
                        y += l.h + gap;
                    }
                }
                GridLegendPosition::Bottom => {
                    let strip_h = legends.iter().map(|l| l.h).fold(0.0, f64::max) + 2.0 * pad;
                    let strip_h = strip_h.min(body.height * 0.4);
                    body.height -= strip_h;
                    let total_w =
                        legends.iter().map(|l| l.w).sum::<f64>() + gap * (legends.len() - 1) as f64;
                    let mut x = (body.x + (body.width - total_w) / 2.0).max(body.x + pad);
                    for l in &legends {
                        place_legend(&mut page, l, x, body.y + body.height + pad);
                        x += l.w + gap;
                    }
                }
            }
        }

        // Phase 3: render cells into their rects.
        let mut warnings = legend_warnings;
        render_prepared(prepared, &body, &mut page, &mut warnings)?;

        let mut reserved = vec![data_grid_attr];
        if svg_warnings_attr {
            reserved.push(("data-warnings".to_string(), String::new()));
        }
        let extra = crate::render::svg_backend::validate_root_attrs(
            &root_attrs,
            &reserved,
            "grid",
            &mut warnings,
        );
        page.extend_root_attrs(extra);
        if svg_warnings_attr {
            page.extend_root_attrs(vec![(
                "data-warnings".to_string(),
                crate::render::svg_backend::warnings_json(&warnings),
            )]);
        }
        Ok((page, warnings))
    }
}

/// A collected legend: rendered markup (also its identity) and extent.
struct Legend {
    body: String,
    x0: f64,
    y0: f64,
    w: f64,
    h: f64,
}

fn place_legend(page: &mut SvgBackend, l: &Legend, x: f64, y: f64) {
    let (tx, ty) = (x - l.x0, y - l.y0);
    if tx.is_finite() && ty.is_finite() {
        page.push_raw(&format!(
            "<g class=\"legend\" transform=\"translate({tx:.2} {ty:.2})\">{}</g>",
            l.body
        ));
    }
}

struct PrepCtx {
    tags: Option<(TagLevels, String, String)>,
    next_tag: usize,
    next_index: usize,
    collect: Option<GridLegendPosition>,
    legends: Vec<Legend>,
    /// The grid's overall page size (sipemu/ggplot-rs#155): a collected
    /// legend's available width/height for its own B3-2 wrap/grid plan,
    /// since a `PlotGrid`'s legend strip has no other natural bound —
    /// it's sized by whatever the legend needs, so the legend instead
    /// wraps to fit within the page itself.
    page_w: f64,
    page_h: f64,
    /// Warnings `capture_legend`'s `plan_all_legends` call produces for a
    /// collected legend (#170 / review #167 finding 6) — previously
    /// discarded; merged into `render_page`'s own `warnings` so they
    /// reach `render_svg_native_with_warnings` / `data-warnings` like
    /// every other warning.
    legend_warnings: Vec<String>,
}

/// A built sub-plot awaiting its cell size.
struct PreparedPlot {
    built: crate::build::BuiltPlot,
    meta: RenderMeta,
    panel: String,
    /// The cell's own `GGPlot::svg_formatted_values` setting (review
    /// PR#141 finding 6, LOW): `build_for_render` consumes the `GGPlot`
    /// and discards this field (it only matters at render time, and
    /// `PlotGrid` builds its cells independently of
    /// `GGPlot::render_native`, the only place that previously read it),
    /// so it has to be captured here and carried to the cell's own
    /// backend in `render_prepared`.
    svg_formatted_values: bool,
    /// The cell's own `GGPlot::svg_attr`/`svg_attrs` (#208): same reason
    /// as `svg_formatted_values` above — `build_for_render` discards
    /// `GGPlot::root_attrs`, so it's captured here and applied in
    /// `render_prepared`, on a `<g>` wrapping the cell's own content (the
    /// cell's root `<svg>` carries `root_data_attrs` and `data-panel`
    /// already; the grid's root carries the *grid's* own `svg_attr`s —
    /// neither is this cell's to share).
    root_attrs: Vec<(String, String)>,
}

enum Prepared {
    Plot(Box<PreparedPlot>),
    Grid(Box<PreparedGrid>),
    Spacer,
}

struct PreparedGrid {
    cells: Vec<Prepared>,
    ncol: usize,
    nrow: usize,
    byrow: bool,
    widths: Vec<f64>,
    heights: Vec<f64>,
    title: Option<String>,
    subtitle: Option<String>,
    caption: Option<String>,
    spacing: f64,
    theme: Theme,
    axis_collect: AxisCollect,
}

impl Prepared {
    fn dims(&self) -> (usize, usize) {
        match self {
            Prepared::Grid(g) => (g.nrow, g.ncol),
            _ => (1, 1),
        }
    }
}

fn prepare_grid(grid: PlotGrid, ctx: &mut PrepCtx) -> Result<Prepared, GGError> {
    let n = grid.cells.len();
    let (ncol, nrow) = match (grid.ncol, grid.nrow, grid.op) {
        (Some(c), Some(r), _) => {
            // Grow rows if the cells don't fit.
            (c, r.max(n.div_ceil(c)))
        }
        (Some(c), None, _) => (c, n.div_ceil(c).max(1)),
        (None, Some(r), _) => (n.div_ceil(r).max(1), r),
        (None, None, Some(Op::Row)) => (n.max(1), 1),
        (None, None, Some(Op::Col)) => (1, n.max(1)),
        (None, None, None) => {
            let c = ((n as f64).sqrt().ceil() as usize).max(1);
            (c, n.div_ceil(c).max(1))
        }
    };
    let mut theme = grid.theme;
    theme.resolve_inheritance();
    let mut cells = Vec::with_capacity(n);
    for cell in grid.cells {
        cells.push(match cell {
            GridCell::Spacer => Prepared::Spacer,
            GridCell::Grid(g) => prepare_grid(*g, ctx)?,
            GridCell::Plot(p) => Prepared::Plot(Box::new(prepare_plot(*p, ctx)?)),
        });
    }
    Ok(Prepared::Grid(Box::new(PreparedGrid {
        cells,
        ncol,
        nrow,
        byrow: grid.byrow,
        widths: grid.widths,
        heights: grid.heights,
        title: grid.title,
        subtitle: grid.subtitle,
        caption: grid.caption,
        spacing: grid.spacing,
        theme,
        axis_collect: grid.axis_collect,
    })))
}

fn prepare_plot(mut plot: GGPlot, ctx: &mut PrepCtx) -> Result<PreparedPlot, GGError> {
    ctx.next_index += 1;
    if let Some((levels, pre, suf)) = &ctx.tags {
        plot.labels.tag = Some(format!("{pre}{}{suf}", levels.tag(ctx.next_tag)));
        ctx.next_tag += 1;
    }
    let panel = plot
        .labels
        .tag
        .clone()
        .unwrap_or_else(|| ctx.next_index.to_string());

    // Collect an outside legend: suppress it in the plot, draw it separately.
    let original = plot.theme.legend_position.clone();
    let collect = ctx.collect.filter(|_| {
        matches!(
            original,
            LegendPosition::Right
                | LegendPosition::Left
                | LegendPosition::Top
                | LegendPosition::Bottom
        )
    });
    if collect.is_some() {
        plot.theme.legend_position = LegendPosition::None;
    }
    let svg_formatted_values = plot.svg_formatted_values;
    let root_attrs = plot.root_attrs.clone();
    let (built, mut meta) = plot.build_for_render()?;
    if let Some(pos) = collect {
        meta.has_legend = false;
        if let Some(l) = capture_legend(
            &built,
            pos,
            ctx.page_w,
            ctx.page_h,
            &mut ctx.legend_warnings,
        )
        .map_err(GGError::Render)?
        {
            if !ctx.legends.iter().any(|o| o.body == l.body) {
                ctx.legends.push(l);
            }
        }
    }
    Ok(PreparedPlot {
        built,
        meta,
        panel,
        svg_formatted_values,
        root_attrs,
    })
}

/// Draw a built plot's legend in isolation, measuring its extent.
///
/// `page_w`/`page_h` are the grid's overall page size (sipemu/
/// ggplot-rs#155): a collected legend has no natural width/height bound
/// of its own — the strip that holds it is sized by whatever the legend
/// needs — so a discrete guide wraps/grids here against the page itself,
/// the same way a single plot's legend wraps against its own canvas.
fn capture_legend(
    built: &crate::build::BuiltPlot,
    pos: GridLegendPosition,
    page_w: f64,
    page_h: f64,
    // Review #167 finding 6 (#170): the plan's own `guide_legend:
    // nrow(0)/ncol(0) is invalid`-style warnings used to go into a
    // `legend_warnings` local that nothing ever read — this path dropped
    // them silently. The caller threads its own warnings vec through so
    // they land in the grid's `render_svg_native_with_warnings` /
    // `data-warnings`, the same place a single plot's own legend
    // warnings go (`GGPlot::layout_built` → `built.warnings`).
    warnings: &mut Vec<String>,
) -> Result<Option<Legend>, RenderError> {
    let mut theme = built.theme.clone();
    let m = theme.legend_margin.clone();
    // Fake a zero-size panel so the legend's origin lands at (0, 0).
    let (pa, lp, is_horizontal) = match pos {
        GridLegendPosition::Right => (
            Rect {
                x: -m.left,
                y: -m.top,
                width: 0.0,
                height: 0.0,
            },
            LegendPosition::Right,
            false,
        ),
        GridLegendPosition::Bottom => (
            Rect {
                x: -m.left,
                y: -(m.top + 30.0),
                width: 0.0,
                height: 0.0,
            },
            LegendPosition::Bottom,
            true,
        ),
    };
    theme.legend_position = lp;
    let mut svg = SvgBackend::new(1, 1, pa.clone());
    let mut meas = Measure::new(&mut svg, pa.clone());
    let avail_width = (page_w + m.left).max(0.0);
    let avail_height = (page_h + m.top).max(0.0);
    let legend_plans = legend::plan_all_legends(
        &built.scales,
        &built.guide_legend,
        &theme,
        &built.suppressed_aes,
        avail_width,
        avail_height,
        is_horizontal,
        warnings,
    );
    legend::draw_legend_planned(
        &built.scales,
        &theme,
        &pa,
        // Review #167 finding 1: step by the same `avail_width` share
        // `plan_all_legends` just planned against, not `pa.width` (always
        // 0.0 — `pa` is a fake zero-size panel so the legend's origin
        // lands at (0, 0)). Using `pa.width` made every discrete guide
        // after the first advance by zero, so two collected guides (e.g.
        // `colour` and `shape`) were drawn on top of each other.
        avail_width,
        &mut meas,
        &built.guide_legend,
        &built.suppressed_aes,
        &legend_plans,
    )?;
    let ext = meas.extent();
    if svg.body().is_empty() {
        return Ok(None);
    }
    let (x0, y0, x1, y1) = match ext {
        Some(e) => e,
        None => return Ok(None),
    };
    Ok(Some(Legend {
        body: svg.body().to_string(),
        x0,
        y0,
        w: (x1 - x0).max(0.0),
        h: (y1 - y0).max(0.0),
    }))
}

/// Normalised track sizes: `n` relative weights (recycled, invalid → 1).
fn tracks(weights: &[f64], n: usize, total: f64, gap: f64) -> Vec<(f64, f64)> {
    let ws: Vec<f64> = (0..n)
        .map(|i| {
            if weights.is_empty() {
                1.0
            } else {
                let w = weights[i % weights.len()];
                if w.is_finite() && w > 0.0 {
                    w
                } else {
                    1.0
                }
            }
        })
        .collect();
    let sum: f64 = ws.iter().sum();
    let avail = (total - gap * n.saturating_sub(1) as f64).max(0.0);
    let mut pos = 0.0;
    ws.iter()
        .map(|w| {
            let size = avail * w / sum;
            let t = (pos, size);
            pos += size + gap;
            t
        })
        .collect()
}

fn text_style(el: &crate::theme::elements::ElementText, anchor: TextAnchor) -> TextStyle {
    TextStyle {
        color: el.color,
        size: el.size,
        anchor,
        angle: 0.0,
        family: (!el.family.is_empty()).then(|| el.family.clone()),
        face: el.face,
    }
}

fn hjust_x(el: &crate::theme::elements::ElementText, x0: f64, x1: f64) -> (f64, TextAnchor) {
    let hj = if el.hjust.is_finite() {
        el.hjust.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let anchor = if hj <= 0.02 {
        TextAnchor::Start
    } else if hj >= 0.98 {
        TextAnchor::End
    } else {
        TextAnchor::Middle
    };
    (x0 + hj * (x1 - x0), anchor)
}

fn render_prepared(
    prepared: Prepared,
    area: &Rect,
    page: &mut SvgBackend,
    warnings: &mut Vec<String>,
) -> Result<(), GGError> {
    match prepared {
        Prepared::Spacer => Ok(()),
        Prepared::Plot(p) => {
            let PreparedPlot {
                mut built,
                meta,
                panel,
                svg_formatted_values,
                root_attrs,
            } = *p;
            let (cw, ch) = (
                area.width.floor().max(1.0) as u32,
                area.height.floor().max(1.0) as u32,
            );
            let layout = GGPlot::layout_built(&mut built, &meta, cw, ch);
            let mut backend = SvgBackend::new(cw, ch, layout.plot_area.clone());
            backend.set_formatted_values(svg_formatted_values);
            let mut attrs = root_data_attrs(&built);
            attrs.push(("data-panel".into(), panel.clone()));
            backend.set_root_attrs(attrs);
            PlotRenderer::render(&built, &mut backend).map_err(GGError::Render)?;
            // #208: this cell's own `svg_attr`s go on a `<g>` wrapping its
            // content, not the cell's root `<svg>` (that one is
            // `root_data_attrs` + `data-panel`, ggplot-rs's own) nor the
            // grid's root (that one is the *grid's* own `svg_attr`s) — so
            // `existing` is empty: nothing built-in to collide with here,
            // only a name reused across two `svg_attr` calls.
            let mut attr_warnings = Vec::new();
            let group_attrs = validate_root_attrs(&root_attrs, &[], "plot", &mut attr_warnings);
            backend.wrap_body_in_group(&group_attrs);
            warnings.extend(
                built
                    .warnings
                    .iter()
                    .cloned()
                    .chain(backend.take_warnings())
                    .chain(attr_warnings)
                    .map(|w| format!("panel {panel}: {w}")),
            );
            page.push_raw(&backend.finish_fragment(area.x, area.y));
            Ok(())
        }
        Prepared::Grid(g) => {
            let mut g = *g;
            let theme = &g.theme;
            let m = &theme.plot_margin;
            // Header / footer bands.
            let mut top = area.y;
            let (x0, x1) = (area.x + m.left, area.x + area.width - m.right);
            if let Some(t) = &g.title {
                top += m.top;
                let (tx, anchor) = hjust_x(&theme.title, x0, x1);
                page.draw_text(
                    t,
                    (tx, top + theme.title.size * 0.8),
                    &text_style(&theme.title, anchor),
                )
                .map_err(GGError::Render)?;
                top += theme.title.size * 1.6;
            }
            if let Some(s) = &g.subtitle {
                if g.title.is_none() {
                    top += m.top;
                }
                let (sx, anchor) = hjust_x(&theme.subtitle, x0, x1);
                page.draw_text(
                    s,
                    (sx, top + theme.subtitle.size * 0.7),
                    &text_style(&theme.subtitle, anchor),
                )
                .map_err(GGError::Render)?;
                top += theme.subtitle.size * 1.5;
            }
            let mut bottom = area.y + area.height;
            if let Some(c) = &g.caption {
                let (cx, anchor) = hjust_x(&theme.caption, x0, x1);
                bottom -= theme.caption.size * 1.8;
                page.draw_text(
                    c,
                    (cx, bottom + theme.caption.size * 0.9),
                    &text_style(&theme.caption, anchor),
                )
                .map_err(GGError::Render)?;
            }
            let body_h = (bottom - top).max(1.0);
            let cols = tracks(&g.widths, g.ncol, area.width, g.spacing);
            let rows = tracks(&g.heights, g.nrow, body_h, g.spacing);
            if g.axis_collect != AxisCollect::Keep {
                align_axes(
                    &mut g.cells,
                    (g.ncol, g.nrow, g.byrow),
                    &cols,
                    &rows,
                    g.axis_collect,
                    warnings,
                );
            }
            for (k, cell) in g.cells.into_iter().enumerate() {
                let (r, c) = if g.byrow {
                    (k / g.ncol, k % g.ncol)
                } else {
                    (k % g.nrow, k / g.nrow)
                };
                if r >= g.nrow || c >= g.ncol {
                    // More cells than nrow × ncol: the rest do not fit.
                    warnings.push(format!(
                        "plot_grid: cell {} does not fit a {}×{} grid; skipped",
                        k + 1,
                        g.nrow,
                        g.ncol
                    ));
                    continue;
                }
                let rect = Rect {
                    x: area.x + cols[c].0,
                    y: top + rows[r].0,
                    width: cols[c].1,
                    height: rows[r].1,
                };
                render_prepared(cell, &rect, page, warnings)?;
            }
            Ok(())
        }
    }
}

/// sipemu/ggplot-rs#175: align a grid's own direct cells' plot areas and/or
/// x domain, column by column ([`AxisCollect::Align`]/[`AxisCollect::CollectX`]).
/// `cols`/`rows` are this grid's own track sizes (same for every cell in a
/// column/row), already computed by the caller. Only `Prepared::Plot` cells
/// participate; a column with fewer than two of them is left untouched.
fn align_axes(
    cells: &mut [Prepared],
    // (ncol, nrow, byrow) — bundled so the function stays under clippy's
    // argument-count limit.
    (ncol, nrow, byrow): (usize, usize, bool),
    cols: &[(f64, f64)],
    rows: &[(f64, f64)],
    mode: AxisCollect,
    warnings: &mut Vec<String>,
) {
    let idx_of = |r: usize, c: usize| -> usize {
        if byrow {
            r * ncol + c
        } else {
            c * nrow + r
        }
    };
    for c in 0..ncol {
        let cw = cols.get(c).map_or(0.0, |t| t.1);
        if !cw.is_finite() || cw <= 0.0 {
            continue;
        }
        // (row, cell index) of every plot cell in this column, top to
        // bottom. A spacer or a nested grid is skipped — it keeps its own
        // layout and doesn't count toward "the bottom cell" either.
        let plot_cells: Vec<(usize, usize)> = (0..nrow)
            .filter_map(|r| {
                let k = idx_of(r, c);
                (k < cells.len() && matches!(cells[k], Prepared::Plot(_))).then_some((r, k))
            })
            .collect();
        if plot_cells.len() < 2 {
            continue; // nothing to align against in this column
        }
        let bottom_row = plot_cells.iter().map(|&(r, _)| r).max().unwrap();

        // Switch every cell to `Shift` (so an overflowing outer label can
        // never grow its margins behind our back) and, under `CollectX`,
        // blank every non-bottom cell's x axis *before* measuring, so the
        // measurement already reflects what's actually drawn.
        for &(r, k) in &plot_cells {
            if let Prepared::Plot(p) = &mut cells[k] {
                p.built.theme.axis_text_x_fit = AxisLabelFit::Shift;
                if mode == AxisCollect::CollectX && r != bottom_row {
                    p.built.theme.axis_text_x = ElementText::blank();
                    p.built.theme.axis_title_x = ElementText::blank();
                    p.built.theme.axis_ticks_x = Some(ElementLine::blank());
                }
            }
        }

        // Each cell's own plot-area rect at this column's width — pure
        // layout arithmetic (`GGPlot::layout_built`, the same computation
        // the real render pass makes), not a drawn-and-measured SVG, so
        // this whole pass costs one extra layout per cell, not a redraw.
        // `y`/`height` are kept too, for the thin-cell correction below —
        // changing a cell's left/right margin never moves them (only
        // top/bottom margins and the vertical bands do), so one layout
        // call per cell covers both passes.
        let mut natural: Vec<(f64, f64, f64, f64, f64)> = Vec::with_capacity(plot_cells.len());
        for &(r, k) in &plot_cells {
            let ch = rows.get(r).map_or(0.0, |t| t.1);
            match &mut cells[k] {
                Prepared::Plot(p) => {
                    let layout = GGPlot::layout_built(
                        &mut p.built,
                        &p.meta,
                        cw.floor().max(1.0) as u32,
                        ch.floor().max(1.0) as u32,
                    );
                    natural.push((
                        layout.plot_area.x,
                        layout.plot_area.x + layout.plot_area.width,
                        layout.plot_area.y,
                        layout.plot_area.height,
                        ch,
                    ));
                }
                _ => unreachable!("plot_cells only holds Prepared::Plot indices"),
            }
        }

        // Equal left/right insets: every cell's left margin grows to the
        // widest left requirement in the column, and likewise for the
        // right (anofox-visualization's `track.rs` `left = max(...)`
        // fallback, generalised to both edges and computed once).
        let target_left = natural.iter().fold(f64::MIN, |m, &(l, ..)| m.max(l));
        let target_right_inset = natural
            .iter()
            .fold(f64::MIN, |m, &(_, r, ..)| m.max(cw - r));
        let width = cw - target_right_inset - target_left;
        if target_left.is_finite() && target_right_inset.is_finite() {
            for (&(_, k), &(nl, nr, ..)) in plot_cells.iter().zip(&natural) {
                if let Prepared::Plot(p) = &mut cells[k] {
                    p.built.theme.plot_margin.left += (target_left - nl).max(0.0);
                    p.built.theme.plot_margin.right += (target_right_inset - (cw - nr)).max(0.0);
                }
            }
        }

        // A cell thinner than the plotting engine's 50px panel-height
        // floor (anofox-visualization's `::TRACK` strips, ~16-30px) would
        // otherwise overflow its own canvas: `layout_built` never shrinks
        // a panel below 50px on its own. Mirror `track.rs`'s `Fit::correct`
        // `aspect` trick — a centred `aspect_ratio` set so the panel comes
        // out exactly as tall as what's actually left after the margins —
        // but computed once from this same `layout_built` measurement
        // instead of track.rs's render-measure-render loop. A cell that
        // already fits (the common case — a normal chart cell) is
        // untouched.
        if width.is_finite() && width > 0.0 {
            for (&(_, k), &(_, _, y, h, ch)) in plot_cells.iter().zip(&natural) {
                if let Prepared::Plot(p) = &mut cells[k] {
                    let bottom_budget = ch - p.built.theme.plot_margin.bottom;
                    if y + h > bottom_budget + 0.5 {
                        let fits = (ch - y - p.built.theme.plot_margin.bottom).max(8.0);
                        p.built.theme.aspect_ratio = Some(fits / width);
                        p.built.theme.plot_margin.top -= (h - fits) / 2.0;
                    }
                }
            }
        }

        if mode == AxisCollect::CollectX {
            collect_x_domain(cells, &plot_cells, warnings);
        }
    }
}

/// Train one x domain across a column's cells — the union of every cell's
/// own trained domain — then pin every cell's x scale to it
/// ([`ScaleSet::set_limits`](crate::scale::ScaleSet::set_limits), the same
/// mechanism `coord_cartesian_zoom` uses). Each scale keeps its own
/// `expand`; since every cell now trains the same raw domain, identical
/// `expand` settings produce an identical expanded (drawn) domain too.
/// Skipped, with a warning, when any cell's x scale is discrete or
/// untrained — there's no well-defined union of levels or of nothing.
fn collect_x_domain(
    cells: &mut [Prepared],
    plot_cells: &[(usize, usize)],
    warnings: &mut Vec<String>,
) {
    let mut domains = Vec::with_capacity(plot_cells.len());
    for &(_, k) in plot_cells {
        let d = match &cells[k] {
            Prepared::Plot(p) => match p.built.scales.get(&Aesthetic::X) {
                Some(s) if !s.is_discrete() => s.domain(),
                _ => None,
            },
            _ => None,
        };
        match d {
            Some(d) => domains.push(d),
            None => {
                warnings.push(
                    "plot_grid: a column's x scale is discrete or untrained; x domain not \
                     collected"
                        .to_string(),
                );
                return;
            }
        }
    }
    let lo = domains.iter().map(|d| d.0).fold(f64::INFINITY, f64::min);
    let hi = domains
        .iter()
        .map(|d| d.1)
        .fold(f64::NEG_INFINITY, f64::max);
    if lo.is_finite() && hi.is_finite() && hi >= lo {
        for &(_, k) in plot_cells {
            if let Prepared::Plot(p) = &mut cells[k] {
                p.built.scales.set_limits(&Aesthetic::X, lo, hi);
            }
        }
    }
}

/// A backend wrapper that records the bounding box of everything drawn.
struct Measure<'a> {
    inner: &'a mut SvgBackend,
    pa: Rect,
    bbox: Option<(f64, f64, f64, f64)>,
}

impl<'a> Measure<'a> {
    fn new(inner: &'a mut SvgBackend, pa: Rect) -> Self {
        Measure {
            inner,
            pa,
            bbox: None,
        }
    }

    fn grow(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
            return;
        }
        self.bbox = Some(match self.bbox {
            None => (x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)),
            Some((a, b, c, d)) => (
                a.min(x0.min(x1)),
                b.min(y0.min(y1)),
                c.max(x0.max(x1)),
                d.max(y0.max(y1)),
            ),
        });
    }

    fn extent(&self) -> Option<(f64, f64, f64, f64)> {
        self.bbox
    }
}

impl DrawBackend for Measure<'_> {
    fn draw_circle(
        &mut self,
        c: (f64, f64),
        r: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        self.grow(c.0 - r, c.1 - r, c.0 + r, c.1 + r);
        self.inner.draw_circle(c, r, style)
    }
    fn draw_line(&mut self, pts: &[(f64, f64)], style: &LineStyle) -> Result<(), RenderError> {
        for p in pts {
            self.grow(p.0, p.1, p.0, p.1);
        }
        self.inner.draw_line(pts, style)
    }
    fn draw_quadratic(
        &mut self,
        p0: (f64, f64),
        ctrl: (f64, f64),
        p1: (f64, f64),
        style: &LineStyle,
    ) -> Result<(), RenderError> {
        self.grow(p0.0, p0.1, p0.0, p0.1);
        self.grow(ctrl.0, ctrl.1, ctrl.0, ctrl.1);
        self.grow(p1.0, p1.1, p1.0, p1.1);
        self.inner.draw_quadratic(p0, ctrl, p1, style)
    }
    fn draw_rect(
        &mut self,
        a: (f64, f64),
        b: (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        self.grow(a.0, a.1, b.0, b.1);
        self.inner.draw_rect(a, b, style)
    }
    fn draw_text(
        &mut self,
        text: &str,
        pos: (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        self.draw_text_with_title(text, pos, style, None)
    }
    fn draw_text_with_title(
        &mut self,
        text: &str,
        pos: (f64, f64),
        style: &TextStyle,
        title: Option<&str>,
    ) -> Result<(), RenderError> {
        // Review #167 finding 4 (#170): this wrapper must forward to
        // `inner.draw_text_with_title`, not `inner.draw_text` — the
        // latter is what `DrawBackend::draw_text_with_title`'s own
        // default impl calls, and relying on that default here would
        // silently drop a truncated legend label's `<title>` tooltip
        // (#159) whenever the label is measured through `Measure`
        // (`compose::capture_legend`'s collected-legend path).
        let w = text.chars().count() as f64 * style.size * 0.6;
        let (x0, x1) = match style.anchor {
            TextAnchor::Start => (pos.0, pos.0 + w),
            TextAnchor::Middle => (pos.0 - w / 2.0, pos.0 + w / 2.0),
            TextAnchor::End => (pos.0 - w, pos.0),
        };
        self.grow(x0, pos.1 - style.size / 2.0, x1, pos.1 + style.size / 2.0);
        self.inner.draw_text_with_title(text, pos, style, title)
    }
    fn draw_polygon(&mut self, pts: &[(f64, f64)], style: &RectStyle) -> Result<(), RenderError> {
        for p in pts {
            self.grow(p.0, p.1, p.0, p.1);
        }
        self.inner.draw_polygon(pts, style)
    }
    fn draw_shape(&mut self, c: (f64, f64), r: f64, style: &PointStyle) -> Result<(), RenderError> {
        self.grow(c.0 - r, c.1 - r, c.0 + r, c.1 + r);
        self.inner.draw_shape(c, r, style)
    }
    fn plot_area(&self) -> Rect {
        self.pa.clone()
    }
    fn total_area(&self) -> Rect {
        self.inner.total_area()
    }
    fn set_tooltip(&mut self, t: Option<String>) {
        self.inner.set_tooltip(t)
    }
    fn set_mark_axis(&mut self, k: Option<String>) {
        self.inner.set_mark_axis(k)
    }
    fn set_mark_series(&mut self, s: Option<String>) {
        self.inner.set_mark_series(s)
    }
    fn set_mark_value(&mut self, v: Option<String>) {
        self.inner.set_mark_value(v)
    }
    fn warn(&mut self, m: String) {
        self.inner.warn(m)
    }
    fn set_mark_geom(&mut self, geom: Option<&str>) {
        self.inner.set_mark_geom(geom)
    }
    fn set_mark_part(&mut self, part: Option<&str>) {
        self.inner.set_mark_part(part)
    }
    fn set_mark_attrs(&mut self, attrs: Vec<(String, String)>) {
        self.inner.set_mark_attrs(attrs)
    }
    fn wants_formatted_values(&self) -> bool {
        self.inner.wants_formatted_values()
    }
    fn set_mark_fvalue(&mut self, value: Option<String>) {
        self.inner.set_mark_fvalue(value)
    }
    fn set_mark_fx(&mut self, value: Option<String>) {
        self.inner.set_mark_fx(value)
    }
}

// ─── Operator sugar ─────────────────────────────────────────────────

fn join(op: Op, a: GridCell, b: GridCell) -> PlotGrid {
    let flatten = |c: GridCell| -> Vec<GridCell> {
        match c {
            GridCell::Grid(g) if g.op == Some(op) && g.is_plain() => g.cells,
            other => vec![other],
        }
    };
    let mut cells = flatten(a);
    cells.extend(flatten(b));
    PlotGrid {
        cells,
        op: Some(op),
        ..PlotGrid::default()
    }
}

impl PlotGrid {
    /// Built purely by an operator chain (no explicit layout/annotations), so
    /// it can be flattened into a longer chain of the same operator.
    fn is_plain(&self) -> bool {
        self.ncol.is_none()
            && self.nrow.is_none()
            && self.widths.is_empty()
            && self.heights.is_empty()
            && self.title.is_none()
            && self.subtitle.is_none()
            && self.caption.is_none()
            && self.tags.is_none()
            && !self.collect_legends
            && self.axis_collect == AxisCollect::Keep
    }
}

macro_rules! grid_ops {
    ($lhs:ty, $rhs:ty) => {
        /// `a | b`: place side by side (patchwork).
        impl BitOr<$rhs> for $lhs {
            type Output = PlotGrid;
            fn bitor(self, rhs: $rhs) -> PlotGrid {
                join(Op::Row, self.into(), rhs.into())
            }
        }
        /// `a / b`: stack vertically (patchwork).
        impl Div<$rhs> for $lhs {
            type Output = PlotGrid;
            fn div(self, rhs: $rhs) -> PlotGrid {
                join(Op::Col, self.into(), rhs.into())
            }
        }
    };
}

grid_ops!(GGPlot, GGPlot);
grid_ops!(GGPlot, PlotGrid);
grid_ops!(PlotGrid, GGPlot);
grid_ops!(PlotGrid, PlotGrid);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_sequences() {
        let l: Vec<String> = (0..28).map(|i| TagLevels::Lower.tag(i)).collect();
        assert_eq!(&l[..3], &["a", "b", "c"]);
        assert_eq!(l[25], "z");
        assert_eq!(l[26], "aa");
        assert_eq!(l[27], "ab");
        assert_eq!(TagLevels::Upper.tag(1), "B");
        assert_eq!(TagLevels::Numeric.tag(9), "10");
        assert_eq!(TagLevels::LowerRoman.tag(3), "iv");
        assert_eq!(TagLevels::UpperRoman.tag(13), "XIV");
        assert_eq!(TagLevels::Custom(vec!["x".into()]).tag(1), "2");
    }

    #[test]
    fn tracks_are_relative_and_robust() {
        let t = tracks(&[2.0, 1.0], 2, 300.0, 0.0);
        assert_eq!(t, vec![(0.0, 200.0), (200.0, 100.0)]);
        let t = tracks(&[f64::NAN, -1.0], 2, 100.0, 10.0);
        assert_eq!(t, vec![(0.0, 45.0), (55.0, 45.0)]);
        let t = tracks(&[], 3, 30.0, 100.0);
        assert!(t.iter().all(|(p, s)| p.is_finite() && *s == 0.0));
    }

    /// `Measure` (the legend bbox pre-pass) forwards every `DrawBackend`
    /// method to its inner backend and grows its tracked extent to include
    /// a quadratic curve's control points — not just straight lines/shapes.
    #[test]
    fn measure_forwards_draw_quadratic_and_mark_metadata_and_grows_its_extent() {
        let pa = Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let mut backend = SvgBackend::new(100, 100, pa.clone());
        backend.set_formatted_values(true);
        let mut meas = Measure::new(&mut backend, pa);

        assert!(meas.extent().is_none());
        // B2-4: Measure forwards the formatted-values query/setters too.
        assert!(meas.wants_formatted_values());
        meas.set_mark_geom(Some("line"));
        meas.set_mark_part(Some("arrow"));
        meas.set_mark_attrs(vec![("data-x".to_string(), "v".to_string())]);
        meas.set_mark_fvalue(Some("1,234.5".to_string()));
        meas.set_mark_fx(Some("2026-10-12".to_string()));
        meas.draw_quadratic(
            (10.0, 20.0),
            (90.0, 5.0),
            (50.0, 80.0),
            &LineStyle::default(),
        )
        .unwrap();
        meas.set_mark_geom(None);
        meas.set_mark_part(None);
        meas.set_mark_attrs(Vec::new());
        meas.set_mark_fvalue(None);
        meas.set_mark_fx(None);

        let (x0, y0, x1, y1) = meas.extent().expect("extent after drawing");
        assert_eq!((x0, y0, x1, y1), (10.0, 5.0, 90.0, 80.0));

        // The marks actually reached the inner SvgBackend (data-part, the
        // data_attr, the mark class and the formatted values all appear in
        // its output).
        let svg = backend.finish();
        assert!(svg.contains("data-part=\"arrow\""));
        assert!(svg.contains("data-x=\"v\""));
        assert!(svg.contains("gg-arrow"));
        assert!(svg.contains("data-fvalue=\"1,234.5\""));
        assert!(svg.contains("data-fx=\"2026-10-12\""));
    }

    /// #170 / review #167 finding 4: `Measure::draw_text_with_title` must
    /// forward the `<title>` to its inner `SvgBackend`, not fall through
    /// to `DrawBackend::draw_text_with_title`'s default impl (which calls
    /// `self.draw_text`, and `Measure::draw_text` only ever calls
    /// `inner.draw_text` — dropping the title). This only matters once a
    /// `<title>` is actually passed through a `Measure`-wrapped backend
    /// (today that's `compose::capture_legend`'s collected-legend path, a
    /// `PlotGrid`'s B3-2 label truncation doesn't run yet, so the bug is
    /// latent rather than reachable from `PlotGrid`'s own public API —
    /// hence this unit test against `Measure` directly).
    #[test]
    fn measure_forwards_the_title_to_its_inner_backend() {
        let pa = Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let mut backend = SvgBackend::new(100, 100, pa.clone());
        let mut meas = Measure::new(&mut backend, pa);
        meas.draw_text_with_title(
            "Grp…",
            (10.0, 20.0),
            &TextStyle {
                size: 12.0,
                anchor: TextAnchor::Start,
                ..Default::default()
            },
            Some("Group 01"),
        )
        .unwrap();
        let svg = backend.finish();
        assert!(
            svg.contains("<title>Group 01</title>"),
            "the full label should reach the inner backend's <title>: {svg}"
        );
    }
}
