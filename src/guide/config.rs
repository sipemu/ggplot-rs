/// Configuration for legend guides.
/// Analogous to R's `guide_legend()` and `guides()`.
#[derive(Clone, Debug, Default)]
pub struct GuideLegend {
    /// Override the legend title. None = use scale name.
    pub title: Option<String>,
    /// Number of columns in the legend layout.
    pub ncol: Option<usize>,
    /// Number of rows in the legend layout.
    pub nrow: Option<usize>,
    /// Reverse the order of legend keys.
    pub reverse: bool,
    /// Fill an explicit `nrow`/`ncol` grid row-major instead of ggplot2's
    /// default column-major (B3-2; R's `guide_legend(byrow = TRUE)`).
    pub byrow: bool,
    /// Cap on the number of rows an auto-wrapped horizontal legend may grow
    /// to (B3-2); entries beyond the cap collapse into a trailing "+N more"
    /// label. `None` (the default) uses `1 + floor(total_height / 3 /
    /// key_height)`. `Some(0)` is invalid (see `validated`, internal) and
    /// is treated as `Some(1)`.
    pub max_rows: Option<usize>,
    /// Aesthetics whose legend is dropped (ggplot2's `guides(fill = "none")`, #409).
    pub hidden: Vec<crate::aes::Aesthetic>,
    /// Per-aesthetic legend titles (`guides(colour = guide_legend(title =))`, #409).
    pub titles: Vec<(crate::aes::Aesthetic, String)>,
    /// Fixed key appearance per legend (ggplot2's `guide_legend(override.aes
    /// = list(...))`, #409).
    pub overrides: Vec<(crate::aes::Aesthetic, KeyOverride)>,
    /// Aesthetics whose legend keys run in reverse (`guides(colour =
    /// guide_legend(reverse = TRUE))`, #409) — `reverse` above flips all.
    pub reversed: Vec<crate::aes::Aesthetic>,
    /// Per-aesthetic colourbar options (`guide_colourbar(..)` /
    /// `guide_coloursteps(..)`, #409) for a continuous colour / fill.
    pub colourbars: Vec<(crate::aes::Aesthetic, GuideColourbar)>,
    /// Per-aesthetic `guide_bins()` options (#409) for a binned scale.
    pub bins: Vec<(crate::aes::Aesthetic, GuideBins)>,
    /// Per-aesthetic legend positions (ggplot2 3.5's `guide_legend(position
    /// =)`, #409): those legends draw at their own side instead of the
    /// theme's `legend_position`.
    pub positions: Vec<(crate::aes::Aesthetic, crate::theme::LegendPosition)>,
    /// Custom legend entries (`guide_custom()`, #409), drawn after the
    /// scales' own legends.
    pub custom: Vec<GuideCustom>,
    /// Binned-scale aesthetics drawn as a plain key-per-bin legend
    /// (`guides(size = "legend")`) instead of their default `guide_bins()`.
    pub plain_keys: Vec<crate::aes::Aesthetic>,
    /// Filled by the build (#410): every layer's legend glyph and the legend
    /// aesthetics it shows, so keys draw each layer's own picture.
    #[doc(hidden)]
    pub layer_keys: Vec<crate::guide::key::LayerKey>,
}

impl GuideLegend {
    pub fn new() -> Self {
        Self::default()
    }

    /// The colourbar options for `aes` (the defaults when none were set).
    pub(crate) fn colourbar_for(&self, aes: &crate::aes::Aesthetic) -> GuideColourbar {
        self.colourbars
            .iter()
            .find(|(a, _)| a == aes)
            .map(|(_, c)| c.clone())
            .unwrap_or_default()
    }

    /// The `guide_bins()` options `aes`'s legend draws with: the ones set
    /// for it, or the defaults for a binned scale (ggplot2's default guide
    /// for `scale_*_binned()`) — `None` for a plain legend.
    pub(crate) fn bins_for(
        &self,
        aes: &crate::aes::Aesthetic,
        scale: &dyn crate::scale::Scale,
    ) -> Option<GuideBins> {
        scale.bin_edge_labels()?;
        if self.plain_keys.contains(aes) {
            return None;
        }
        Some(
            self.bins
                .iter()
                .find(|(a, _)| a == aes)
                .map(|(_, b)| b.clone())
                .unwrap_or_default(),
        )
    }

