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
}

impl FacetLabeller {
    pub fn format(&self, var: &str, value: &str) -> String {
        match self {
            FacetLabeller::Value => value.to_string(),
            FacetLabeller::Both => format!("{var}: {value}"),
            FacetLabeller::Custom(f) => f(var, value),
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
