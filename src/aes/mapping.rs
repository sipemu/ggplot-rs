use crate::aes::Aes;
use crate::data::DataFrame;

/// Evaluate aesthetic mappings: extract columns from source data
/// and rename them to canonical aesthetic names (x, y, color, etc.).
pub fn resolve_mappings(data: &DataFrame, mapping: &Aes) -> DataFrame {
    let mut result = DataFrame::new();
    let nrows = data.nrows();

    if nrows == 0 {
        return result;
    }

    for m in &mapping.mappings {
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
