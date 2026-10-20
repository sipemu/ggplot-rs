use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// A pointy-top hex lattice in data units: points are assigned to the
/// nearest centre via cube rounding, so bins are fixed by `(w, h, origin)`
/// alone — unlike floor-binned rectangles, they don't move when the data
/// extent changes.
///
/// `w`/`h` are the horizontal/vertical centre-to-centre spacing; equal
/// `w == h` gives a regular hexagon, independent values stretch it (as
/// ggplot2's `hexbin` does for non-square bin counts).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HexLattice {
    pub w: f64,
    pub h: f64,
    pub origin: (f64, f64),
}

impl HexLattice {
    pub fn new(w: f64, h: f64, origin: (f64, f64)) -> Self {
        HexLattice { w, h, origin }
    }

    /// The centre of cell `(q, r)` in data units: `x = ox + w*(q + r/2)`,
    /// `y = oy + r*h*sqrt(3)/2`.
    pub fn centre(&self, q: i64, r: i64) -> (f64, f64) {
        let x = self.origin.0 + self.w * (q as f64 + r as f64 / 2.0);
        let y = self.origin.1 + r as f64 * self.h * SQRT_3 / 2.0;
        (x, y)
    }

    /// The 6 vertices of cell `(q, r)`, pointy-top, in data units.
    pub fn vertices(&self, q: i64, r: i64) -> [(f64, f64); 6] {
        let (cx, cy) = self.centre(q, r);
        let radius = 1.0 / SQRT_3;
        std::array::from_fn(|k| {
            let angle = std::f64::consts::FRAC_PI_2 + std::f64::consts::FRAC_PI_3 * k as f64;
            (
                cx + radius * self.w * angle.cos(),
                cy + radius * self.h * angle.sin(),
            )
        })
    }

    /// The cell nearest `(x, y)`, via cube rounding on normalized axial
    /// coordinates (the inverse of [`centre`](Self::centre)).
    pub fn locate(&self, x: f64, y: f64) -> (i64, i64) {
        let r_frac = (y - self.origin.1) * 2.0 / (self.h * SQRT_3);
        let q_frac = (x - self.origin.0) / self.w - r_frac / 2.0;
        let cx = q_frac;
        let cz = r_frac;
        let cy = -cx - cz;
        let (rx, _ry, rz) = cube_round(cx, cy, cz);
        (rx as i64, rz as i64)
    }
}

const SQRT_3: f64 = 1.732_050_807_568_877_2;

fn cube_round(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let mut rx = x.round();
    let mut ry = y.round();
    let mut rz = z.round();
    let dx = (rx - x).abs();
    let dy = (ry - y).abs();
    let dz = (rz - z).abs();
    if dx > dy && dx > dz {
        rx = -ry - rz;
    } else if dy > dz {
        ry = -rx - rz;
    } else {
        rz = -rx - ry;
    }
    (rx, ry, rz)
}

fn minmax(xs: &[f64]) -> (f64, f64) {
    let min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (min, max)
}

/// Hexagonal binning on a fixed hex lattice (data units), unlike the
/// floor-binned staggered rectangles this replaces (see the 0.20.0
/// CHANGELOG). Output: `x, y` (hex centres), `count`, `density`, `ncount`,
/// `ndensity`, `width`, `height`, `fill` (= `count`, back-compat), `n`
/// (unweighted point count), `q`, `r`.
pub struct StatBinHex {
    pub bins_x: usize,
    pub bins_y: usize,
    pub binwidth: Option<(f64, f64)>,
    pub origin: Option<(f64, f64)>,
}

impl Default for StatBinHex {
    fn default() -> Self {
        StatBinHex {
            bins_x: 30,
            bins_y: 30,
            binwidth: None,
            origin: None,
        }
    }
}

