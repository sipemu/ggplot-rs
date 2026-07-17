//! `stat_compare_means` — annotate grouped data with a group-comparison
//! p-value, matching R's `ggpubr::stat_compare_means()`. The test statistics
//! come from the `anofox-statistics` crate; this stat picks the test, formats
//! the label, and positions it. It is [panelwise](Stat::panelwise): it sees
//! every group in a panel at once so it can compare across them.

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::cor::format_p_value;
use super::Stat;

/// Group-comparison test for [`StatCompareMeans`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompareMethod {
    /// ggpubr's default: Wilcoxon rank-sum for two groups, Kruskal-Wallis for
    /// more than two.
    #[default]
    Auto,
    /// Wilcoxon rank-sum (Mann-Whitney U) — two groups.
    Wilcoxon,
    /// Two-sample Welch t-test — two groups.
    TTest,
    /// Kruskal-Wallis rank test — two or more groups.
    Kruskal,
    /// One-way (Fisher) ANOVA — two or more groups.
    Anova,
}

/// Compares y across the discrete x groups and emits one text row with the
/// test name and p-value (`"Kruskal-Wallis, p = 0.001"`), consumed by
/// `GeomText`.
pub struct StatCompareMeans {
    /// Which test to run (defaults to [`CompareMethod::Auto`]).
    pub method: CompareMethod,
    /// Label y position (data coords); defaults to the overall maximum y.
    pub label_y: Option<f64>,
}

impl Default for StatCompareMeans {
    fn default() -> Self {
        StatCompareMeans {
            method: CompareMethod::Auto,
            label_y: None,
        }
    }
}

impl StatCompareMeans {
    /// A comparison stat using `method`.
    pub fn new(method: CompareMethod) -> Self {
        StatCompareMeans {
            method,
            label_y: None,
        }
    }
    /// Pin the label to an explicit y (data coords).
    pub fn label_y(mut self, y: f64) -> Self {
        self.label_y = Some(y);
        self
    }
}

impl Stat for StatCompareMeans {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (xc, yc) = match (data.column("x"), data.column("y")) {
            (Some(x), Some(y)) => (x, y),
            _ => return DataFrame::new(),
        };

        // Bucket y by x category, preserving first-seen order.
        let mut groups: Vec<(String, Vec<f64>)> = Vec::new();
        for (xv, yv) in xc.iter().zip(yc.iter()) {
            let y = match yv.as_f64() {
                Some(y) if y.is_finite() => y,
                _ => continue,
            };
            let key = xv.to_group_key();
            if let Some(g) = groups.iter_mut().find(|(k, _)| *k == key) {
                g.1.push(y);
            } else {
                groups.push((key, vec![y]));
            }
        }
        if groups.len() < 2 || groups.iter().any(|(_, v)| v.len() < 2) {
            return DataFrame::new();
        }

        let slices: Vec<&[f64]> = groups.iter().map(|(_, v)| v.as_slice()).collect();
        let (name, p) = run_test(self.method, &slices);
        let p = match p {
            Some(p) => p,
            None => return DataFrame::new(),
        };
        let label = format!("{name}, {}", format_p_value(p));

        let lx = xc.first().cloned().unwrap_or(Value::Float(0.0));
        let ymax = yc
            .iter()
            .filter_map(|v| v.as_f64())
            .filter(|v| v.is_finite())
            .fold(f64::NEG_INFINITY, f64::max);
        let ly = self.label_y.unwrap_or(ymax);

        let mut out = DataFrame::new();
        out.add_column("x".into(), vec![lx]);
        out.add_column("y".into(), vec![Value::Float(ly)]);
        out.add_column("label".into(), vec![Value::Str(label)]);
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "compare_means"
    }
}

