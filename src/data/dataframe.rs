use indexmap::IndexMap;

use super::{GroupKey, Value};

/// Internal columnar DataFrame for data storage and manipulation.
#[derive(Clone, Debug)]
pub struct DataFrame {
    columns: IndexMap<String, Vec<Value>>,
    nrows: usize,
    /// Problems found while assembling the frame (e.g. mismatched column
    /// lengths). Reported by [`validate`](Self::validate), and turned into a
    /// `GGError::ValidationError` when a plot using this frame is built.
    issues: Vec<String>,
}

impl DataFrame {
    /// Create an empty DataFrame.
    pub fn new() -> Self {
        DataFrame {
            columns: IndexMap::new(),
            nrows: 0,
            issues: Vec::new(),
        }
    }

    /// Get a column by name.
    pub fn column(&self, name: &str) -> Option<&[Value]> {
        self.columns.get(name).map(|v| v.as_slice())
    }

    /// Get number of rows.
    pub fn nrows(&self) -> usize {
        self.nrows
    }

    /// Get number of columns.
    pub fn ncols(&self) -> usize {
        self.columns.len()
    }

    /// Get column names.
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a column exists.
    pub fn has_column(&self, name: &str) -> bool {
        self.columns.contains_key(name)
    }

    /// Add (or replace) a column.
    ///
    /// Never panics. If the length doesn't match the existing rows, the shorter
    /// side is padded with [`Value::Na`] and the mismatch is recorded as an
    /// [issue](Self::issues): building or rendering a plot from this frame then
    /// fails with `GGError::ValidationError`. Use
    /// [`try_add_column`](Self::try_add_column) to reject the column instead.
    pub fn add_column(&mut self, name: String, mut values: Vec<Value>) {
        if self.columns.is_empty() {
            self.nrows = values.len();
        } else if values.len() != self.nrows {
            self.issues
                .push(Self::mismatch_message(&name, values.len(), self.nrows));
            if values.len() < self.nrows {
                values.resize(self.nrows, Value::Na);
            } else {
                let n = values.len();
                for col in self.columns.values_mut() {
                    col.resize(n, Value::Na);
                }
                self.nrows = n;
            }
        }
        self.columns.insert(name, values);
    }

    /// Add (or replace) a column, rejecting a length mismatch with a
    /// `GGError::ValidationError` (the frame is left unchanged).
    pub fn try_add_column(
        &mut self,
        name: String,
        values: Vec<Value>,
    ) -> Result<(), crate::plot::GGError> {
        if !self.columns.is_empty() && values.len() != self.nrows {
            return Err(crate::plot::GGError::ValidationError(
                Self::mismatch_message(&name, values.len(), self.nrows),
            ));
        }
        self.add_column(name, values);
        Ok(())
    }

    fn mismatch_message(name: &str, len: usize, nrows: usize) -> String {
        format!("column '{name}' has {len} values but the data has {nrows} rows")
    }

    /// Problems recorded while assembling this frame (empty when well-formed).
    pub fn issues(&self) -> &[String] {
        &self.issues
    }

    /// `Ok` when the frame is well-formed, else a `GGError::ValidationError`
    /// describing every recorded [issue](Self::issues).
    pub fn validate(&self) -> Result<(), crate::plot::GGError> {
        if self.issues.is_empty() {
            Ok(())
        } else {
            Err(crate::plot::GGError::ValidationError(
                self.issues.join("; "),
            ))
        }
    }

    /// Get a mutable reference to a column.
    pub fn column_mut(&mut self, name: &str) -> Option<&mut Vec<Value>> {
        self.columns.get_mut(name)
    }

    /// Group by one or more key columns. Returns a Vec of DataFrames, one per group.
    pub fn group_by(&self, keys: &[&str]) -> Vec<DataFrame> {
        if self.nrows == 0 {
            return vec![];
        }

        // Build group keys for each row
        // Borrowing keys (no per-row String clones for text columns); a missing
        // value is its own group, distinct from the literal string "NA".
        let key_cols: Vec<Option<&Vec<Value>>> =
            keys.iter().map(|k| self.columns.get(*k)).collect();
        let mut group_map: IndexMap<Vec<GroupKey<'_>>, Vec<usize>> = IndexMap::new();

        for i in 0..self.nrows {
            let key: Vec<GroupKey<'_>> = key_cols
                .iter()
                .map(|col| col.map_or(GroupKey::Na, |c| c[i].group_key()))
                .collect();
            group_map.entry(key).or_default().push(i);
        }

        group_map
            .into_values()
            .map(|indices| {
                let mut df = DataFrame::new();
                for (name, col) in &self.columns {
                    let values: Vec<Value> = indices.iter().map(|&i| col[i].clone()).collect();
                    df.add_column(name.clone(), values);
                }
                df
            })
            .collect()
    }

