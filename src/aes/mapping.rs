use crate::aes::{Aes, MappingStage};
use crate::data::DataFrame;

/// Evaluate aesthetic mappings: extract columns from source data
/// and rename them to canonical aesthetic names (x, y, color, etc.).
/// Only resolves BeforeStat mappings.
pub fn resolve_mappings(data: &DataFrame, mapping: &Aes) -> DataFrame {
    let mut result = DataFrame::new();
    let nrows = data.nrows();

    if nrows == 0 {
        return result;
    }

    for m in &mapping.mappings {
        if m.stage != MappingStage::BeforeStat {
            continue;
        }
        let col_name = m.aesthetic.col_name();
        if let Some(values) = data.column(&m.column) {
            result.add_column(col_name.to_string(), values.to_vec());
        }
    }

    // Also carry over any columns that already have canonical names and aren't mapped
    for name in data.column_names() {
        if !result.has_column(name) {
            // Keep original columns available for stats that need them
            if let Some(values) = data.column(name) {
                if result.nrows() == 0 || values.len() == result.nrows() {
                    result.add_column(name.to_string(), values.to_vec());
                }
            }
        }
    }

    result
}

/// Apply after_stat mappings: rename stat-computed columns to canonical aesthetic names.
/// Called after the stat step in the build pipeline.
pub fn apply_after_stat(data: &mut DataFrame, mapping: &Aes) {
    for m in &mapping.mappings {
        if m.stage != MappingStage::AfterStat {
            continue;
        }
        let target = m.aesthetic.col_name();
        let source = &m.column;

        // If the stat produced the source column, rename it to the target aesthetic
        if let Some(values) = data.column(source) {
            let values = values.to_vec();
            // Remove existing target column if any, then add the new mapping
            if data.has_column(target) {
                if let Some(col) = data.column_mut(target) {
                    *col = values;
                }
            } else {
                data.add_column(target.to_string(), values);
            }
        }
    }
}