    /// Where `aes`'s legend draws: its own position, else `main`.
    pub(crate) fn position_for<'a>(
        &'a self,
        aes: &crate::aes::Aesthetic,
        main: &'a crate::theme::LegendPosition,
    ) -> &'a crate::theme::LegendPosition {
        self.positions
            .iter()
            .find(|(a, _)| a == aes)
            .map_or(main, |(_, p)| p)
    }

    /// Are `aes`'s legend keys reversed (globally or for `aes` alone)?
    pub(crate) fn is_reversed(&self, aes: &crate::aes::Aesthetic) -> bool {
        self.reverse != self.reversed.contains(aes)
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_ncol(mut self, ncol: usize) -> Self {
        self.ncol = Some(ncol);
        self
    }

    pub fn with_nrow(mut self, nrow: usize) -> Self {
        self.nrow = Some(nrow);
        self
    }

    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Fill an explicit grid row-major (left-to-right, top-to-bottom)
    /// instead of ggplot2's default column-major.
    pub fn with_byrow(mut self, byrow: bool) -> Self {
        self.byrow = byrow;
        self
    }

    /// Cap the number of rows an auto-wrapped horizontal legend may grow to.
    pub fn with_max_rows(mut self, max_rows: usize) -> Self {
        self.max_rows = Some(max_rows);
        self
    }

    /// Sanitize `nrow`/`ncol`/`max_rows` against `n` breaks (B3-2, D2):
    /// `nrow`/`ncol` of `0` is treated as unset; an explicit grid smaller
    /// than `n` drops `ncol` (falls back to an `nrow`-only grid, which
    /// grows columns to fit); `max_rows` of `0` is treated as `1`. Each
    /// correction pushes one warning into `warnings` and never panics.
    pub(crate) fn validated(&self, n: usize, warnings: &mut Vec<String>) -> GuideLegend {
        let mut out = self.clone();

        if out.nrow == Some(0) {
            warnings.push("guide_legend: nrow(0) is invalid; using the default".to_string());
            out.nrow = None;
        }
        if out.ncol == Some(0) {
            warnings.push("guide_legend: ncol(0) is invalid; using the default".to_string());
            out.ncol = None;
        }
        if let (Some(nrow), Some(ncol)) = (out.nrow, out.ncol) {
            if nrow.saturating_mul(ncol) < n {
                warnings.push(format!(
                    "guide_legend: nrow({nrow}) x ncol({ncol}) = {} is smaller than {n} keys; dropping ncol",
                    nrow.saturating_mul(ncol)
                ));
                out.ncol = None;
            }
        }
        if out.max_rows == Some(0) {
            warnings.push("guide_legend: max_rows(0) is invalid; using 1".to_string());
            out.max_rows = Some(1);
        }
        out
    }
}

/// The drawing callback of a [`GuideCustom`]: paints into the given box.
pub type CustomGuideDraw = std::sync::Arc<
    dyn Fn(
            &mut dyn crate::render::backend::DrawBackend,
            &crate::render::Rect,
        ) -> Result<(), crate::render::RenderError>
        + Send
        + Sync,
>;

/// A legend entry drawn by your own code — ggplot2's `guide_custom(grob,
/// width, height, title)` (#409): a `width` × `height` px box in the legend
/// stack (under `title`, if any), handed to `draw` to paint with the usual
/// [`DrawBackend`](crate::render::backend::DrawBackend) calls. Add with
/// [`GGPlot::guide_custom`](crate::GGPlot::guide_custom).
#[derive(Clone)]
pub struct GuideCustom {
    pub title: Option<String>,
    pub width: f64,
    pub height: f64,
    pub draw: CustomGuideDraw,
}

impl GuideCustom {
    /// A `width` × `height` px box painted by `draw`.
    pub fn new<F>(width: f64, height: f64, draw: F) -> Self
    where
        F: Fn(
                &mut dyn crate::render::backend::DrawBackend,
                &crate::render::Rect,
            ) -> Result<(), crate::render::RenderError>
            + Send
            + Sync
            + 'static,
    {
        GuideCustom {
            title: None,
            width: width.max(0.0),
            height: height.max(0.0),
            draw: std::sync::Arc::new(draw),
        }
    }

    /// A title above the box, styled like the other legend titles.
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Height of the title line above the box (0 without a title).
    pub(crate) fn title_offset(&self, theme: &crate::theme::Theme) -> f64 {
        match &self.title {
            Some(t) if !t.is_empty() => theme.legend_title.size + 4.0,
            _ => 0.0,
        }
    }
}

impl std::fmt::Debug for GuideCustom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuideCustom")
            .field("title", &self.title)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

/// Options of a binned scale's legend — ggplot2's `guide_bins()` (#409):
/// keys stacked edge to edge, labelled at the bin boundaries along a small
/// axis. Apply with [`GGPlot::guide_bins`](crate::GGPlot::guide_bins).
#[derive(Clone, Debug, PartialEq)]
pub struct GuideBins {
    /// Label the two outer limits too (`show.limits`, #408).
    pub show_limits: bool,
    /// Draw the axis line and ticks beside the keys.
    pub axis: bool,
}

impl Default for GuideBins {
    fn default() -> Self {
        GuideBins {
            show_limits: false,
            axis: true,
        }
    }
}

impl GuideBins {
    pub fn new() -> Self {
        Self::default()
    }

    /// Label the outer limits (`show.limits = TRUE`).
    pub fn with_show_limits(mut self, on: bool) -> Self {
        self.show_limits = on;
        self
    }