    /// Vertically stack another DataFrame onto this one.
    pub fn vstack(&mut self, other: &DataFrame) {
        if other.nrows == 0 {
            return;
        }
        if self.columns.is_empty() {
            *self = other.clone();
            return;
        }

        // Add columns from other that we have
        for (name, col) in &self.columns {
            if let Some(other_col) = other.columns.get(name) {
                // Will extend below
                let _ = (col, other_col);
            }
        }

        // Also add columns from other that we don't have (fill with NA)
        for name in other.columns.keys() {
            if !self.columns.contains_key(name) {
                self.columns
                    .insert(name.clone(), vec![Value::Na; self.nrows]);
            }
        }

        let old_nrows = self.nrows;
        self.nrows += other.nrows;

        for (name, col) in &mut self.columns {
            if let Some(other_col) = other.columns.get(name) {
                col.extend(other_col.iter().cloned());
            } else {
                col.extend(std::iter::repeat_with(|| Value::Na).take(other.nrows));
            }
            debug_assert_eq!(col.len(), old_nrows + other.nrows);
        }
    }

    /// Select a subset of columns.
    pub fn select(&self, columns: &[&str]) -> DataFrame {
        let mut df = DataFrame::new();
        for &col_name in columns {
            if let Some(col) = self.columns.get(col_name) {
                df.add_column(col_name.to_string(), col.clone());
            }
        }
        df
    }

    /// Get a single row as a map.
    pub fn row(&self, idx: usize) -> IndexMap<String, Value> {
        assert!(
            idx < self.nrows,
            "Row index {idx} out of bounds ({} rows)",
            self.nrows
        );
        let mut map = IndexMap::new();
        for (name, col) in &self.columns {
            map.insert(name.clone(), col[idx].clone());
        }
        map
    }

    /// Sort by a column (ascending). Returns a new DataFrame.
    pub fn sort_by(&self, column: &str) -> DataFrame {
        let col = match self.columns.get(column) {
            Some(c) => c,
            None => return self.clone(),
        };

        let mut indices: Vec<usize> = (0..self.nrows).collect();
        indices.sort_by(|&a, &b| {
            let va = col[a].as_f64().unwrap_or(f64::NAN);
            let vb = col[b].as_f64().unwrap_or(f64::NAN);
            va.total_cmp(&vb)
        });

        let mut df = DataFrame::new();
        for (name, c) in &self.columns {
            let values: Vec<Value> = indices.iter().map(|&i| c[i].clone()).collect();
            df.add_column(name.clone(), values);
        }
        df
    }

    /// Create from rows (list of maps).
    pub fn from_rows(rows: Vec<IndexMap<String, Value>>) -> Self {
        if rows.is_empty() {
            return DataFrame::new();
        }

        // Collect all column names from all rows
        let mut col_names: IndexMap<String, ()> = IndexMap::new();
        for row in &rows {
            for key in row.keys() {
                col_names.entry(key.clone()).or_default();
            }
        }

        let mut df = DataFrame::new();
        for name in col_names.keys() {
            let values: Vec<Value> = rows
                .iter()
                .map(|row| row.get(name).cloned().unwrap_or(Value::Na))
                .collect();
            df.add_column(name.clone(), values);
        }
        df
    }

    /// Get all unique values in a column.
    pub fn unique_values(&self, column: &str) -> Vec<Value> {
        let col = match self.columns.get(column) {
            Some(c) => c,
            None => return vec![],
        };
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for v in col {
            if seen.insert(v.group_key()) {
                result.push(v.clone());
            }
        }
        result
    }
}

impl DataFrame {
    /// Load a DataFrame from a CSV file.
    /// First row is treated as column headers.
    /// Values are parsed as Float if possible, otherwise kept as strings.
    /// The literal string "NA" is parsed as Value::Na.
    pub fn from_csv(path: &str) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let mut lines = content.lines();

        let header = match lines.next() {
            Some(h) => h,
            None => return Ok(DataFrame::new()),
        };

