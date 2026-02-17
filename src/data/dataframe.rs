use indexmap::IndexMap;

use super::Value;

/// Internal columnar DataFrame for data storage and manipulation.
#[derive(Clone, Debug)]
pub struct DataFrame {
    columns: IndexMap<String, Vec<Value>>,
    nrows: usize,
}

impl DataFrame {
    /// Create an empty DataFrame.
    pub fn new() -> Self {
        DataFrame {
            columns: IndexMap::new(),
            nrows: 0,
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

    /// Add a column. Panics if length doesn't match existing rows (unless empty).
    pub fn add_column(&mut self, name: String, values: Vec<Value>) {
        if self.columns.is_empty() {
            self.nrows = values.len();
        } else {
            assert_eq!(
                values.len(),
                self.nrows,
                "Column '{}' has {} values but DataFrame has {} rows",
                name,
                values.len(),
                self.nrows
            );
        }
        self.columns.insert(name, values);
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
        let mut group_map: IndexMap<Vec<String>, Vec<usize>> = IndexMap::new();

        for i in 0..self.nrows {
            let key: Vec<String> = keys
                .iter()
                .map(|k| {
                    self.columns
                        .get(*k)
                        .map(|col| col[i].to_group_key())
                        .unwrap_or_else(|| "NA".to_string())
                })
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
        assert!(idx < self.nrows, "Row index {idx} out of bounds ({} rows)", self.nrows);
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
            va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
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
        let mut seen: Vec<String> = Vec::new();
        let mut result = Vec::new();
        for v in col {
            let key = v.to_group_key();
            if !seen.contains(&key) {
                seen.push(key);
                result.push(v.clone());
            }
        }
        result
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
            vec![Value::Str("a".into()), Value::Str("b".into()), Value::Str("a".into())],
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
