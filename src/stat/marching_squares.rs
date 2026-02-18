use crate::data::Value;

/// Extract contour line segments from a regular grid at specified level thresholds.
/// Returns vectors of (x, y, level, group_id) for each segment endpoint.
#[allow(clippy::too_many_arguments)]
pub fn extract_contours(
    grid: &[f64],
    nx: usize,
    ny: usize,
    x_min: f64,
    y_min: f64,
    dx: f64,
    dy: f64,
    levels: &[f64],
) -> (Vec<Value>, Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut x_vals = Vec::new();
    let mut y_vals = Vec::new();
    let mut level_vals = Vec::new();
    let mut group_vals = Vec::new();
    let mut group_id = 0;

    for &threshold in levels {
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
                    continue;
                }

                let cell_x = x_min + ix as f64 * dx;
                let cell_y = y_min + iy as f64 * dy;

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

    (x_vals, y_vals, level_vals, group_vals)
}

/// Linear interpolation between two values.
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

/// Interpolation fraction for threshold crossing between v0 and v1.
pub fn frac(v0: f64, v1: f64, threshold: f64) -> f64 {
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
pub fn marching_squares_segments(
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