        let col_names: Vec<&str> = header.split(',').map(|s| s.trim()).collect();
        let mut columns: Vec<Vec<Value>> = vec![Vec::new(); col_names.len()];

        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split(',').collect();
            for (i, field) in fields.iter().enumerate() {
                if i >= col_names.len() {
                    continue;
                }
                let field = field.trim();
                let val = if field == "NA" || field == "na" {
                    Value::Na
                } else if let Ok(f) = field.parse::<f64>() {
                    Value::Float(f)
                } else {
                    Value::Str(field.to_string())
                };
                columns[i].push(val);
            }
            // Pad missing columns with NA
            for col in columns.iter_mut().skip(fields.len()) {
                col.push(Value::Na);
            }
        }

        let mut df = DataFrame::new();
        for (i, name) in col_names.iter().enumerate() {
            if !columns[i].is_empty() {
                df.add_column(name.to_string(), std::mem::take(&mut columns[i]));
            }
        }

        Ok(df)
    }
}

impl Default for DataFrame {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatched_column_lengths_pad_and_record_an_issue() {
        let mut df = DataFrame::new();
        df.add_column("x".into(), vec![Value::Float(1.0), Value::Float(2.0)]);
        df.add_column("y".into(), vec![Value::Float(3.0)]);
        assert_eq!(df.nrows(), 2);
        assert_eq!(df.column("y").unwrap()[1], Value::Na);
        df.add_column("z".into(), vec![Value::Float(0.0); 4]);
        assert_eq!(df.nrows(), 4);
        assert!(df
            .column_names()
            .iter()
            .all(|c| df.column(c).unwrap().len() == 4));
        assert_eq!(df.issues().len(), 2);
        let err = df.validate().unwrap_err().to_string();
        assert!(err.contains("'y'") && err.contains("'z'"), "{err}");

        let mut ok = DataFrame::new();
        ok.add_column("x".into(), vec![Value::Float(1.0)]);
        assert!(ok.validate().is_ok());
        assert!(ok
            .try_add_column("y".into(), vec![Value::Na, Value::Na])
            .is_err());
        assert!(!ok.has_column("y"), "rejected column is not added");
        assert!(ok.try_add_column("y".into(), vec![Value::Na]).is_ok());
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn group_by_keeps_na_apart_from_literal_na() {
        let mut df = DataFrame::new();
        df.add_column(
            "g".into(),
            vec![
                Value::Na,
                Value::Str("NA".into()),
                Value::Na,
                Value::Str("NA".into()),
            ],
        );
        df.add_column("v".into(), (0..4).map(Value::Integer).collect());
        let groups = df.group_by(&["g"]);
        assert_eq!(groups.len(), 2);
        assert!(groups[0].column("g").unwrap().iter().all(Value::is_na));
        assert!(groups[1]
            .column("g")
            .unwrap()
            .iter()
            .all(|v| v.as_str() == Some("NA")));
        assert_eq!(df.unique_values("g").len(), 2);
    }

    #[test]
    fn test_add_column_and_access() {
        let mut df = DataFrame::new();
        df.add_column("x".into(), vec![Value::Float(1.0), Value::Float(2.0)]);
        df.add_column("y".into(), vec![Value::Float(3.0), Value::Float(4.0)]);

        assert_eq!(df.nrows(), 2);
        assert_eq!(df.ncols(), 2);
        assert!(df.has_column("x"));
        assert!(!df.has_column("z"));
    }

    #[test]
    fn test_group_by() {
        let mut df = DataFrame::new();
        df.add_column(
            "cat".into(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("a".into()),
            ],
        );
        df.add_column(
            "val".into(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        );

        let groups = df.group_by(&["cat"]);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].nrows(), 2); // "a" group
        assert_eq!(groups[1].nrows(), 1); // "b" group
    }

    #[test]
    fn test_vstack() {
        let mut df1 = DataFrame::new();
        df1.add_column("x".into(), vec![Value::Float(1.0)]);

        let mut df2 = DataFrame::new();
        df2.add_column("x".into(), vec![Value::Float(2.0)]);

        df1.vstack(&df2);
        assert_eq!(df1.nrows(), 2);
    }

    #[test]
    fn test_sort_by() {
        let mut df = DataFrame::new();
        df.add_column(
            "x".into(),
            vec![Value::Float(3.0), Value::Float(1.0), Value::Float(2.0)],
        );
        let sorted = df.sort_by("x");
        let col = sorted.column("x").unwrap();
        assert_eq!(col[0].as_f64(), Some(1.0));
        assert_eq!(col[1].as_f64(), Some(2.0));
        assert_eq!(col[2].as_f64(), Some(3.0));
    }
}
