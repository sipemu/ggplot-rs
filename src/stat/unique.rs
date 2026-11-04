use crate::aes::Aesthetic;
use crate::data::DataFrame;
use crate::scale::ScaleSet;

use super::Stat;

/// Drops duplicate rows (identical in every column) before drawing — ggplot2's
/// `stat_unique()`, e.g. to avoid overplotting repeated observations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatUnique;

impl Stat for StatUnique {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let names = data.column_names();
        let cols: Vec<_> = names.iter().map(|n| data.column(n).unwrap()).collect();
        let mut seen = std::collections::HashSet::new();
        let keep: Vec<usize> = (0..data.nrows())
            .filter(|&i| {
                let key: Vec<String> = cols.iter().map(|c| c[i].to_group_key()).collect();
                seen.insert(key)
            })
            .collect();
        let mut out = DataFrame::new();
        for (name, col) in names.into_iter().zip(cols) {
            out.add_column(
                name.to_string(),
                keep.iter().map(|&i| col[i].clone()).collect(),
            );
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        Vec::new()
    }

    fn name(&self) -> &str {
        "unique"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Value;

    #[test]
    fn drops_exact_duplicates_only() {
        let mut df = DataFrame::new();
        df.add_column("x".into(), [1.0, 1.0, 1.0, 2.0].map(Value::Float).to_vec());
        df.add_column("y".into(), [5.0, 5.0, 6.0, 5.0].map(Value::Float).to_vec());
        let out = StatUnique.compute_group(&df, &ScaleSet::new());
        assert_eq!(out.nrows(), 3);
    }
}
