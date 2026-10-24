use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// ggplot2 bin alignment: place bins so an edge falls on `boundary` (mod
/// `width`), then return the left edge of the first bin (`origin`, ≤ `min`) and
/// the number of bins needed to cover `[min, max]`.
pub(crate) fn aligned_bins_at(min: f64, max: f64, width: f64, boundary: f64) -> (f64, usize) {
    let shift = ((min - boundary) / width).floor();
    let origin = boundary + shift * width;
    let n = (((max - origin) / width).ceil() as usize).max(1);
    (origin, n)
}

/// 1-D histogram alignment: a *bin* is centered on 0 (`boundary = width/2`),
/// matching `geom_histogram`.
fn aligned_bins(min: f64, max: f64, width: f64) -> (f64, usize) {
    aligned_bins_at(min, max, width, width / 2.0)
}

/// Bins continuous x values into histogram bins.
pub struct StatBin {
    pub bins: usize,
    pub binwidth: Option<f64>,
}

impl StatBin {
    /// Set bin width (overrides bins count).
    pub fn with_binwidth(mut self, width: f64) -> Self {
        self.binwidth = Some(width);
        self
    }

    /// Set number of bins.
    pub fn with_bins(mut self, bins: usize) -> Self {
        self.bins = bins;
        self.binwidth = None;
        self
    }
}

impl Default for StatBin {
    fn default() -> Self {
        StatBin {
            bins: 30,
            binwidth: None,
        }
    }
}

impl Stat for StatBin {
    /// #197: `panelwise` (every group's rows arrive together, grouped only
    /// by facet) so bins are computed once over the whole panel — ggplot2's
    /// own `stat_bin` behaviour — then counted per `aes(fill=)`/`aes(color=)`
    /// group. Ungrouped input (the common case: no fill/color/group aes) is
    /// just one implicit group, so this is byte-identical to before #197.
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let values: Vec<f64> = x_col.iter().filter_map(|v| v.as_f64()).collect();
        if values.is_empty() {
            return DataFrame::new();
        }

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Handle case where all values are the same
        let (min, max) = if (max - min).abs() < f64::EPSILON {
            (min - 0.5, max + 0.5)
        } else {
            (min, max)
        };

        // Match ggplot2's bin_breaks: for a bin count, width spans the range in
        // `bins - 1` steps; bins are then aligned to `boundary = width/2` so a
        // bin is centered on 0 (the origin is shifted left of the data min),
        // rather than starting exactly at the data minimum.
        let (bin_width, origin, n_bins) = if let Some(bw) = self.binwidth {
            let (o, n) = aligned_bins(min, max, bw);
            (bw, o, n)
        } else if self.bins <= 1 {
            (max - min, min, 1)
        } else {
            let bw = (max - min) / (self.bins - 1) as f64;
            let (o, n) = aligned_bins(min, max, bw);
            (bw, o, n)
        };
        // ggplot2's default bins are right-closed: (a, b]. A point on a
        // boundary falls in the lower bin.
        let bin_of = |v: f64| -> usize {
            let raw = ((v - origin) / bin_width).ceil() as i64 - 1;
            raw.clamp(0, n_bins as i64 - 1) as usize
        };

        // Rows grouped by every column but `x` (the aes `fill`/`color`/
        // `group` columns, if any) — first-appearance order, as `StatAlign`
        // groups a panel. A plain, ungrouped histogram has no such columns,
        // so every row joins the one group keyed `""`.
        let group_cols: Vec<String> = crate::build::detect_group_columns(data)
            .into_iter()
            .filter(|c| c != "x")
            .collect();
        let mut order: Vec<String> = Vec::new();
        let mut first_row: std::collections::HashMap<String, usize> = Default::default();
        let mut counts: std::collections::HashMap<String, Vec<usize>> = Default::default();
        for (i, x) in x_col.iter().enumerate() {
            let Some(v) = x.as_f64() else {
                continue;
            };
            let key: String = group_cols
                .iter()
                .map(|c| data.column(c).unwrap()[i].to_group_key())
                .collect::<Vec<_>>()
                .join("\u{1f}");
            if !first_row.contains_key(&key) {
                order.push(key.clone());
                first_row.insert(key.clone(), i);
            }
            counts.entry(key).or_insert_with(|| vec![0usize; n_bins])[bin_of(v)] += 1;
        }

