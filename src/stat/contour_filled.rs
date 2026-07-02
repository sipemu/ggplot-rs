use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::contour::build_grid_from_xyz;
use super::Stat;

/// Filled contour bands from gridded (x, y, z) data (R's `stat_contour_filled` /
/// `geom_contour_filled`).
///
/// The domain is resampled onto a grid; each cell is split into two triangles,
/// and each triangle is clipped to every level band `[lo, hi]` (z is linear on a
/// triangle, so the band region is convex). Emits filled polygons with one
/// `group` per polygon and `fill` = the band midpoint — pair with a continuous
/// fill scale via `geom_contour_filled()`.
pub struct StatContourFilled {
    pub bins: usize,
    pub n_bands: usize,
}

impl Default for StatContourFilled {
    fn default() -> Self {
        StatContourFilled {
            bins: 24,
            n_bands: 8,
        }
    }
}

type Vtx = (f64, f64, f64);

/// Sutherland–Hodgman clip of a polygon against the iso-level `thr`, keeping the
/// side where `z >= thr` (or `z <= thr` when `keep_ge` is false). New vertices
/// land exactly on the iso-level.
fn clip(poly: &[Vtx], thr: f64, keep_ge: bool) -> Vec<Vtx> {
    let n = poly.len();
    if n == 0 {
        return Vec::new();
    }
    let inside = |z: f64| if keep_ge { z >= thr } else { z <= thr };
    let mut out = Vec::new();
    for i in 0..n {
        let cur = poly[i];
        let nxt = poly[(i + 1) % n];
        let ci = inside(cur.2);
        let ni = inside(nxt.2);
        if ci {
            out.push(cur);
        }
        if ci != ni {
            let denom = nxt.2 - cur.2;
            let t = if denom.abs() < 1e-12 {
                0.0
            } else {
                (thr - cur.2) / denom
            };
            out.push((
                cur.0 + (nxt.0 - cur.0) * t,
                cur.1 + (nxt.1 - cur.1) * t,
                thr,
            ));
        }
    }
    out
}

impl Stat for StatContourFilled {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (x_col, y_col, z_col) = match (data.column("x"), data.column("y"), data.column("z")) {
            (Some(x), Some(y), Some(z)) => (x, y, z),
            _ => return DataFrame::new(),
        };
        let points: Vec<Vtx> = x_col
            .iter()
            .zip(y_col.iter())
            .zip(z_col.iter())
            .filter_map(|((x, y), z)| {
                let (xv, yv, zv) = (x.as_f64()?, y.as_f64()?, z.as_f64()?);
                (xv.is_finite() && yv.is_finite() && zv.is_finite()).then_some((xv, yv, zv))
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
        if (x_max - x_min).abs() < f64::EPSILON
            || (y_max - y_min).abs() < f64::EPSILON
            || (z_max - z_min).abs() < f64::EPSILON
        {
            return DataFrame::new();
        }

        let nx = self.bins.max(1);
        let ny = self.bins.max(1);
        let dx = (x_max - x_min) / nx as f64;
        let dy = (y_max - y_min) / ny as f64;
        let grid = build_grid_from_xyz(&points, nx, ny, x_min, y_min, dx, dy, z_min, z_max);

        let n_bands = self.n_bands.max(1);
        let step = (z_max - z_min) / n_bands as f64;
        let levels: Vec<f64> = (0..=n_bands).map(|k| z_min + k as f64 * step).collect();

        let mut x_vals = Vec::new();
        let mut y_vals = Vec::new();
        let mut group_vals = Vec::new();
        let mut fill_vals = Vec::new();
        let mut gid: u64 = 0;
        let at = |ix: usize, iy: usize| grid[iy * (nx + 1) + ix];

        for iy in 0..ny {
            for ix in 0..nx {
                let x0 = x_min + ix as f64 * dx;
                let x1 = x0 + dx;
                let y0 = y_min + iy as f64 * dy;
                let y1 = y0 + dy;
                let p00 = (x0, y0, at(ix, iy));
                let p10 = (x1, y0, at(ix + 1, iy));
                let p01 = (x0, y1, at(ix, iy + 1));
                let p11 = (x1, y1, at(ix + 1, iy + 1));

                for tri in [[p00, p10, p11], [p00, p11, p01]] {
                    for k in 0..n_bands {
                        let (lo, hi) = (levels[k], levels[k + 1]);
                        let band = clip(&clip(&tri, lo, true), hi, false);
                        if band.len() >= 3 {
                            let mid = 0.5 * (lo + hi);
                            for v in &band {
                                x_vals.push(Value::Float(v.0));
                                y_vals.push(Value::Float(v.1));
                                group_vals.push(Value::Str(format!("b{gid}")));
                                fill_vals.push(Value::Float(mid));
                            }
                            gid += 1;
                        }
                    }
                }
            }
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result.add_column("group".to_string(), group_vals);
        result.add_column("fill".to_string(), fill_vals);
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "contour_filled"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cone_grid() -> DataFrame {
        // A radial cone z = -sqrt(x^2+y^2) over a grid → nested filled bands.
        let mut df = DataFrame::new();
        let (mut xs, mut ys, mut zs) = (Vec::new(), Vec::new(), Vec::new());
        for i in 0..20 {
            for j in 0..20 {
                let x = i as f64 - 10.0;
                let y = j as f64 - 10.0;
                xs.push(Value::Float(x));
                ys.push(Value::Float(y));
                zs.push(Value::Float(-(x * x + y * y).sqrt()));
            }
        }
        df.add_column("x".into(), xs);
        df.add_column("y".into(), ys);
        df.add_column("z".into(), zs);
        df
    }

    #[test]
    fn produces_filled_bands() {
        let out = StatContourFilled {
            bins: 16,
            n_bands: 5,
        }
        .compute_group(&cone_grid(), &ScaleSet::new());
        assert!(out.nrows() > 0);
        assert!(out.has_column("group") && out.has_column("fill"));
        // Several distinct band fill levels should be present.
        let fills: std::collections::HashSet<String> = out
            .column("fill")
            .unwrap()
            .iter()
            .map(|v| format!("{v:?}"))
            .collect();
        assert!(
            fills.len() >= 3,
            "expected multiple bands, got {}",
            fills.len()
        );
    }

    #[test]
    fn degenerate_z_returns_empty() {
        let mut df = DataFrame::new();
        df.add_column("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]);
        df.add_column("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]);
        df.add_column("z".into(), vec![Value::Float(5.0), Value::Float(5.0)]);
        let out = StatContourFilled::default().compute_group(&df, &ScaleSet::new());
        assert_eq!(out.nrows(), 0);
    }
}
