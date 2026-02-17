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
}

impl GuideLegend {
    pub fn new() -> Self {
        Self::default()
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
}
