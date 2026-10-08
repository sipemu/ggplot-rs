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

use crate::guide::legend;
use crate::plot::{GGError, GGPlot, RenderMeta};
use crate::render::backend::{
    DrawBackend, LineStyle, PointStyle, RectStyle, TextAnchor, TextStyle,
};
use crate::render::renderer::PlotRenderer;
use crate::render::svg_backend::{root_data_attrs, SvgBackend};
use crate::render::{Rect, RenderError};
use crate::theme::{LegendPosition, Theme};

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
    op: Option<Op>,
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
            op: None,
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
        };
        let prepared = prepare_grid(self, &mut ctx)?;
        let legends = ctx.legends;

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
        page.set_root_attrs(vec![("data-grid".into(), format!("{rows} {cols}"))]);
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
        let mut warnings = Vec::new();
        render_prepared(prepared, &body, &mut page, &mut warnings)?;
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
}

/// A built sub-plot awaiting its cell size.
struct PreparedPlot {
    built: crate::build::BuiltPlot,
    meta: RenderMeta,
    panel: String,
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
    let (built, mut meta) = plot.build_for_render()?;
    if let Some(pos) = collect {
        meta.has_legend = false;
        if let Some(l) = capture_legend(&built, pos).map_err(GGError::Render)? {
            if !ctx.legends.iter().any(|o| o.body == l.body) {
                ctx.legends.push(l);
            }
        }
    }
    Ok(PreparedPlot { built, meta, panel })
}

/// Draw a built plot's legend in isolation, measuring its extent.
fn capture_legend(
    built: &crate::build::BuiltPlot,
    pos: GridLegendPosition,
) -> Result<Option<Legend>, RenderError> {
    let mut theme = built.theme.clone();
    let m = theme.legend_margin.clone();
    // Fake a zero-size panel so the legend's origin lands at (0, 0).
    let (pa, lp) = match pos {
        GridLegendPosition::Right => (
            Rect {
                x: -m.left,
                y: -m.top,
                width: 0.0,
                height: 0.0,
            },
            LegendPosition::Right,
        ),
        GridLegendPosition::Bottom => (
            Rect {
                x: -m.left,
                y: -(m.top + 30.0),
                width: 0.0,
                height: 0.0,
            },
            LegendPosition::Bottom,
        ),
    };
    theme.legend_position = lp;
    let mut svg = SvgBackend::new(1, 1, pa.clone());
    let mut meas = Measure::new(&mut svg, pa.clone());
    legend::draw_legend(
        &built.scales,
        &theme,
        &pa,
        &mut meas,
        &built.guide_legend,
        &built.suppressed_aes,
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
            } = *p;
            let (cw, ch) = (
                area.width.floor().max(1.0) as u32,
                area.height.floor().max(1.0) as u32,
            );
            let layout = GGPlot::layout_built(&mut built, &meta, cw, ch);
            let mut backend = SvgBackend::new(cw, ch, layout.plot_area.clone());
            let mut attrs = root_data_attrs(&built);
            attrs.push(("data-panel".into(), panel.clone()));
            backend.set_root_attrs(attrs);
            PlotRenderer::render(&built, &mut backend).map_err(GGError::Render)?;
            warnings.extend(
                built
                    .warnings
                    .iter()
                    .cloned()
                    .chain(backend.take_warnings())
                    .map(|w| format!("panel {panel}: {w}")),
            );
            page.push_raw(&backend.finish_fragment(area.x, area.y));
            Ok(())
        }
        Prepared::Grid(g) => {
            let g = *g;
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
        let w = text.chars().count() as f64 * style.size * 0.6;
        let (x0, x1) = match style.anchor {
            TextAnchor::Start => (pos.0, pos.0 + w),
            TextAnchor::Middle => (pos.0 - w / 2.0, pos.0 + w / 2.0),
            TextAnchor::End => (pos.0 - w, pos.0),
        };
        self.grow(x0, pos.1 - style.size / 2.0, x1, pos.1 + style.size / 2.0);
        self.inner.draw_text(text, pos, style)
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
}
