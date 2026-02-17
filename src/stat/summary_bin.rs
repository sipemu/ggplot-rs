use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::summary::SummaryFun;
use super::Stat;

/// Bin x values and apply summary function to y within each bin.
/// Produces x (bin center), y, ymin, ymax columns.
pub struct StatSummaryBin {
    pub bins: usize,
    pub fun_y: SummaryFun,
    pub fun_ymin: SummaryFun,
    pub fun_ymax: SummaryFun,
}

impl Default for StatSummaryBin {
    fn default() -> Self {
        StatSummaryBin {
            bins: 30,
            fun_y: SummaryFun::Mean,
            fun_ymin: SummaryFun::Min,
            fun_ymax: SummaryFun::Max,
        }
    }
}

impl StatSummaryBin {
    pub fn with_bins(mut self, bins: usize) -> Self {
        self.bins = bins;
        self
    }

    pub fn with_fun(mut self, fun_y: SummaryFun) -> Self {
        self.fun_y = fun_y;
        self
    }

    pub fn with_fun_range(mut self, fun_ymin: SummaryFun, fun_ymax: SummaryFun) -> Self {
        self.fun_ymin = fun_ymin;
        self.fun_ymax = fun_ymax;
        self
    }
}

impl Stat for StatSummaryBin {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        // Extract numeric pairs
        let mut pairs: Vec<(f64, f64)> = Vec::new();
        for (x, y) in x_col.iter().zip(y_col.iter()) {
            if let (Some(xv), Some(yv)) = (x.as_f64(), y.as_f64()) {
                if xv.is_finite() && yv.is_finite() {
                    pairs.push((xv, yv));
                }
            }
        }

        if pairs.is_empty() {
            return DataFrame::new();
        }

        let x_min = pairs.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x_max = pairs.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);

        let (x_min, x_max) = if (x_max - x_min).abs() < f64::EPSILON {
            (x_min - 0.5, x_max + 0.5)
        } else {
            (x_min, x_max)
        };

        let bin_width = (x_max - x_min) / self.bins as f64;
        let n_bins = self.bins;

        // Collect y values per bin
        let mut bin_ys: Vec<Vec<f64>> = vec![Vec::new(); n_bins];
        for &(x, y) in &pairs {
            let bin = ((x - x_min) / bin_width).floor() as usize;
            let bin = bin.min(n_bins - 1);
            bin_ys[bin].push(y);
        }

        let mut x_vals = Vec::new();
        let mut y_vals = Vec::new();
        let mut ymin_vals = Vec::new();
        let mut ymax_vals = Vec::new();

        for (i, ys) in bin_ys.iter().enumerate() {
            if ys.is_empty() {
                continue;
            }
            let bin_center = x_min + (i as f64 + 0.5) * bin_width;
            x_vals.push(Value::Float(bin_center));
            y_vals.push(Value::Float(self.fun_y.apply(ys)));
            ymin_vals.push(Value::Float(self.fun_ymin.apply(ys)));
            ymax_vals.push(Value::Float(self.fun_ymax.apply(ys)));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result.add_column("ymin".to_string(), ymin_vals);
        result.add_column("ymax".to_string(), ymax_vals);

        // Carry over grouping columns
        let n = result.nrows();
        for col_name in &["color", "fill", "group"] {
            if let Some(col) = data.column(col_name) {
                if let Some(first) = col.first() {
                    result.add_column(col_name.to_string(), vec![first.clone(); n]);
                }
            }
        }

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "summary_bin"
    }
}