/// Run the chosen comparison, resolving `Auto` and falling back from two-group
/// tests to their k-group analogue when more than two groups are present.
fn run_test(method: CompareMethod, groups: &[&[f64]]) -> (&'static str, Option<f64>) {
    use anofox_statistics::{Alternative, AnovaKind, TTestKind};
    let k = groups.len();
    let resolved = match method {
        CompareMethod::Auto if k == 2 => CompareMethod::Wilcoxon,
        CompareMethod::Auto => CompareMethod::Kruskal,
        CompareMethod::Wilcoxon if k > 2 => CompareMethod::Kruskal,
        CompareMethod::TTest if k > 2 => CompareMethod::Anova,
        m => m,
    };
    match resolved {
        CompareMethod::Wilcoxon => (
            "Wilcoxon",
            anofox_statistics::mann_whitney_u(
                groups[0],
                groups[1],
                Alternative::TwoSided,
                true,
                false,
                None,
                None,
            )
            .ok()
            .map(|r| r.p_value),
        ),
        CompareMethod::TTest => (
            "T-test",
            anofox_statistics::t_test(
                groups[0],
                groups[1],
                TTestKind::Welch,
                Alternative::TwoSided,
                0.0,
                None,
            )
            .ok()
            .map(|r| r.p_value),
        ),
        CompareMethod::Anova => (
            "Anova",
            anofox_statistics::one_way_anova(groups, AnovaKind::Fisher)
                .ok()
                .map(|r| r.p_value),
        ),
        // Kruskal (and the resolved-Auto/​fallback cases).
        _ => (
            "Kruskal-Wallis",
            anofox_statistics::kruskal_wallis(groups)
                .ok()
                .map(|r| r.p_value),
        ),
    }
}

/// Two-sample p-value for one pairwise comparison, honouring `method`: `TTest`
/// uses a Welch t-test; every other method (incl. `Auto`/`Kruskal`/`Anova`,
/// which aren't pairwise) falls back to the Wilcoxon rank-sum (Mann-Whitney U),
/// matching ggpubr's default pairwise test.
pub(crate) fn pairwise_p(method: CompareMethod, a: &[f64], b: &[f64]) -> Option<f64> {
    use anofox_statistics::{Alternative, TTestKind};
    match method {
        CompareMethod::TTest => {
            anofox_statistics::t_test(a, b, TTestKind::Welch, Alternative::TwoSided, 0.0, None)
                .ok()
                .map(|r| r.p_value)
        }
        _ => {
            anofox_statistics::mann_whitney_u(a, b, Alternative::TwoSided, true, false, None, None)
                .ok()
                .map(|r| r.p_value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(groups: &[(&str, &[f64])]) -> DataFrame {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for (name, vals) in groups {
            for &v in *vals {
                xs.push(Value::Str((*name).to_string()));
                ys.push(Value::Float(v));
            }
        }
        let mut df = DataFrame::new();
        df.add_column("x".into(), xs);
        df.add_column("y".into(), ys);
        df
    }

    #[test]
    fn two_groups_auto_uses_wilcoxon() {
        let df = frame(&[
            ("a", &[1.0, 2.0, 3.0, 4.0, 5.0]),
            ("b", &[6.0, 7.0, 8.0, 9.0, 10.0]),
        ]);
        let out = StatCompareMeans::default().compute_group(&df, &ScaleSet::new());
        assert_eq!(out.nrows(), 1);
        let s = match out.column("label").unwrap()[0].clone() {
            Value::Str(s) => s,
            _ => unreachable!(),
        };
        assert!(s.starts_with("Wilcoxon, p"), "got {s}");
    }

    #[test]
    fn three_groups_auto_uses_kruskal() {
        let df = frame(&[
            ("a", &[1.0, 2.0, 3.0]),
            ("b", &[4.0, 5.0, 6.0]),
            ("c", &[7.0, 8.0, 9.0]),
        ]);
        let out = StatCompareMeans::default().compute_group(&df, &ScaleSet::new());
        let s = match out.column("label").unwrap()[0].clone() {
            Value::Str(s) => s,
            _ => unreachable!(),
        };
        assert!(s.starts_with("Kruskal-Wallis, p"), "got {s}");
    }

    #[test]
    fn single_group_returns_empty() {
        let df = frame(&[("a", &[1.0, 2.0, 3.0])]);
        let out = StatCompareMeans::default().compute_group(&df, &ScaleSet::new());
        assert_eq!(out.nrows(), 0);
    }
}
