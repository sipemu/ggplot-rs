use crate::aes::{
    validated_data_attr_internal_name, Aes, Aesthetic, MappingStage, INTERNAL_PREFIX,
    TOOLTIP_INTERNAL_NAME,
};
use crate::data::DataFrame;

/// The internal column name a mapping should be stored under, or `None` if
/// it's invalid/reserved (in which case a warning was already pushed).
/// Shared by [`resolve_mappings`] and [`apply_after_stat`] so `DataAttr` and
/// `Tooltip` get exactly the same validation and routing regardless of
/// which build stage (before/after stat) created the mapping.
fn target_column(aesthetic: &Aesthetic, warnings: &mut Vec<String>) -> Option<String> {
    match aesthetic {
        Aesthetic::DataAttr(target) => match validated_data_attr_internal_name(target) {
            Some(internal) => Some(internal),
            None => {
                warnings.push(format!(
                    "aes: data_attr target {target:?} is invalid or reserved; skipped"
                ));
                None
            }
        },
        // A `Tooltip` mapping is stored under its own internal name, not the
        // plain `"tooltip"` an unrelated raw source column could also be
        // named — the same class of bug `DataAttr`'s internal prefix fixes.
        Aesthetic::Tooltip => Some(TOOLTIP_INTERNAL_NAME.to_string()),
        Aesthetic::Class => Some(crate::aes::CLASS_INTERNAL_NAME.to_string()),
        Aesthetic::GroupWrap(target) => {
            match crate::aes::validated_group_wrap_internal_name(target) {
                Some(internal) => Some(internal),
                None => {
                    warnings.push(format!(
                        "aes: group_wrap target {target:?} is invalid or reserved; skipped"
                    ));
                    None
                }
            }
        }
        other => Some(other.col_name().to_string()),
    }
}

/// Evaluate aesthetic mappings: extract columns from source data
/// and rename them to canonical aesthetic names (x, y, color, etc.).
/// Only resolves BeforeStat mappings.
///
/// A `DataAttr`/`Tooltip` mapping's target is re-validated and routed to an
/// internal, non-colliding column name here (see [`target_column`]),
/// regardless of how the mapping was built. `geom::set_row_extras` only
/// reads those internal names, so it never mistakes an incidental raw
/// column of the same plain name (`"tooltip"`, or `"data-<name>"`) for a
/// validated one — and a raw source column is never carried through under
/// an internal name either: any column whose name contains a NUL byte (or,
/// redundantly, the internal prefix itself — the NUL check already covers
/// it, since the prefix starts with one, but a future internal scheme might
/// not) is dropped outright by the carry-over loop below, rather than
/// copied through unchanged. Without that, a caller could still *forge* an
/// internal name directly (`GGPlot::new` accepts any `String` column name,
/// NUL byte included) and have it misread as a validated attribute.
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
        let Some(col_name) = target_column(&m.aesthetic, warnings) else {
            continue;
        };
        if let Some(values) = data.column(&m.column) {
            result.add_column(col_name, values.to_vec());
        } else if let Some(values) = super::expr::eval_expression(&m.column, data) {
            // Computed aesthetic — `m.column` is an expression like "a / b" or
            // "log(gdp)" rather than a bare column name.
            result.add_column(col_name, values);
        }
    }

    // Also carry over any columns that already have canonical names and
    // aren't mapped — except one that could forge an internal name (see
    // this function's doc).
    for name in data.column_names() {
        if is_forged_internal_name(name) {
            continue;
        }
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

/// A raw column name that could be mistaken for — or collide with — an
/// internal column written by `target_column` (NUL byte, or the internal
/// prefix text itself, which always contains a NUL but is checked
/// explicitly too in case that ever changes).
fn is_forged_internal_name(name: &str) -> bool {
    name.contains('\0') || name.contains(INTERNAL_PREFIX)
}

/// Apply after_stat mappings: rename stat-computed columns to canonical
/// aesthetic names. Called after the stat step in the build pipeline.
/// `DataAttr`/`Tooltip` targets are validated and routed the same way as in
/// [`resolve_mappings`] (built directly and pushed with
/// `MappingStage::AfterStat` is obscure but not impossible, so it gets the
/// same gate).
pub fn apply_after_stat(data: &mut DataFrame, mapping: &Aes, warnings: &mut Vec<String>) {
    for m in &mapping.mappings {
        if m.stage != MappingStage::AfterStat {
            continue;
        }
        let Some(target) = target_column(&m.aesthetic, warnings) else {
            continue;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aes::AesMapping;
    use crate::data::Value;

    /// An invalid/reserved `DataAttr` target — bypassing `Aes::data_attr`'s
    /// own validation by pushing the mapping directly — is dropped with a
    /// warning in `resolve_mappings` (the single authoritative gate),
    /// rather than silently resolved.
    #[test]
    fn resolve_mappings_drops_an_invalid_data_attr_target_with_a_warning() {
        let mut data = DataFrame::new();
        data.add_column("x".to_string(), vec![Value::Float(1.0)]);
        data.add_column("v".to_string(), vec![Value::Str("payload".into())]);

        let mut aes = Aes::new();
        aes.mappings.push(AesMapping {
            column: "v".to_string(),
            aesthetic: Aesthetic::DataAttr("data-part".to_string()), // reserved
            stage: MappingStage::BeforeStat,
        });

        let mut warnings = Vec::new();
        let result = resolve_mappings(&data, &aes, &mut warnings);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("reserved"));
        // No internal data-attr column was created for it.
        assert!(!result
            .column_names()
            .iter()
            .any(|n| n.starts_with(crate::aes::DATA_ATTR_INTERNAL_PREFIX)));
    }

    /// Same gate, reached via `apply_after_stat` (a `DataAttr` mapping
    /// pushed with `MappingStage::AfterStat` directly).
    #[test]
    fn apply_after_stat_drops_an_invalid_data_attr_target_with_a_warning() {
        let mut data = DataFrame::new();
        data.add_column("count".to_string(), vec![Value::Float(5.0)]);

        let mut aes = Aes::new();
        aes.mappings.push(AesMapping {
            column: "count".to_string(),
            aesthetic: Aesthetic::DataAttr("data-Bad Name".to_string()),
            stage: MappingStage::AfterStat,
        });

        let mut warnings = Vec::new();
        apply_after_stat(&mut data, &aes, &mut warnings);

        assert_eq!(warnings.len(), 1);
        assert!(!data
            .column_names()
            .iter()
            .any(|n| n.starts_with(crate::aes::DATA_ATTR_INTERNAL_PREFIX)));
    }
}
