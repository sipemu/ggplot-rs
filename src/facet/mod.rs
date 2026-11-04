pub mod grid;
pub mod wrap;

use crate::render::Rect;

/// Whether facet panels share axes.
#[derive(Clone, Debug)]
pub enum FacetScales {
    Fixed,
    FreeX,
    FreeY,
    Free,
}

/// Whether facet panels are sized proportionally to their data range
/// (R's `space =`). `Fixed` = equal-size panels (default).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum FacetSpace {
    #[default]
    Fixed,
    FreeX,
    FreeY,
    Free,
}

impl FacetSpace {
    pub fn free_x(&self) -> bool {
        matches!(self, FacetSpace::FreeX | FacetSpace::Free)
    }
    pub fn free_y(&self) -> bool {
        matches!(self, FacetSpace::FreeY | FacetSpace::Free)
    }
}

/// How facet strip labels are formatted.
#[derive(Clone, Default)]
pub enum FacetLabeller {
    /// Show just the value (default).
    #[default]
    Value,
    /// Show "var: value".
    Both,
    /// Custom formatting function: fn(variable_name, value) -> label.
    Custom(fn(&str, &str) -> String),
    /// The value, wrapped onto lines of at most this many characters
    /// (ggplot2's `label_wrap_gen(width)`, #412).
    Wrap(usize),
}

/// Greedy word wrap at `width` characters (`strwrap`), lines joined by `\n`.
pub fn wrap_label(text: &str, width: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(l) if l.chars().count() + 1 + word.chars().count() <= width.max(1) => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines.join("\n")
}

impl FacetLabeller {
    pub fn format(&self, var: &str, value: &str) -> String {
        match self {
            FacetLabeller::Value => value.to_string(),
            FacetLabeller::Both => format!("{var}: {value}"),
            FacetLabeller::Custom(f) => f(var, value),
            FacetLabeller::Wrap(w) => wrap_label(value, *w),
        }
    }
}

/// A single panel in a faceted layout.
#[derive(Clone, Debug)]
pub struct Panel {
    pub row: usize,
    pub col: usize,
    pub label: String,
    pub row_label: Option<String>,
    pub col_label: Option<String>,
    pub rect: Rect,
}

/// Facet specification.
#[derive(Clone, Default)]
pub enum Facet {
    #[default]
    None,
    Wrap {
        var: String,
        ncol: Option<usize>,
        scales: FacetScales,
        labeller: FacetLabeller,
        /// Explicit panel order (ggplot2's factor levels): every level gets
        /// a panel, even with no data (`drop = FALSE`), and rows whose value
        /// isn't listed are dropped. `None` = values in first-appearance
        /// order. Set with [`GGPlot::facet_levels`](crate::GGPlot::facet_levels).
        levels: Option<Vec<String>>,
        /// Fill panels column by column (ggplot2's `dir = "v"`, #412).
        vertical: bool,
        /// First row at the top (default; ggplot2's `as.table = TRUE`).
        as_table: bool,
    },
    Grid {
        row_var: Option<String>,
        col_var: Option<String>,
        scales: FacetScales,
        labeller: FacetLabeller,
        #[doc = "Proportional panel sizing (R's `space =`)."]
        space: FacetSpace,
    },
}

impl Facet {
    pub fn is_none(&self) -> bool {
        matches!(self, Facet::None)
    }

    pub fn labeller(&self) -> &FacetLabeller {
        match self {
            Facet::None => &FacetLabeller::Value,
            Facet::Wrap { labeller, .. } => labeller,
            Facet::Grid { labeller, .. } => labeller,
        }
    }
}