        // Carry every other input column (fill/color/group, and anything
        // else the caller mapped) from the group's first row onto all of
        // that group's output rows — the same idiom `StatAlign`/`StatQQ`
        // use to keep a layer's grouping visible after the stat runs.
        // Excludes this stat's own output columns, not just `x`: aes
        // resolution can leave the *source* column (e.g. a data column
        // literally named "y", mapped to the x aesthetic) sitting
        // alongside the renamed one, and carrying it forward here would
        // silently overwrite the real computed "y"/"count"/etc. with that
        // unrelated pre-stat column's value.
        const OUTPUT_COLS: [&str; 6] = ["x", "y", "count", "density", "xmin", "xmax"];
        let carry: Vec<String> = data
            .column_names()
            .into_iter()
            .filter(|n| !OUTPUT_COLS.contains(n))
            .map(str::to_string)
            .collect();

        let mut x_vals = Vec::new();
        let mut y_vals = Vec::new();
        let mut density_vals = Vec::new();
        let mut xmin_vals = Vec::new();
        let mut xmax_vals = Vec::new();
        let mut carried: Vec<Vec<Value>> = vec![Vec::new(); carry.len()];
        // One row per (group, bin) — every bin for every group, so a
        // sparse group's empty bins still draw a zero-height bar/point
        // instead of vanishing (ggplot2's own stat_bin behaviour).
        for key in &order {
            let group_counts = &counts[key];
            let total = group_counts.iter().sum::<usize>() as f64;
            let first = first_row[key];
            for (i, &count) in group_counts.iter().enumerate() {
                let bin_min = origin + i as f64 * bin_width;
                let bin_max = bin_min + bin_width;
                x_vals.push(Value::Float((bin_min + bin_max) / 2.0));
                y_vals.push(Value::Float(count as f64));
                density_vals.push(Value::Float(if total > 0.0 {
                    count as f64 / (total * bin_width)
                } else {
                    0.0
                }));
                xmin_vals.push(Value::Float(bin_min));
                xmax_vals.push(Value::Float(bin_max));
                for (c, name) in carry.iter().enumerate() {
                    carried[c].push(data.column(name).unwrap()[first].clone());
                }
            }
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals.clone());
        // Expose the count under its ggplot stat name for after_stat expressions
        // (e.g. after_stat_y("count / sum(count)")).
        result.add_column("count".to_string(), y_vals);
        result.add_column("density".to_string(), density_vals);
        result.add_column("xmin".to_string(), xmin_vals);
        result.add_column("xmax".to_string(), xmax_vals);
        for (name, vals) in carry.into_iter().zip(carried) {
            result.add_column(name, vals);
        }
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "bin"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::ScaleSet;

    fn ungrouped(values: &[f64]) -> DataFrame {
        let mut df = DataFrame::new();
        df.add_column(
            "x".into(),
            values.iter().map(|v| Value::Float(*v)).collect(),
        );
        df
    }

    fn grouped(values: &[f64], groups: &[&str]) -> DataFrame {
        let mut df = DataFrame::new();
        df.add_column(
            "x".into(),
            values.iter().map(|v| Value::Float(*v)).collect(),
        );
        df.add_column(
            "fill".into(),
            groups.iter().map(|g| Value::Str((*g).into())).collect(),
        );
        df
    }

    #[test]
    fn panelwise_is_true() {
        assert!(StatBin::default().panelwise());
    }

    #[test]
    fn ungrouped_output_is_unchanged_by_197() {
        // #197's grouped path reduces to exactly this for a single implicit
        // group: same column set, same values, no "fill"/"color" columns.
        let data = ungrouped(&[1.0, 1.0, 2.0, 2.0, 2.0, 3.0]);
        let out = StatBin {
            bins: 3,
            binwidth: None,
        }
        .compute_group(&data, &ScaleSet::new());
        assert_eq!(
            out.column_names(),
            vec!["x", "y", "count", "density", "xmin", "xmax"]
        );
        let y: Vec<f64> = out
            .column("y")
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert_eq!(y.iter().sum::<f64>(), 6.0);
    }

