use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::summary::SummaryFun;
use super::Stat;

/// 2-D binned summary (analogous to R's `stat_summary_2d`): bins x/y into a grid
/// and applies a summary function to the `z` values in each cell, emitting
/// `xmin/xmax/ymin/ymax/fill` (fill = the per-cell summary) for `geom_bin2d`.
pub struct StatSummary2d {
    pub bins_x: usize,
    pub bins_y: usize,
    pub fun: SummaryFun,
}

impl Default for StatSummary2d {
    fn default() -> Self {
        StatSummary2d {
            bins_x: 30,
            bins_y: 30,
            fun: SummaryFun::Mean,
        }
    }
}

impl StatSummary2d {
    pub fn new(fun: SummaryFun) -> Self {
        StatSummary2d {
            fun,
            ..Default::default()
        }
    }

    pub fn with_bins(mut self, bins_x: usize, bins_y: usize) -> Self {
        self.bins_x = bins_x.max(1);
        self.bins_y = bins_y.max(1);
        self
    }
}

impl Stat for StatSummary2d {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (x_col, y_col, z_col) = match (data.column("x"), data.column("y"), data.column("z")) {
            (Some(x), Some(y), Some(z)) => (x, y, z),
            _ => return DataFrame::new(),
        };
        let rows: Vec<(f64, f64, f64)> = x_col
            .iter()
            .zip(y_col.iter())
            .zip(z_col.iter())
            .filter_map(|((x, y), z)| Some((x.as_f64()?, y.as_f64()?, z.as_f64()?)))
            .collect();
        if rows.is_empty() {
            return DataFrame::new();
        }

        let x_min = rows.iter().map(|r| r.0).fold(f64::INFINITY, f64::min);
        let x_max = rows.iter().map(|r| r.0).fold(f64::NEG_INFINITY, f64::max);
        let y_min = rows.iter().map(|r| r.1).fold(f64::INFINITY, f64::min);
        let y_max = rows.iter().map(|r| r.1).fold(f64::NEG_INFINITY, f64::max);
        let (x_min, x_max) = if (x_max - x_min).abs() < f64::EPSILON {
            (x_min - 0.5, x_max + 0.5)
        } else {
            (x_min, x_max)
        };
        let (y_min, y_max) = if (y_max - y_min).abs() < f64::EPSILON {
            (y_min - 0.5, y_max + 0.5)
        } else {
            (y_min, y_max)
        };
        let bw_x = (x_max - x_min) / self.bins_x as f64;
        let bw_y = (y_max - y_min) / self.bins_y as f64;

        let mut cells: Vec<Vec<Vec<f64>>> = vec![vec![Vec::new(); self.bins_y]; self.bins_x];
        for &(x, y, z) in &rows {
            let bx = (((x - x_min) / bw_x).floor() as usize).min(self.bins_x - 1);
            let by = (((y - y_min) / bw_y).floor() as usize).min(self.bins_y - 1);
            cells[bx][by].push(z);
        }

        let mut xmin_vals = Vec::new();
        let mut xmax_vals = Vec::new();
        let mut ymin_vals = Vec::new();
        let mut ymax_vals = Vec::new();
        let mut fill_vals = Vec::new();
        for (bx, col) in cells.iter().enumerate() {
            for (by, zs) in col.iter().enumerate() {
                if zs.is_empty() {
                    continue;
                }
                let cell_xmin = x_min + bx as f64 * bw_x;
                let cell_ymin = y_min + by as f64 * bw_y;
                xmin_vals.push(Value::Float(cell_xmin));
                xmax_vals.push(Value::Float(cell_xmin + bw_x));
                ymin_vals.push(Value::Float(cell_ymin));
                ymax_vals.push(Value::Float(cell_ymin + bw_y));
                fill_vals.push(Value::Float(self.fun.apply(zs)));
            }
        }

        let mut result = DataFrame::new();
        result.add_column("xmin".to_string(), xmin_vals);
        result.add_column("xmax".to_string(), xmax_vals);
        result.add_column("ymin".to_string(), ymin_vals);
        result.add_column("ymax".to_string(), ymax_vals);
        result.add_column("fill".to_string(), fill_vals);
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "summary_2d"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarises_z_per_cell() {
        let mut df = DataFrame::new();
        // Two cells: left cluster z≈10, right cluster z≈20.
        let xs = [0.0, 0.1, 0.2, 9.0, 9.1, 9.2];
        let ys = [0.0, 0.1, 0.0, 9.0, 9.1, 9.0];
        let zs = [10.0, 10.0, 10.0, 20.0, 20.0, 20.0];
        df.add_column("x".into(), xs.iter().map(|v| Value::Float(*v)).collect());
        df.add_column("y".into(), ys.iter().map(|v| Value::Float(*v)).collect());
        df.add_column("z".into(), zs.iter().map(|v| Value::Float(*v)).collect());

        let out = StatSummary2d::new(SummaryFun::Mean)
            .with_bins(2, 2)
            .compute_group(&df, &ScaleSet::new());
        let fills: Vec<f64> = out
            .column("fill")
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        assert_eq!(fills.len(), 2);
        assert!(fills.contains(&10.0) && fills.contains(&20.0), "{fills:?}");
    }

    #[test]
    fn missing_z_returns_empty() {
        let mut df = DataFrame::new();
        df.add_column("x".into(), vec![Value::Float(1.0)]);
        df.add_column("y".into(), vec![Value::Float(1.0)]);
        let out = StatSummary2d::default().compute_group(&df, &ScaleSet::new());
        assert_eq!(out.nrows(), 0);
    }
}