    /// Draw the axis line and ticks (`TRUE` by default).
    pub fn with_axis(mut self, on: bool) -> Self {
        self.axis = on;
        self
    }
}

/// How a colourbar paints its colours (ggplot2's `guide_colourbar(display =)`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColourbarDisplay {
    /// A smooth ramp (`"gradient"`, the default).
    #[default]
    Gradient,
    /// `nbin` solid, visibly separate blocks (`"rectangles"`).
    Rectangles,
}

/// Options of a continuous colour / fill legend — ggplot2's
/// `guide_colourbar()`, or `guide_coloursteps()` when `steps` is set
/// (#409). Apply with [`GGPlot::guide_colourbar`](crate::GGPlot::guide_colourbar).
#[derive(Clone, Debug, PartialEq)]
pub struct GuideColourbar {
    /// The bar's horizontal extent in px (`barwidth`); `None` = the theme's
    /// key size (long side 8 keys).
    pub bar_width: Option<f64>,
    /// The bar's vertical extent in px (`barheight`).
    pub bar_height: Option<f64>,
    /// Colour samples along the bar (`nbin`).
    pub nbin: usize,
    /// Gradient or separate rectangles (`display`).
    pub display: ColourbarDisplay,
    /// Draw the outline round the bar (`frame`); `None` = the theme's
    /// `legend.frame` (`legend_bar_border`).
    pub frame: Option<bool>,
    /// Draw tick marks at the breaks (`ticks`).
    pub ticks: bool,
    /// `guide_coloursteps()`: one solid block per interval between breaks.
    pub steps: bool,
    /// Coloursteps: every block the same length (`even.steps`, default
    /// `true`), instead of proportional to its value range.
    pub even_steps: bool,
    /// Coloursteps: label the two outer limits too (`show.limits`).
    pub show_limits: bool,
}

impl Default for GuideColourbar {
    fn default() -> Self {
        GuideColourbar {
            bar_width: None,
            bar_height: None,
            nbin: 50,
            display: ColourbarDisplay::Gradient,
            frame: None,
            ticks: true,
            steps: false,
            even_steps: true,
            show_limits: false,
        }
    }
}

impl GuideColourbar {
    /// `guide_colourbar()` with its defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// `guide_coloursteps()`: stepped blocks, even steps, no ticks.
    pub fn steps() -> Self {
        GuideColourbar {
            steps: true,
            ticks: false,
            ..Self::default()
        }
    }

    pub fn with_bar_width(mut self, px: f64) -> Self {
        self.bar_width = Some(px);
        self
    }

    pub fn with_bar_height(mut self, px: f64) -> Self {
        self.bar_height = Some(px);
        self
    }

    pub fn with_nbin(mut self, nbin: usize) -> Self {
        self.nbin = nbin.max(1);
        self
    }

    pub fn with_display(mut self, display: ColourbarDisplay) -> Self {
        self.display = display;
        self
    }

    pub fn with_frame(mut self, frame: bool) -> Self {
        self.frame = Some(frame);
        self
    }

    pub fn with_ticks(mut self, ticks: bool) -> Self {
        self.ticks = ticks;
        self
    }

    pub fn with_even_steps(mut self, even: bool) -> Self {
        self.even_steps = even;
        self
    }

    pub fn with_show_limits(mut self, show: bool) -> Self {
        self.show_limits = show;
        self
    }
}

/// Fixed values a legend's keys are drawn with, whatever the data maps
/// (ggplot2's `override.aes`, #409) — e.g. opaque, larger keys for a
/// scatter plot drawn with small, translucent points. `None` keeps the key's
/// own value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyOverride {
    pub color: Option<(u8, u8, u8)>,
    pub fill: Option<(u8, u8, u8)>,
    pub shape: Option<crate::render::backend::PointShape>,
    pub linetype: Option<crate::render::backend::Linetype>,
    pub size: Option<f64>,
    pub alpha: Option<f64>,
    pub linewidth: Option<f64>,
}

impl KeyOverride {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn color(mut self, c: (u8, u8, u8)) -> Self {
        self.color = Some(c);
        self
    }
    pub fn fill(mut self, c: (u8, u8, u8)) -> Self {
        self.fill = Some(c);
        self
    }
    pub fn shape(mut self, s: crate::render::backend::PointShape) -> Self {
        self.shape = Some(s);
        self
    }
    pub fn linetype(mut self, l: crate::render::backend::Linetype) -> Self {
        self.linetype = Some(l);
        self
    }
    pub fn size(mut self, v: f64) -> Self {
        self.size = Some(v);
        self
    }
    pub fn alpha(mut self, v: f64) -> Self {
        self.alpha = Some(v.clamp(0.0, 1.0));
        self
    }
    pub fn linewidth(mut self, v: f64) -> Self {
        self.linewidth = Some(v);
        self
    }
}
