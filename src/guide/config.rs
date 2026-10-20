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
