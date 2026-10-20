use crate::aes::{validated_data_attr_internal_name, Aes, Aesthetic, MappingStage};
use crate::data::DataFrame;

/// Evaluate aesthetic mappings: extract columns from source data
/// and rename them to canonical aesthetic names (x, y, color, etc.).
/// Only resolves BeforeStat mappings.
///
/// A `DataAttr` mapping's target is re-validated here (see
/// `validated_data_attr_internal_name`) regardless of how it was built,
/// and stored under an internal, non-colliding column name — not the plain
/// `"data-<name>"` a caller's own unrelated raw data column could also be
/// named. `geom::set_row_extras` only reads that internal prefix, so it
/// never mistakes an incidental raw `data-*` column for a validated one.
/// An invalid or reserved target is dropped with a warning.
pub fn resolve_mappings(data: &DataFrame, mapping: &Aes, warnings: &mut Vec<String>) -> DataFrame {
    let mut result = DataFrame::new();
    let nrows = data.nrows();

    if nrows == 0 {
        return result;
    }

    for m in &mapping.mappings {
        if m.stage != MappingStage::BeforeStat {
            continue;
        }
        let col_name = match &m.aesthetic {
            Aesthetic::DataAttr(target) => match validated_data_attr_internal_name(target) {
                Some(internal) => internal,
                None => {
                    warnings.push(format!(
                        "aes: data_attr target {target:?} is invalid or reserved; skipped"
                    ));
                    continue;
                }
            },
            other => other.col_name().to_string(),
        };
        if let Some(values) = data.column(&m.column) {
            result.add_column(col_name, values.to_vec());
        } else if let Some(values) = super::expr::eval_expression(&m.column, data) {
            // Computed aesthetic — `m.column` is an expression like "a / b" or
            // "log(gdp)" rather than a bare column name.
            result.add_column(col_name, values);
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
/// Called after the stat step in the build pipeline. `DataAttr` targets are
/// validated the same way as in [`resolve_mappings`] (a `DataAttr` mapping
/// built directly and pushed with `MappingStage::AfterStat` is obscure but
/// not impossible, so it gets the same gate).
pub fn apply_after_stat(data: &mut DataFrame, mapping: &Aes, warnings: &mut Vec<String>) {
    for m in &mapping.mappings {
        if m.stage != MappingStage::AfterStat {
            continue;
        }
        let target = match &m.aesthetic {
            Aesthetic::DataAttr(target) => match validated_data_attr_internal_name(target) {
                Some(internal) => internal,
                None => {
                    warnings.push(format!(
                        "aes: data_attr target {target:?} is invalid or reserved; skipped"
                    ));
                    continue;
                }
            },
            other => other.col_name().to_string(),
        };
        let source = &m.column;

        // If the stat produced the source column, rename it to the target
        // aesthetic; otherwise evaluate `source` as an expression over the
        // stat output (e.g. after_stat_y("count / sum(count)") for proportions).
        let values = if let Some(values) = data.column(source) {
            Some(values.to_vec())
        } else {
            super::expr::eval_expression(source, data)
        };
        if let Some(values) = values {
            // Remove existing target column if any, then add the new mapping
            if data.has_column(&target) {
                if let Some(col) = data.column_mut(&target) {
                    *col = values;
                }
            } else {
                data.add_column(target, values);
            }
        }
    }
}
