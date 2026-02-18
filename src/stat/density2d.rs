use crate::aes::Aesthetic;
use crate::data::DataFrame;
use crate::scale::ScaleSet;

use super::marching_squares::extract_contours;
use super::Stat;

/// Compute 2D density contour lines from (x, y) point data.
/// Uses Gaussian 2D KDE with Silverman bandwidth selection.
pub struct StatDensity2d {
    pub n_grid: usize,
    pub n_levels: usize,
}

impl Default for StatDensity2d {
    fn default() -> Self {
        StatDensity2d {
            n_grid: 50,
            n_levels: 10,
        }
    }
}

impl StatDensity2d {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_grid(mut self, n: usize) -> Self {
        self.n_grid = n;
        self
    }

    pub fn with_levels(mut self, n: usize) -> Self {
        self.n_levels = n;
        self
    }
}

impl Stat for StatDensity2d {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        // Extract finite (x, y) points
        let points: Vec<(f64, f64)> = x_col
            .iter()
            .zip(y_col.iter())
            .filter_map(|(x, y)| match (x.as_f64(), y.as_f64()) {
                (Some(xv), Some(yv)) if xv.is_finite() && yv.is_finite() => Some((xv, yv)),
                _ => None,
            })
            .collect();

        if points.len() < 2 {
            return DataFrame::new();
        }

        let n = points.len() as f64;

        // Compute means and standard deviations
        let x_mean = points.iter().map(|p| p.0).sum::<f64>() / n;
        let y_mean = points.iter().map(|p| p.1).sum::<f64>() / n;
        let x_var = points.iter().map(|p| (p.0 - x_mean).powi(2)).sum::<f64>() / (n - 1.0);
        let y_var = points.iter().map(|p| (p.1 - y_mean).powi(2)).sum::<f64>() / (n - 1.0);
        let x_sd = x_var.sqrt();
        let y_sd = y_var.sqrt();

        if x_sd < f64::EPSILON || y_sd < f64::EPSILON {
            return DataFrame::new();
        }

        // 2D Silverman bandwidth: bw = sd * n^(-1/6)
        let bw_x = x_sd * n.powf(-1.0 / 6.0);
        let bw_y = y_sd * n.powf(-1.0 / 6.0);

        // Compute grid bounds (extend by 3*bw beyond data range)
        let x_min = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min) - 3.0 * bw_x;
        let x_max = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max) + 3.0 * bw_x;
        let y_min = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min) - 3.0 * bw_y;
        let y_max = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max) + 3.0 * bw_y;

        let nx = self.n_grid;
        let ny = self.n_grid;
        let dx = (x_max - x_min) / nx as f64;
        let dy = (y_max - y_min) / ny as f64;

        // Build density grid via 2D Gaussian KDE
        let mut grid = vec![0.0f64; (nx + 1) * (ny + 1)];
        let inv_2bwx2 = 1.0 / (2.0 * bw_x * bw_x);
        let inv_2bwy2 = 1.0 / (2.0 * bw_y * bw_y);
        let norm = 1.0 / (2.0 * std::f64::consts::PI * bw_x * bw_y * n);

        for iy in 0..=ny {
            let gy = y_min + iy as f64 * dy;
            for ix in 0..=nx {
                let gx = x_min + ix as f64 * dx;
                let mut density = 0.0;
                for &(px, py) in &points {
                    let dx2 = (gx - px) * (gx - px) * inv_2bwx2;
                    let dy2 = (gy - py) * (gy - py) * inv_2bwy2;
                    density += (-dx2 - dy2).exp();
                }
                grid[iy * (nx + 1) + ix] = density * norm;
            }
        }

        // Determine density range for contour levels
        let d_min = grid.iter().copied().fold(f64::INFINITY, f64::min);
        let d_max = grid.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let d_range = d_max - d_min;

        if d_range.abs() < f64::EPSILON {
            return DataFrame::new();
        }

        let n_levels = self.n_levels;
        let levels: Vec<f64> = (1..=n_levels)
            .map(|li| d_min + li as f64 * d_range / (n_levels + 1) as f64)
            .collect();

        let (x_vals, y_vals, level_vals, group_vals) =
            extract_contours(&grid, nx, ny, x_min, y_min, dx, dy, &levels);

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result.add_column("level".to_string(), level_vals);
        result.add_column("group".to_string(), group_vals);
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "density_2d"
    }
}
