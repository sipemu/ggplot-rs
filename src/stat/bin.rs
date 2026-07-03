use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// ggplot2 bin alignment: place bins so an edge falls on `boundary` (mod
/// `width`), then return the left edge of the first bin (`origin`, ≤ `min`) and
/// the number of bins needed to cover `[min, max]`.
pub(crate) fn aligned_bins_at(min: f64, max: f64, width: f64, boundary: f64) -> (f64, usize) {
    let shift = ((min - boundary) / width).floor();
    let origin = boundary + shift * width;
    let n = (((max - origin) / width).ceil() as usize).max(1);
    (origin, n)
}

/// 1-D histogram alignment: a *bin* is centered on 0 (`boundary = width/2`),
/// matching `geom_histogram`.
fn aligned_bins(min: f64, max: f64, width: f64) -> (f64, usize) {
    aligned_bins_at(min, max, width, width / 2.0)
}

/// Bins continuous x values into histogram bins.
pub struct StatBin {
    pub bins: usize,
    pub binwidth: Option<f64>,
}

impl StatBin {
    /// Set bin width (overrides bins count).
    pub fn with_binwidth(mut self, width: f64) -> Self {
        self.binwidth = Some(width);
        self
    }

    /// Set number of bins.
    pub fn with_bins(mut self, bins: usize) -> Self {
        self.bins = bins;
        self.binwidth = None;
        self
    }
}

impl Default for StatBin {
    fn default() -> Self {
        StatBin {
            bins: 30,
            binwidth: None,
        }
    }
}

impl Stat for StatBin {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let values: Vec<f64> = x_col.iter().filter_map(|v| v.as_f64()).collect();
        if values.is_empty() {
            return DataFrame::new();
        }

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Handle case where all values are the same
        let (min, max) = if (max - min).abs() < f64::EPSILON {
            (min - 0.5, max + 0.5)
        } else {
            (min, max)
        };

        // Match ggplot2's bin_breaks: for a bin count, width spans the range in
        // `bins - 1` steps; bins are then aligned to `boundary = width/2` so a
        // bin is centered on 0 (the origin is shifted left of the data min),
        // rather than starting exactly at the data minimum.
        let (bin_width, origin, n_bins) = if let Some(bw) = self.binwidth {
            let (o, n) = aligned_bins(min, max, bw);
            (bw, o, n)
        } else if self.bins <= 1 {
            (max - min, min, 1)
        } else {
            let bw = (max - min) / (self.bins - 1) as f64;
            let (o, n) = aligned_bins(min, max, bw);
            (bw, o, n)
        };

        let mut counts = vec![0usize; n_bins];

        for &v in &values {
            // ggplot2's default bins are right-closed: (a, b]. A point on a
            // boundary falls in the lower bin.
            let raw = ((v - origin) / bin_width).ceil() as i64 - 1;
            let bin = raw.clamp(0, n_bins as i64 - 1) as usize;
            counts[bin] += 1;
        }

        let total = values.len() as f64;
        let mut x_vals = Vec::with_capacity(n_bins);
        let mut y_vals = Vec::with_capacity(n_bins);
        let mut density_vals = Vec::with_capacity(n_bins);
        let mut xmin_vals = Vec::with_capacity(n_bins);
        let mut xmax_vals = Vec::with_capacity(n_bins);

        for (i, &count) in counts.iter().enumerate() {
            let bin_min = origin + i as f64 * bin_width;
            let bin_max = bin_min + bin_width;
            let center = (bin_min + bin_max) / 2.0;

            x_vals.push(Value::Float(center));
            y_vals.push(Value::Float(count as f64));
            density_vals.push(Value::Float(count as f64 / (total * bin_width)));
            xmin_vals.push(Value::Float(bin_min));
            xmax_vals.push(Value::Float(bin_max));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals.clone());
        // Expose the count under its ggplot stat name for after_stat expressions
        // (e.g. after_stat_y("count / sum(count)")).
        result.add_column("count".to_string(), y_vals);
        result.add_column("density".to_string(), density_vals);
        result.add_column("xmin".to_string(), xmin_vals);
        result.add_column("xmax".to_string(), xmax_vals);
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn name(&self) -> &str {
        "bin"
    }
}