    #[test]
    fn groups_share_one_set_of_bin_edges() {
        // Two groups with different ranges: every group's bins must land on
        // the SAME grid (computed from the pooled x range), not each its
        // own — otherwise overlaid/stacked/dodged bars from different
        // groups wouldn't line up.
        let data = grouped(&[0.0, 1.0, 9.0, 10.0], &["a", "a", "b", "b"]);
        let out = StatBin {
            bins: 5,
            binwidth: None,
        }
        .compute_group(&data, &ScaleSet::new());
        let xmins: Vec<f64> = out
            .column("xmin")
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let fills: Vec<String> = out
            .column("fill")
            .unwrap()
            .iter()
            .map(|v| v.to_group_key())
            .collect();
        let a_edges: std::collections::BTreeSet<i64> = xmins
            .iter()
            .zip(&fills)
            .filter(|(_, f)| *f == "a")
            .map(|(x, _)| (x * 1000.0).round() as i64)
            .collect();
        let b_edges: std::collections::BTreeSet<i64> = xmins
            .iter()
            .zip(&fills)
            .filter(|(_, f)| *f == "b")
            .map(|(x, _)| (x * 1000.0).round() as i64)
            .collect();
        assert_eq!(a_edges, b_edges, "both groups share the same bin edges");
        // Every bin appears for every group (b has none in [0,1], a has
        // none in [9,10]) — a sparse group still gets a zero-count row,
        // not a missing one.
        let a_total: f64 = out
            .column("y")
            .unwrap()
            .iter()
            .zip(&fills)
            .filter(|(_, f)| *f == "a")
            .map(|(v, _)| v.as_f64().unwrap())
            .sum();
        assert_eq!(a_total, 2.0);
    }

    #[test]
    fn density_is_normalised_within_its_own_group() {
        // Group "a" has 3 rows, "b" has 1 — each group's own density must
        // integrate to 1 (not the pooled total), ggplot2's stat_bin default.
        let data = grouped(&[1.0, 1.0, 1.0, 1.0], &["a", "a", "a", "b"]);
        let out = StatBin {
            bins: 1,
            binwidth: None,
        }
        .compute_group(&data, &ScaleSet::new());
        let width = out.column("xmax").unwrap()[0].as_f64().unwrap()
            - out.column("xmin").unwrap()[0].as_f64().unwrap();
        let fills: Vec<String> = out
            .column("fill")
            .unwrap()
            .iter()
            .map(|v| v.to_group_key())
            .collect();
        let density: Vec<f64> = out
            .column("density")
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        for (f, d) in fills.iter().zip(&density) {
            // A single bin holds every one of the group's values, so its
            // density * width (its share of that group's own total) is 1 —
            // independent of the *other* group's size (3 vs. 1 rows).
            assert!((d * width - 1.0).abs() < 1e-9, "{f}: {d}");
        }
    }

    #[test]
    fn carried_columns_stay_constant_within_a_group() {
        let mut data = grouped(&[1.0, 2.0, 1.0, 2.0], &["a", "a", "b", "b"]);
        data.add_column(
            "color".into(),
            vec![
                Value::Str("red".into()),
                Value::Str("red".into()),
                Value::Str("blue".into()),
                Value::Str("blue".into()),
            ],
        );
        let out = StatBin {
            bins: 2,
            binwidth: None,
        }
        .compute_group(&data, &ScaleSet::new());
        let fills = out.column("fill").unwrap();
        let colors = out.column("color").unwrap();
        for (f, c) in fills.iter().zip(colors.iter()) {
            let expect = if f.to_group_key() == "a" {
                "red"
            } else {
                "blue"
            };
            assert_eq!(c.to_group_key(), expect);
        }
    }
}
