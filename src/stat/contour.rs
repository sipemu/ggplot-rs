use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

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

        // Extract data points
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

        // Compute bounds
        let x_min = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x_max = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let y_min = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let y_max = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        let z_min = points.iter().map(|p| p.2).fold(f64::INFINITY, f64::min);
        let z_max = points.iter().map(|p| p.2).fold(f64::NEG_INFINITY, f64::max);

        if (x_max - x_min).abs() < f64::EPSILON || (y_max - y_min).abs() < f64::EPSILON {
            return DataFrame::new();
        }

        // Bin to a regular grid using nearest-neighbor averaging
        let nx = self.bins;
        let ny = self.bins;
        let dx = (x_max - x_min) / nx as f64;
        let dy = (y_max - y_min) / ny as f64;

        let mut grid = vec![0.0f64; (nx + 1) * (ny + 1)];
        let mut counts = vec![0usize; (nx + 1) * (ny + 1)];

        for &(x, y, z) in &points {
            let ix = ((x - x_min) / dx).round() as usize;
            let iy = ((y - y_min) / dy).round() as usize;
            let ix = ix.min(nx);
            let iy = iy.min(ny);
            let idx = iy * (nx + 1) + ix;
            grid[idx] += z;
            counts[idx] += 1;
        }

        // Average and fill empty cells with neighbor interpolation
        for i in 0..grid.len() {
            if counts[i] > 0 {
                grid[i] /= counts[i] as f64;
            }
        }

        // Simple fill for empty cells: use nearest filled neighbor
        for iy in 0..=ny {
            for ix in 0..=nx {
                let idx = iy * (nx + 1) + ix;
                if counts[idx] == 0 {
                    // Find nearest neighbor with data
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

        // Compute contour levels
        let z_range = z_max - z_min;
        if z_range.abs() < f64::EPSILON {
            return DataFrame::new();
        }

        let n_levels = self.n_levels;
        let level_step = z_range / (n_levels + 1) as f64;

        let mut x_vals = Vec::new();
        let mut y_vals = Vec::new();
        let mut level_vals = Vec::new();
        let mut group_vals = Vec::new();
        let mut group_id = 0;

        // Marching squares for each level
        for li in 1..=n_levels {
            let threshold = z_min + li as f64 * level_step;

            // Process each cell
            for iy in 0..ny {
                for ix in 0..nx {
                    let tl = grid[iy * (nx + 1) + ix];
                    let tr = grid[iy * (nx + 1) + ix + 1];
                    let br = grid[(iy + 1) * (nx + 1) + ix + 1];
                    let bl = grid[(iy + 1) * (nx + 1) + ix];

                    let case = ((tl >= threshold) as u8)
                        | (((tr >= threshold) as u8) << 1)
                        | (((br >= threshold) as u8) << 2)
                        | (((bl >= threshold) as u8) << 3);

                    if case == 0 || case == 15 {
                        continue; // No contour in this cell
                    }

                    let cell_x = x_min + ix as f64 * dx;
                    let cell_y = y_min + iy as f64 * dy;

                    // Interpolate edge crossings
                    let segments = marching_squares_segments(
                        case, tl, tr, br, bl, threshold, cell_x, cell_y, dx, dy,
                    );

                    for (p1, p2) in segments {
                        x_vals.push(Value::Float(p1.0));
                        y_vals.push(Value::Float(p1.1));
                        level_vals.push(Value::Float(threshold));
                        group_vals.push(Value::Integer(group_id));

                        x_vals.push(Value::Float(p2.0));
                        y_vals.push(Value::Float(p2.1));
                        level_vals.push(Value::Float(threshold));
                        group_vals.push(Value::Integer(group_id));

                        group_id += 1;
                    }
                }
            }
        }

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

/// Linear interpolation between two values.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

/// Interpolation fraction for threshold crossing between v0 and v1.
fn frac(v0: f64, v1: f64, threshold: f64) -> f64 {
    let denom = v1 - v0;
    if denom.abs() < f64::EPSILON {
        0.5
    } else {
        (threshold - v0) / denom
    }
}

/// Return line segments for a marching squares cell.
/// Corners: tl=top-left, tr=top-right, br=bottom-right, bl=bottom-left.
#[allow(clippy::too_many_arguments)]
fn marching_squares_segments(
    case: u8,
    tl: f64,
    tr: f64,
    br: f64,
    bl: f64,
    threshold: f64,
    cx: f64,
    cy: f64,
    dx: f64,
    dy: f64,
) -> Vec<((f64, f64), (f64, f64))> {
    // Edge midpoints via interpolation
    let top = || {
        let t = frac(tl, tr, threshold);
        (lerp(cx, cx + dx, t), cy)
    };
    let bottom = || {
        let t = frac(bl, br, threshold);
        (lerp(cx, cx + dx, t), cy + dy)
    };
    let left = || {
        let t = frac(tl, bl, threshold);
        (cx, lerp(cy, cy + dy, t))
    };
    let right = || {
        let t = frac(tr, br, threshold);
        (cx + dx, lerp(cy, cy + dy, t))
    };

    match case {
        1 => vec![(top(), left())],
        2 => vec![(top(), right())],
        3 => vec![(left(), right())],
        4 => vec![(right(), bottom())],
        5 => {
            // Saddle point — use average to disambiguate
            let avg = (tl + tr + br + bl) / 4.0;
            if avg >= threshold {
                vec![(top(), right()), (left(), bottom())]
            } else {
                vec![(top(), left()), (right(), bottom())]
            }
        }
        6 => vec![(top(), bottom())],
        7 => vec![(left(), bottom())],
        8 => vec![(left(), bottom())],
        9 => vec![(top(), bottom())],
        10 => {
            // Saddle point
            let avg = (tl + tr + br + bl) / 4.0;
            if avg >= threshold {
                vec![(top(), left()), (right(), bottom())]
            } else {
                vec![(top(), right()), (left(), bottom())]
            }
        }
        11 => vec![(right(), bottom())],
        12 => vec![(left(), right())],
        13 => vec![(top(), right())],
        14 => vec![(top(), left())],
        _ => vec![],
    }
}
