use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// 2D rectangular binning. Divides x/y ranges into a grid, counts per cell.
pub struct StatBin2d {
    pub bins_x: usize,
    pub bins_y: usize,
}

impl Default for StatBin2d {
    fn default() -> Self {
        StatBin2d {
            bins_x: 30,
            bins_y: 30,
        }
    }
}

impl Stat for StatBin2d {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let xs: Vec<f64> = x_col.iter().filter_map(|v| v.as_f64()).collect();
        let ys: Vec<f64> = y_col.iter().filter_map(|v| v.as_f64()).collect();
        let n = xs.len().min(ys.len());
        if n == 0 {
            return DataFrame::new();
        }

        let x_min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let x_max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let y_min = ys.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

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

        // Match ggplot2's geom_bin2d: width spans the range in `bins - 1` steps,
        // with a bin *edge* aligned to 0 (boundary = 0), and right-closed cells.
        let bw_x = (x_max - x_min) / (self.bins_x.max(2) - 1) as f64;
        let bw_y = (y_max - y_min) / (self.bins_y.max(2) - 1) as f64;
        let (x_origin, nbx) = super::bin::aligned_bins_at(x_min, x_max, bw_x, 0.0);
        let (y_origin, nby) = super::bin::aligned_bins_at(y_min, y_max, bw_y, 0.0);

        let mut counts = vec![vec![0usize; nby]; nbx];

        for i in 0..n {
            let bx =
                (((xs[i] - x_origin) / bw_x).ceil() as i64 - 1).clamp(0, nbx as i64 - 1) as usize;
            let by =
                (((ys[i] - y_origin) / bw_y).ceil() as i64 - 1).clamp(0, nby as i64 - 1) as usize;
            counts[bx][by] += 1;
        }

        let mut xmin_vals = Vec::new();
        let mut xmax_vals = Vec::new();
        let mut ymin_vals = Vec::new();
        let mut ymax_vals = Vec::new();
        let mut fill_vals = Vec::new();

        for (bx, row) in counts.iter().enumerate() {
            for (by, &count) in row.iter().enumerate() {
                if count == 0 {
                    continue;
                }
                let cell_xmin = x_origin + bx as f64 * bw_x;
                let cell_xmax = cell_xmin + bw_x;
                let cell_ymin = y_origin + by as f64 * bw_y;
                let cell_ymax = cell_ymin + bw_y;

                xmin_vals.push(Value::Float(cell_xmin));
                xmax_vals.push(Value::Float(cell_xmax));
                ymin_vals.push(Value::Float(cell_ymin));
                ymax_vals.push(Value::Float(cell_ymax));
                fill_vals.push(Value::Float(count as f64));
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
        "bin2d"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bin2d_basic() {
        let mut data = DataFrame::new();
        let x_vals: Vec<Value> = (0..100).map(|i| Value::Float(i as f64 / 10.0)).collect();
        let y_vals: Vec<Value> = (0..100).map(|i| Value::Float(i as f64 / 5.0)).collect();
        data.add_column("x".to_string(), x_vals);
        data.add_column("y".to_string(), y_vals);

        let stat = StatBin2d {
            bins_x: 5,
            bins_y: 5,
        };
        let scales = ScaleSet::new();
        let result = stat.compute_group(&data, &scales);

        assert!(result.nrows() > 0);
        assert!(result.column("xmin").is_some());
        assert!(result.column("xmax").is_some());
        assert!(result.column("ymin").is_some());
        assert!(result.column("ymax").is_some());
        assert!(result.column("fill").is_some());
    }
}
