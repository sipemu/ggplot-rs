//! `StatWaffle`: unit layouts for waffle and parliament charts.
//!
//! Each input row contributes `weight` units (1 when unmapped; rounded to
//! whole units) of its `fill` category; categories keep data order.
//!
//! * Waffle: units fill a grid `rows` high, column by column from the
//!   bottom-left, so each category is a contiguous block.
//! * Parliament: seats sit on concentric half-circle arcs (inner radius
//!   `inner`, outer 1); each arc gets seats in proportion to its length and
//!   categories are assigned left to right by angle.
//!
//! Output: one row per unit with its centre `x`/`y`, `fill` (and every other
//! column of the row it came from), `unit` (half the unit spacing, in data
//! units — the drawn radius) and constant `xmin`/`xmax`/`ymin`/`ymax`
//! framing the layout.

use std::f64::consts::PI;

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Unit layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnitLayout {
    /// Square grid `rows` units high.
    Waffle { rows: usize },
    /// Half-circle seating; `rows` arcs (0 picks a count from the total) with
    /// inner radius `inner` (0–1).
    Parliament { rows: usize, inner: f64 },
}

/// Unit-chart stat behind [`GeomWaffle`](crate::geom::waffle::GeomWaffle).
#[derive(Clone, Debug)]
pub struct StatWaffle {
    pub layout: UnitLayout,
}

impl Default for StatWaffle {
    fn default() -> Self {
        StatWaffle {
            layout: UnitLayout::Waffle { rows: 10 },
        }
    }
}

/// Seats per arc for a parliament of `n` seats on `rows` arcs: proportional
/// to arc radius, rounded so they sum to `n`.
pub(crate) fn seats_per_row(n: usize, rows: usize, inner: f64) -> Vec<usize> {
    let radius = |i: usize| {
        if rows == 1 {
            1.0
        } else {
            inner + (1.0 - inner) * i as f64 / (rows - 1) as f64
        }
    };
    let total_r: f64 = (0..rows).map(radius).sum();
    let exact: Vec<f64> = (0..rows).map(|i| n as f64 * radius(i) / total_r).collect();
    let mut seats: Vec<usize> = exact.iter().map(|e| e.floor() as usize).collect();
    let mut left = n - seats.iter().sum::<usize>();
    // Largest remainders first.
    let mut order: Vec<usize> = (0..rows).collect();
    order.sort_by(|&a, &b| (exact[b] - exact[b].floor()).total_cmp(&(exact[a] - exact[a].floor())));
    for i in order {
        if left == 0 {
            break;
        }
        seats[i] += 1;
        left -= 1;
    }
    seats
}

impl Stat for StatWaffle {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let weight = data.column("weight");
        // Units in category order: (source row) repeated by its weight.
        let fill = data.column("fill");
        let mut cats: indexmap::IndexMap<String, Vec<usize>> = indexmap::IndexMap::new();
        for i in 0..data.nrows() {
            let w = match weight {
                Some(c) => c[i].as_f64().filter(|w| w.is_finite() && *w > 0.0),
                None => Some(1.0),
            };
            let Some(w) = w else { continue };
            let key = fill
                .map(|c| c[i].key_str().into_owned())
                .unwrap_or_default();
            let units = cats.entry(key).or_default();
            units.extend(std::iter::repeat_n(i, w.round() as usize));
        }
        let units: Vec<usize> = cats.into_values().flatten().collect();
        let n = units.len();
        if n == 0 {
            return DataFrame::new();
        }