impl StatBinHex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Same bin count on both axes.
    pub fn bins(mut self, n: usize) -> Self {
        self.bins_x = n;
        self.bins_y = n;
        self
    }

    pub fn bins_xy(mut self, x: usize, y: usize) -> Self {
        self.bins_x = x;
        self.bins_y = y;
        self
    }

    /// Fix the bin width/height in data units (overrides `bins`).
    pub fn binwidth(mut self, w: f64, h: f64) -> Self {
        self.binwidth = Some((w, h));
        self
    }

    /// Fix the lattice origin in data units.
    pub fn origin(mut self, x: f64, y: f64) -> Self {
        self.origin = Some((x, y));
        self
    }

    /// Build the [`HexLattice`] this stat would use for `(xs, ys)`, following
    /// the origin rules:
    /// - `binwidth` and `origin` both given: the lattice is fixed exactly.
    /// - `binwidth` only: anchored at `(0, 0)`.
    /// - neither: `w, h` from the range and bin counts, anchored at the data
    ///   minimum (ggplot2) — or at `origin`, if given without `binwidth`.
    pub fn lattice(&self, xs: &[f64], ys: &[f64]) -> HexLattice {
        if let Some((w, h)) = self.binwidth {
            return HexLattice::new(w, h, self.origin.unwrap_or((0.0, 0.0)));
        }
        let (x_min, x_max) = minmax(xs);
        let (y_min, y_max) = minmax(ys);
        let x_range = if (x_max - x_min).abs() < f64::EPSILON {
            1.0
        } else {
            x_max - x_min
        };
        let y_range = if (y_max - y_min).abs() < f64::EPSILON {
            1.0
        } else {
            y_max - y_min
        };
        let w = x_range / self.bins_x.max(1) as f64;
        let h = y_range / self.bins_y.max(1) as f64;
        let origin = self.origin.unwrap_or((x_min, y_min));
        HexLattice::new(w, h, origin)
    }

    /// The cell `(q, r)` nearest `(x, y)` on `lattice`.
    pub fn locate(x: f64, y: f64, lattice: &HexLattice) -> (i64, i64) {
        lattice.locate(x, y)
    }
}

impl Stat for StatBinHex {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let weight_col = data.column("weight");

        let n_rows = x_col.len().min(y_col.len());
        let mut xs = Vec::with_capacity(n_rows);
        let mut ys = Vec::with_capacity(n_rows);
        let mut weights = Vec::with_capacity(n_rows);
        for i in 0..n_rows {
            let (Some(x), Some(y)) = (x_col[i].as_f64(), y_col[i].as_f64()) else {
                continue;
            };
            if !(x.is_finite() && y.is_finite()) {
                continue;
            }
            xs.push(x);
            ys.push(y);
            weights.push(weight_col.and_then(|wc| wc[i].as_f64()));
        }
        if xs.is_empty() {
            return DataFrame::new();
        }

        let lattice = self.lattice(&xs, &ys);

        // Row-major (r, q) key order: deterministic, and y never decreases
        // along the output (matches the pre-0.20 convention).
        let mut cells: std::collections::BTreeMap<(i64, i64), (f64, usize)> =
            std::collections::BTreeMap::new();
        for i in 0..xs.len() {
            let (q, r) = lattice.locate(xs[i], ys[i]);
            // No `weight` column: every point counts as weight 1 (ggplot2's
            // default). A `weight` column that evaluates to non-finite for
            // this row contributes 0 but still creates the cell (D8).
            let w = if weight_col.is_none() {
                1.0
            } else {
                match weights[i] {
                    Some(w) if w.is_finite() => w,
                    _ => 0.0,
                }
            };
            let entry = cells.entry((r, q)).or_insert((0.0, 0));
            entry.0 += w;
            entry.1 += 1;
        }

        let total_count: f64 = cells.values().map(|&(c, _)| c).sum();
        let max_count = cells.values().map(|&(c, _)| c).fold(0.0, f64::max);

        let mut x_vals = Vec::new();
        let mut y_vals = Vec::new();
        let mut count_vals = Vec::new();
        let mut density_vals = Vec::new();
        let mut ncount_vals = Vec::new();
        let mut ndensity_vals = Vec::new();
        let mut width_vals = Vec::new();
        let mut height_vals = Vec::new();
        let mut fill_vals = Vec::new();
        let mut n_vals = Vec::new();
        let mut q_vals = Vec::new();
        let mut r_vals = Vec::new();

