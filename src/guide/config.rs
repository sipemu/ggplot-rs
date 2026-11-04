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
    /// Aesthetics whose legend keys run in reverse (`guides(colour =
    /// guide_legend(reverse = TRUE))`, #409) — `reverse` above flips all.
    pub reversed: Vec<crate::aes::Aesthetic>,
    /// Per-aesthetic colourbar options (`guide_colourbar(..)` /
    /// `guide_coloursteps(..)`, #409) for a continuous colour / fill.
    pub colourbars: Vec<(crate::aes::Aesthetic, GuideColourbar)>,
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