        let (pos, unit, frame): (Vec<(f64, f64)>, f64, [f64; 4]) = match self.layout {
            UnitLayout::Waffle { rows } => {
                let rows = rows.max(1);
                let cols = n.div_ceil(rows);
                let pos = (0..n)
                    .map(|k| ((k / rows) as f64 + 1.0, (k % rows) as f64 + 1.0))
                    .collect();
                (pos, 0.5, [0.5, cols as f64 + 0.5, 0.5, rows as f64 + 0.5])
            }
            UnitLayout::Parliament { rows, inner } => {
                let inner = inner.clamp(0.05, 0.95);
                let rows = if rows > 0 {
                    rows
                } else {
                    ((n as f64 / 4.0).sqrt().round() as usize).clamp(1, 40)
                };
                let spacing = if rows > 1 {
                    (1.0 - inner) / (rows - 1) as f64
                } else {
                    1.0 - inner
                };
                let mut seats: Vec<(f64, f64, f64)> = Vec::with_capacity(n); // (angle, x, y)
                for (i, k) in seats_per_row(n, rows, inner).into_iter().enumerate() {
                    let r = if rows == 1 {
                        1.0
                    } else {
                        inner + spacing * i as f64
                    };
                    for j in 0..k {
                        let a = if k == 1 {
                            PI / 2.0
                        } else {
                            PI * (1.0 - j as f64 / (k - 1) as f64)
                        };
                        seats.push((a, r * a.cos(), r * a.sin()));
                    }
                }
                // Left to right: decreasing angle, outer rows first on ties.
                seats.sort_by(|a, b| {
                    b.0.total_cmp(&a.0)
                        .then((b.1.hypot(b.2)).total_cmp(&a.1.hypot(a.2)))
                });
                // A seat's radius: half the smaller of row spacing and the
                // outer arc's seat pitch.
                let unit = 0.45 * spacing.min(PI / (n as f64 / rows as f64).max(1.0));
                let pos = seats.into_iter().map(|s| (s.1, s.2)).collect();
                (pos, unit, [-1.0 - unit, 1.0 + unit, -unit, 1.0 + unit])
            }
        };

        let mut out = DataFrame::new();
        out.add_column("x".into(), pos.iter().map(|p| Value::Float(p.0)).collect());
        out.add_column("y".into(), pos.iter().map(|p| Value::Float(p.1)).collect());
        out.add_column("unit".into(), vec![Value::Float(unit); n]);
        for (c, v) in ["xmin", "xmax", "ymin", "ymax"].iter().zip(frame) {
            out.add_column(c.to_string(), vec![Value::Float(v); n]);
        }
        for name in data.column_names() {
            if out.has_column(name) {
                continue;
            }
            let col = data.column(name).unwrap_or_default();
            out.add_column(
                name.to_string(),
                units.iter().map(|&i| col[i].clone()).collect(),
            );
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        Vec::new()
    }

    /// All categories share one grid / chamber.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "waffle"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> DataFrame {
        let mut d = DataFrame::new();
        d.add_column(
            "fill".into(),
            ["A", "B", "C"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        );
        d.add_column(
            "weight".into(),
            [12.0, 7.0, 4.0].iter().map(|&v| Value::Float(v)).collect(),
        );
        d
    }

    #[test]
    fn waffle_fills_columns_bottom_up_in_category_order() {
        let out = StatWaffle {
            layout: UnitLayout::Waffle { rows: 5 },
        }
        .compute_group(&data(), &ScaleSet::new());
        assert_eq!(out.nrows(), 23);
        let x: Vec<f64> = out
            .column("x")
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let y: Vec<f64> = out
            .column("y")
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert_eq!((x[0], y[0]), (1.0, 1.0));
        assert_eq!((x[5], y[5]), (2.0, 1.0));
        assert_eq!(out.column("fill").unwrap()[12], Value::Str("B".into()));
        assert_eq!(
            out.column("xmax").unwrap()[0],
            Value::Float(5.5),
            "ceil(23/5) cols"
        );
    }

    #[test]
    fn parliament_seats_sum_and_sit_on_the_half_disk() {
        assert_eq!(seats_per_row(101, 4, 0.4).iter().sum::<usize>(), 101);
        let out = StatWaffle {
            layout: UnitLayout::Parliament {
                rows: 0,
                inner: 0.4,
            },
        }
        .compute_group(&data(), &ScaleSet::new());
        assert_eq!(out.nrows(), 23);
        for i in 0..23 {
            let (x, y) = (
                out.column("x").unwrap()[i].as_f64().unwrap(),
                out.column("y").unwrap()[i].as_f64().unwrap(),
            );
            let r = x.hypot(y);
            assert!(y >= -1e-9 && (0.39..=1.0 + 1e-9).contains(&r), "{x} {y}");
        }
        // Category A takes the leftmost seats.
        let x0 = out.column("x").unwrap()[0].as_f64().unwrap();
        assert!(x0 < -0.3, "{x0}");
    }
}