        for (&(r, q), &(count, n)) in &cells {
            let (cx, cy) = lattice.centre(q, r);
            let density = if total_count > 0.0 {
                count / total_count
            } else {
                0.0
            };
            // density is a constant multiple of count (1/total_count), so
            // its own max-normalized form is identical to ncount.
            let ncount = if max_count > 0.0 {
                count / max_count
            } else {
                0.0
            };
            let ndensity = ncount;

            x_vals.push(Value::Float(cx));
            y_vals.push(Value::Float(cy));
            count_vals.push(Value::Float(count));
            density_vals.push(Value::Float(density));
            ncount_vals.push(Value::Float(ncount));
            ndensity_vals.push(Value::Float(ndensity));
            width_vals.push(Value::Float(lattice.w));
            height_vals.push(Value::Float(lattice.h));
            fill_vals.push(Value::Float(count));
            n_vals.push(Value::Integer(n as i64));
            q_vals.push(Value::Integer(q));
            r_vals.push(Value::Integer(r));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result.add_column("count".to_string(), count_vals);
        result.add_column("density".to_string(), density_vals);
        result.add_column("ncount".to_string(), ncount_vals);
        result.add_column("ndensity".to_string(), ndensity_vals);
        result.add_column("width".to_string(), width_vals);
        result.add_column("height".to_string(), height_vals);
        result.add_column("fill".to_string(), fill_vals);
        result.add_column("n".to_string(), n_vals);
        result.add_column("q".to_string(), q_vals);
        result.add_column("r".to_string(), r_vals);

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "binhex"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::cartesian::CoordCartesian;
    use crate::coord::Coord;
    use crate::render::Rect;

    fn col_f64(df: &DataFrame, name: &str) -> Vec<f64> {
        df.column(name)
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    }

    #[test]
    fn test_binhex_basic() {
        let mut data = DataFrame::new();
        let x_vals: Vec<Value> = (0..100).map(|i| Value::Float(i as f64 / 10.0)).collect();
        let y_vals: Vec<Value> = (0..100).map(|i| Value::Float(i as f64 / 5.0)).collect();
        data.add_column("x".to_string(), x_vals);
        data.add_column("y".to_string(), y_vals);

        let stat = StatBinHex::new().bins_xy(5, 5);
        let scales = ScaleSet::new();
        let result = stat.compute_group(&data, &scales);

        assert!(result.nrows() > 0);
        for col in [
            "x", "y", "count", "density", "ncount", "ndensity", "width", "height", "fill", "n",
            "q", "r",
        ] {
            assert!(result.column(col).is_some(), "missing column {col}");
        }
    }

    #[test]
    fn output_order_is_deterministic_and_conserves_the_total() {
        let mut data = DataFrame::new();
        let x: Vec<Value> = (0..500)
            .map(|i| Value::Float(((i * 37) % 101) as f64))
            .collect();
        let y: Vec<Value> = (0..500)
            .map(|i| Value::Float(((i * 53) % 97) as f64))
            .collect();
        data.add_column("x".to_string(), x);
        data.add_column("y".to_string(), y);
        let stat = StatBinHex::new().bins_xy(12, 12);
        let scales = ScaleSet::new();
        let a = stat.compute_group(&data, &scales);
        for _ in 0..5 {
            let b = stat.compute_group(&data, &scales);
            assert_eq!(a.column("x"), b.column("x"));
            assert_eq!(a.column("count"), b.column("count"));
            assert_eq!(a.column("q"), b.column("q"));
            assert_eq!(a.column("r"), b.column("r"));
        }
        let ys = col_f64(&a, "y");
        assert!(ys.windows(2).all(|w| w[0] <= w[1]));
        let total: f64 = col_f64(&a, "count").iter().sum();
        assert_eq!(total, 500.0);
        let n_total: i64 = a
            .column("n")
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap() as i64)
            .sum();
        assert_eq!(n_total, 500);
    }

    /// 1. Points land in their nearest centre.
    #[test]
    fn points_land_in_their_nearest_centre() {
        let lattice = HexLattice::new(1.0, 1.0, (0.0, 0.0));
        for q in -3..=3 {
            for r in -3..=3 {
                let (cx, cy) = lattice.centre(q, r);
                assert_eq!(lattice.locate(cx, cy), (q, r));
            }
        }
    }

