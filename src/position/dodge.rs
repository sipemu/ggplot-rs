use crate::data::{DataFrame, Value};

use super::{Position, PositionParams, DODGE_OFFSET_COL};

/// Place groups side-by-side (ggplot2's `position_dodge(width)`).
///
/// Within each x position the groups present there (group / fill / colour
/// level, in first-appearance order — the legend order — or reversed with
/// [`with_reverse`](Self::with_reverse)) share `width` (in x units; one
/// category on a discrete axis) and are centred on x:
/// `offset = (k − (n − 1)/2) · width / n` for the k-th of n groups. On a
/// continuous axis x itself moves; on a discrete axis the offset is stored in
/// the `.x_dodge_offset` column and applied by the geoms that support it
/// (point, pointrange, errorbar, linerange, ribbon, censor marks), so several
/// models' intervals per term sit side by side — also under `coord_flip`.
///
/// `PositionDodge` is also usable as a value (the default, `width` from the
/// layer, 0.9), as before 0.17.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PositionDodge {
    /// Total dodge width; `None` = the layer default (0.9).
    pub width: Option<f64>,
    /// Reverse the group order (ggplot2 ≥ 3.5 `reverse = TRUE`).
    pub reverse: bool,
}

/// The default dodge (`width` 0.9, groups in legend order): lets
/// `.position(PositionDodge)` keep compiling now that the type has fields.
#[allow(non_upper_case_globals)]
pub const PositionDodge: PositionDodge = PositionDodge {
    width: None,
    reverse: false,
};

impl PositionDodge {
    /// Dodge with an explicit total `width` (ggplot2's `position_dodge(width)`).
    pub fn new(width: f64) -> Self {
        PositionDodge {
            width: Some(width),
            reverse: false,
        }
    }

    /// Reverse the order of the dodged groups.
    pub fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
}

/// `position_dodge(width)` — see [`PositionDodge`](struct@PositionDodge).
pub fn position_dodge(width: f64) -> PositionDodge {
    PositionDodge::new(width)
}

impl Position for PositionDodge {
    fn compute(&self, data: &mut DataFrame, params: &PositionParams) {
        dodge_rows(data, self.width.unwrap_or(params.width), None, self.reverse);
    }

    fn name(&self) -> &str {
        "dodge"
    }
}

/// The dodge key of each row: its group / fill / colour levels combined
/// (`None` when the frame has none of these columns).
fn group_keys(data: &DataFrame) -> Option<Vec<String>> {
    let cols: Vec<&[Value]> = ["group", "fill", "color"]
        .iter()
        .filter_map(|c| data.column(c))
        .collect();
    if cols.is_empty() {
        return None;
    }
    Some(
        (0..data.nrows())
            .map(|i| {
                cols.iter()
                    .map(|c| c[i].to_group_key())
                    .collect::<Vec<_>>()
                    .join("\u{1f}")
            })
            .collect(),
    )
}

