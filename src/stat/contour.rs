use crate::aes::Aesthetic;
use crate::data::DataFrame;
use crate::scale::ScaleSet;

use super::marching_squares::extract_contours;
use super::Stat;

/// Compute contour lines from gridded (x, y, z) data using Marching Squares.
pub struct StatContour {
    pub bins: usize,
    pub n_levels: usize,
}

impl Default for StatContour {
    fn default() -> Self {
        StatContour {
            bins: 50,
            n_levels: 10,
        }
    }
}

impl Stat for StatContour {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let z_col = match data.column("z") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let points: Vec<(f64, f64, f64)> = x_col
            .iter()
            .zip(y_col.iter())
            .zip(z_col.iter())
            .filter_map(|((x, y), z)| match (x.as_f64(), y.as_f64(), z.as_f64()) {
                (Some(xv), Some(yv), Some(zv))
                    if xv.is_finite() && yv.is_finite() && zv.is_finite() =>
                {
                    Some((xv, yv, zv))
                }
                _ => None,
            })
            .collect();

        if points.is_empty() {
            return DataFrame::new();
        }

        let x_min = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x_max = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let y_min = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let y_max = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        let z_min = points.iter().map(|p| p.2).fold(f64::INFINITY, f64::min);
        let z_max = points.iter().map(|p| p.2).fold(f64::NEG_INFINITY, f64::max);

        if (x_max - x_min).abs() < f64::EPSILON || (y_max - y_min).abs() < f64::EPSILON {
            return DataFrame::new();
        }

        let nx = self.bins;
        let ny = self.bins;
        let dx = (x_max - x_min) / nx as f64;
        let dy = (y_max - y_min) / ny as f64;

        let grid = build_grid_from_xyz(&points, nx, ny, x_min, y_min, dx, dy, z_min, z_max);

        let z_range = z_max - z_min;
        if z_range.abs() < f64::EPSILON {
            return DataFrame::new();
        }

        let n_levels = self.n_levels;
        let level_step = z_range / (n_levels + 1) as f64;
        let levels: Vec<f64> = (1..=n_levels)
            .map(|li| z_min + li as f64 * level_step)
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
        "contour"
    }
}

/// Build a regular grid from scattered (x, y, z) points via nearest-neighbor binning.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_grid_from_xyz(
    points: &[(f64, f64, f64)],
    nx: usize,
    ny: usize,
    x_min: f64,
    y_min: f64,
    dx: f64,
    dy: f64,
    z_min: f64,
    z_max: f64,
) -> Vec<f64> {
    let mut grid = vec![0.0f64; (nx + 1) * (ny + 1)];
    let mut counts = vec![0usize; (nx + 1) * (ny + 1)];

    for &(x, y, z) in points {
        let ix = ((x - x_min) / dx).round() as usize;
        let iy = ((y - y_min) / dy).round() as usize;
        let ix = ix.min(nx);
        let iy = iy.min(ny);
        let idx = iy * (nx + 1) + ix;
        grid[idx] += z;
        counts[idx] += 1;
    }

    for i in 0..grid.len() {
        if counts[i] > 0 {
            grid[i] /= counts[i] as f64;
        }
    }

    // Fill empty cells with nearest filled neighbor
    for iy in 0..=ny {
        for ix in 0..=nx {
            let idx = iy * (nx + 1) + ix;
            if counts[idx] == 0 {
                let mut best_dist = f64::INFINITY;
                let mut best_val = (z_min + z_max) / 2.0;
                for dy2 in -3i32..=3 {
                    for dx2 in -3i32..=3 {
                        let nx2 = ix as i32 + dx2;
                        let ny2 = iy as i32 + dy2;
                        if nx2 >= 0 && nx2 <= nx as i32 && ny2 >= 0 && ny2 <= ny as i32 {
                            let idx2 = ny2 as usize * (nx + 1) + nx2 as usize;
                            if counts[idx2] > 0 {
                                let d = ((dx2 * dx2 + dy2 * dy2) as f64).sqrt();
                                if d < best_dist {
                                    best_dist = d;
                                    best_val = grid[idx2];
                                }
                            }
                        }
                    }
                }
                grid[idx] = best_val;
            }
        }
    }

    grid
}