    /// 2. D7: the cells shared with a superset of the data have identical centres.
    #[test]
    fn fixed_binwidth_and_origin_give_identical_centres_on_a_superset() {
        let mut small = DataFrame::new();
        small.add_column(
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(1.5)],
        );
        small.add_column(
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(1.5)],
        );

        let mut big = DataFrame::new();
        big.add_column(
            "x".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(1.5),
                Value::Float(50.0),
                Value::Float(-50.0),
            ],
        );
        big.add_column(
            "y".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(1.5),
                Value::Float(50.0),
                Value::Float(-50.0),
            ],
        );

        let stat = StatBinHex::new().binwidth(1.0, 1.0).origin(0.0, 0.0);
        let scales = ScaleSet::new();
        let a = stat.compute_group(&small, &scales);
        let b = stat.compute_group(&big, &scales);

        let centres = |df: &DataFrame| -> std::collections::HashMap<(i64, i64), (f64, f64)> {
            df.column("q")
                .unwrap()
                .iter()
                .zip(df.column("r").unwrap().iter())
                .zip(df.column("x").unwrap().iter())
                .zip(df.column("y").unwrap().iter())
                .map(|(((q, r), x), y)| {
                    (
                        (q.as_f64().unwrap() as i64, r.as_f64().unwrap() as i64),
                        (x.as_f64().unwrap(), y.as_f64().unwrap()),
                    )
                })
                .collect()
        };
        let a_centres = centres(&a);
        let b_centres = centres(&b);
        // Every cell `small` produced is also produced by `big` (a superset
        // of the same data), with an identical centre — the lattice is
        // fixed by binwidth+origin, not the data extent.
        for (key, centre) in &a_centres {
            let b_centre = b_centres
                .get(key)
                .unwrap_or_else(|| panic!("{key:?} missing from the superset"));
            assert_eq!(centre, b_centre);
        }
    }

    /// 3. D8: weights `[2, NaN, 3]` give count = 5, n = 3; an all-NaN-weight
    ///    cell has count = 0.
    #[test]
    fn weights_sum_with_non_finite_as_zero_but_still_create_the_cell() {
        let mut data = DataFrame::new();
        data.add_column(
            "x".to_string(),
            vec![Value::Float(0.0), Value::Float(0.0), Value::Float(0.0)],
        );
        data.add_column(
            "y".to_string(),
            vec![Value::Float(0.0), Value::Float(0.0), Value::Float(0.0)],
        );
        data.add_column(
            "weight".to_string(),
            vec![Value::Float(2.0), Value::Float(f64::NAN), Value::Float(3.0)],
        );
        let stat = StatBinHex::new().binwidth(1.0, 1.0).origin(0.0, 0.0);
        let scales = ScaleSet::new();
        let result = stat.compute_group(&data, &scales);
        assert_eq!(result.nrows(), 1);
        assert_eq!(result.column("count").unwrap()[0].as_f64(), Some(5.0));
        assert_eq!(result.column("n").unwrap()[0].as_f64(), Some(3.0));

        // An all-NaN-weight cell still exists, with count = 0.
        let mut nan_only = DataFrame::new();
        nan_only.add_column("x".to_string(), vec![Value::Float(0.0)]);
        nan_only.add_column("y".to_string(), vec![Value::Float(0.0)]);
        nan_only.add_column("weight".to_string(), vec![Value::Float(f64::NAN)]);
        let result2 = stat.compute_group(&nan_only, &scales);
        assert_eq!(result2.nrows(), 1);
        assert_eq!(result2.column("count").unwrap()[0].as_f64(), Some(0.0));
        assert_eq!(result2.column("n").unwrap()[0].as_f64(), Some(1.0));
    }

    /// 5. Under coord_sf with square data, the hex edges are equal within 0.05 px.
    #[test]
    fn hex_edges_are_equal_under_identity_coord() {
        let lattice = HexLattice::new(1.0, 1.0, (0.0, 0.0));
        let coord = CoordCartesian::default();
        let plot_area = Rect {
            x: 0.0,
            y: 0.0,
            width: 1000.0,
            height: 1000.0,
        };
        // Map data-space vertices through an identity-like transform (same
        // scale factor in x and y) and check the resulting polygon's edges
        // are equal, as a regular hexagon's should be.
        let verts_data = lattice.vertices(0, 0);
        let verts_px: Vec<(f64, f64)> = verts_data
            .iter()
            .map(|&(x, y)| coord.transform((x / 100.0 + 0.5, y / 100.0 + 0.5), &plot_area))
            .collect();
        let edge_len =
            |a: (f64, f64), b: (f64, f64)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        let edges: Vec<f64> = (0..6)
            .map(|i| edge_len(verts_px[i], verts_px[(i + 1) % 6]))
            .collect();
        let first = edges[0];
        for e in &edges {
            assert!((e - first).abs() < 0.05, "edges should be equal: {edges:?}");
        }
    }

    /// 6. `locate()` agrees with `compute_group` on 500 random points.
    #[test]
    fn locate_agrees_with_compute_group() {
        let mut rng_state: u64 = 42;
        let mut next = move || {
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            (rng_state % 10000) as f64 / 100.0 - 50.0
        };
        let xs: Vec<f64> = (0..500).map(|_| next()).collect();
        let ys: Vec<f64> = (0..500).map(|_| next()).collect();

        let mut data = DataFrame::new();
        data.add_column(
            "x".to_string(),
            xs.iter().map(|&v| Value::Float(v)).collect(),
        );
        data.add_column(
            "y".to_string(),
            ys.iter().map(|&v| Value::Float(v)).collect(),
        );

        let stat = StatBinHex::new().bins_xy(15, 15);
        let scales = ScaleSet::new();
        let lattice = stat.lattice(&xs, &ys);
        let result = stat.compute_group(&data, &scales);

        let produced_qr: std::collections::HashSet<(i64, i64)> = result
            .column("q")
            .unwrap()
            .iter()
            .zip(result.column("r").unwrap().iter())
            .map(|(q, r)| (q.as_f64().unwrap() as i64, r.as_f64().unwrap() as i64))
            .collect();

        for i in 0..xs.len() {
            let (q, r) = StatBinHex::locate(xs[i], ys[i], &lattice);
            assert!(
                produced_qr.contains(&(q, r)),
                "locate({}, {}) = ({q}, {r}) not in compute_group's cells",
                xs[i],
                ys[i]
            );
        }
    }
}