/// Shared dodge / dodge2 implementation. `padding` (dodge2) also rewrites
/// `xmin`/`xmax` on a continuous axis to the shrunken element extent.
pub(crate) fn dodge_rows(data: &mut DataFrame, width: f64, padding: Option<f64>, reverse: bool) {
    let Some(x_col) = data.column("x").map(|c| c.to_vec()) else {
        return;
    };
    let Some(keys) = group_keys(data) else {
        return;
    };
    let width = if width.is_finite() && width > 0.0 {
        width
    } else {
        0.9
    };
    // Global group order (legend order), optionally reversed.
    let mut order: indexmap::IndexSet<&str> = keys.iter().map(|k| k.as_str()).collect();
    if order.len() <= 1 {
        return;
    }
    if reverse {
        order.reverse();
    }
    let rank = |k: &str| order.get_index_of(k).unwrap_or(0);

    // Groups present at each x position, sorted by the global order.
    let mut slots: indexmap::IndexMap<String, Vec<usize>> = indexmap::IndexMap::new();
    for (i, x) in x_col.iter().enumerate() {
        if x.is_na() {
            continue;
        }
        let present = slots.entry(x.to_group_key()).or_default();
        let r = rank(&keys[i]);
        if !present.contains(&r) {
            present.push(r);
        }
    }
    for present in slots.values_mut() {
        present.sort_unstable();
    }

    let discrete = x_col
        .iter()
        .any(|v| matches!(v, Value::Str(_) | Value::Bool(_)));
    let mut offsets = vec![Value::Float(0.0); x_col.len()];
    let mut new_x = x_col.clone();
    let mut new_xmin = data.column("xmin").map(|c| c.to_vec());
    let mut new_xmax = data.column("xmax").map(|c| c.to_vec());
    for (i, x) in x_col.iter().enumerate() {
        let Some(present) = slots.get(&x.to_group_key()).filter(|_| !x.is_na()) else {
            continue;
        };
        let n = present.len() as f64;
        let k = present
            .iter()
            .position(|&r| r == rank(&keys[i]))
            .unwrap_or(0) as f64;
        let group_width = width / n;
        let offset = (k - (n - 1.0) / 2.0) * group_width;
        if discrete {
            offsets[i] = Value::Float(offset);
            continue;
        }
        let Some(xv) = x.as_f64() else {
            continue;
        };
        let center = xv + offset;
        new_x[i] = Value::Float(center);
        if let Some(pad) = padding {
            let half = group_width * (1.0 - pad) / 2.0;
            if let Some(c) = new_xmin.as_mut() {
                c[i] = Value::Float(center - half);
            }
            if let Some(c) = new_xmax.as_mut() {
                c[i] = Value::Float(center + half);
            }
        }
    }

    if discrete {
        if let Some(col) = data.column_mut(DODGE_OFFSET_COL) {
            *col = offsets;
        } else {
            data.add_column(DODGE_OFFSET_COL.to_string(), offsets);
        }
        return;
    }
    if let Some(col) = data.column_mut("x") {
        *col = new_x;
    }
    if padding.is_some() {
        for (name, vals) in [("xmin", new_xmin), ("xmax", new_xmax)] {
            if let (Some(vals), Some(col)) = (vals, data.column_mut(name)) {
                *col = vals;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(x: Vec<Value>, g: &[&str]) -> DataFrame {
        let mut df = DataFrame::new();
        df.add_column("x".into(), x);
        df.add_column(
            "color".into(),
            g.iter().map(|s| Value::Str(s.to_string())).collect(),
        );
        df
    }

    fn col(df: &DataFrame, c: &str) -> Vec<f64> {
        df.column(c)
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    }

    #[test]
    fn continuous_dodge_matches_ggplot2() {
        // ggplot2: position_dodge(width = 0.6) of 3 groups at x = 1 →
        // 0.8, 1.0, 1.2.
        let mut df = frame(vec![Value::Float(1.0); 3], &["a", "b", "c"]);
        PositionDodge::new(0.6).compute(&mut df, &PositionParams::default());
        let x = col(&df, "x");
        for (got, want) in x.iter().zip([0.8, 1.0, 1.2]) {
            assert!((got - want).abs() < 1e-12);
        }
        let mut df = frame(vec![Value::Float(1.0); 3], &["a", "b", "c"]);
        PositionDodge::new(0.6)
            .with_reverse(true)
            .compute(&mut df, &PositionParams::default());
        let x = col(&df, "x");
        assert!((x[0] - 1.2).abs() < 1e-12 && (x[2] - 0.8).abs() < 1e-12);
    }

    #[test]
    fn discrete_dodge_stores_offsets_and_centres_partial_slots() {
        let x: Vec<Value> = ["t1", "t1", "t2"]
            .iter()
            .map(|s| Value::Str(s.to_string()))
            .collect();
        let mut df = frame(x, &["m1", "m2", "m2"]);
        PositionDodge::new(0.5).compute(&mut df, &PositionParams::default());
        // x itself untouched (a category), offsets in category units.
        assert_eq!(df.column("x").unwrap()[0].as_str(), Some("t1"));
        let off = col(&df, DODGE_OFFSET_COL);
        assert!((off[0] + 0.125).abs() < 1e-12);
        assert!((off[1] - 0.125).abs() < 1e-12);
        // t2 has only m2 → centred (ggplot2's preserve = "total").
        assert_eq!(off[2], 0.0);
    }

    #[test]
    fn unit_value_still_works() {
        let mut df = frame(vec![Value::Float(1.0); 2], &["a", "b"]);
        PositionDodge.compute(&mut df, &PositionParams::default());
        let x = col(&df, "x");
        assert!((x[0] - 0.775).abs() < 1e-12 && (x[1] - 1.225).abs() < 1e-12);
    }
}
