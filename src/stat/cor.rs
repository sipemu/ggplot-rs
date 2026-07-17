//! `stat_cor` — annotate a scatter plot with a correlation coefficient and
//! p-value, matching R's `ggpubr::stat_cor()`. The statistics come from the
//! `anofox-statistics` crate (Pearson / Spearman `cor.test`); this stat only
//! formats the label and positions it.

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Correlation method for [`StatCor`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CorMethod {
    /// Pearson product-moment correlation.
    #[default]
    Pearson,
    /// Spearman rank correlation.
    Spearman,
}

/// Computes a correlation label (`"R = 0.87, p = 0.0012"`) for each group and
/// emits a single text row per group, consumed by `GeomText`.
pub struct StatCor {
    /// Pearson or Spearman.
    pub method: CorMethod,
    /// Label x position (data coords); defaults to the group's minimum x.
    pub label_x: Option<f64>,
    /// Label y position (data coords); defaults to the group's maximum y.
    pub label_y: Option<f64>,
    /// Significant digits shown for the coefficient.
    pub digits: usize,
}

impl Default for StatCor {
    fn default() -> Self {
        StatCor {
            method: CorMethod::Pearson,
            label_x: None,
            label_y: None,
            digits: 2,
        }
    }
}

impl StatCor {
    /// A correlation stat using `method`.
    pub fn new(method: CorMethod) -> Self {
        StatCor {
            method,
            ..Default::default()
        }
    }
    /// Pin the label to explicit data coordinates.
    pub fn label_pos(mut self, x: f64, y: f64) -> Self {
        self.label_x = Some(x);
        self.label_y = Some(y);
        self
    }
}

impl Stat for StatCor {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (xc, yc) = match (data.column("x"), data.column("y")) {
            (Some(x), Some(y)) => (x, y),
            _ => return DataFrame::new(),
        };
        let pairs: Vec<(f64, f64)> = xc
            .iter()
            .zip(yc.iter())
            .filter_map(|(a, b)| Some((a.as_f64()?, b.as_f64()?)))
            .filter(|(a, b)| a.is_finite() && b.is_finite())
            .collect();
        if pairs.len() < 3 {
            return DataFrame::new();
        }
        let xs: Vec<f64> = pairs.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = pairs.iter().map(|p| p.1).collect();

        let res = match self.method {
            CorMethod::Pearson => anofox_statistics::pearson(&xs, &ys, None),
            CorMethod::Spearman => anofox_statistics::spearman(&xs, &ys, None),
        };
        let res = match res {
            Ok(r) => r,
            Err(_) => return DataFrame::new(),
        };
        let label = format!(
            "R = {:.*}, {}",
            self.digits,
            res.estimate,
            format_p_value(res.p_value)
        );

        let xmin = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let ymax = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let lx = self.label_x.unwrap_or(xmin);
        let ly = self.label_y.unwrap_or(ymax);

        let mut out = DataFrame::new();
        out.add_column("x".into(), vec![Value::Float(lx)]);
        out.add_column("y".into(), vec![Value::Float(ly)]);
        out.add_column("label".into(), vec![Value::Str(label)]);
        for col in &["color", "fill", "group"] {
            if let Some(c) = data.column(col) {
                if let Some(first) = c.first() {
                    out.add_column((*col).to_string(), vec![first.clone()]);
                }
            }
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "cor"
    }
}

/// Format a p-value the way ggpubr does: a `< 2.2e-16` floor, scientific for
/// very small values, fixed-point otherwise. Shared with `stat_compare_means`.
pub(crate) fn format_p_value(p: f64) -> String {
    if p < 2.2e-16 {
        "p < 2.2e-16".to_string()
    } else if p < 1e-4 {
        format!("p = {p:.2e}")
    } else {
        format!("p = {p:.4}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(xy: &[(f64, f64)]) -> DataFrame {
        let mut df = DataFrame::new();
        df.add_column("x".into(), xy.iter().map(|p| Value::Float(p.0)).collect());
        df.add_column("y".into(), xy.iter().map(|p| Value::Float(p.1)).collect());
        df
    }

    #[test]
    fn emits_single_label_row_with_correlation() {
        // Perfectly correlated → R = 1.00.
        let xy: Vec<(f64, f64)> = (0..10).map(|i| (i as f64, 2.0 * i as f64 + 1.0)).collect();
        let out = StatCor::default().compute_group(&frame(&xy), &ScaleSet::new());
        assert_eq!(out.nrows(), 1);
        let label = out.column("label").unwrap()[0].clone();
        let s = match label {
            Value::Str(s) => s,
            _ => panic!("label must be a string"),
        };
        assert!(s.starts_with("R = 1.00"), "got {s}");
    }

    #[test]
    fn too_few_points_returns_empty() {
        let out =
            StatCor::default().compute_group(&frame(&[(0.0, 0.0), (1.0, 1.0)]), &ScaleSet::new());
        assert_eq!(out.nrows(), 0);
    }

    #[test]
    fn spearman_handles_monotone_nonlinear() {
        // Monotone but nonlinear → Spearman R = 1.00, Pearson < 1.
        let xy: Vec<(f64, f64)> = (1..12).map(|i| (i as f64, (i as f64).powi(3))).collect();
        let out = StatCor::new(CorMethod::Spearman).compute_group(&frame(&xy), &ScaleSet::new());
        let s = match out.column("label").unwrap()[0].clone() {
            Value::Str(s) => s,
            _ => unreachable!(),
        };
        assert!(s.starts_with("R = 1.00"), "spearman got {s}");
    }
}
