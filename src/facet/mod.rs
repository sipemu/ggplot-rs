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
    },
    Grid {
        row_var: Option<String>,
        col_var: Option<String>,
        scales: FacetScales,
    },
}

impl Facet {
    pub fn is_none(&self) -> bool {
        matches!(self, Facet::None)
    }
}
