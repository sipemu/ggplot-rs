//! End-to-end: geom_boxplot + stat_compare_means() renders a p-value (ggpubr).
#![cfg(feature = "ggpubr")]

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

/// Grouped data: `n` categories, each an offset ramp.
fn grouped(categories: &[(&str, f64)]) -> Vec<(String, Vec<Value>)> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for (name, base) in categories {
        for i in 0..8 {
            xs.push(Value::Str((*name).to_string()));
            ys.push(Value::Float(base + i as f64 + ((i * 3 % 4) as f64 - 1.5)));
        }
    }
    vec![("grp".to_string(), xs), ("val".to_string(), ys)]
}

#[test]
fn two_groups_render_wilcoxon_label() {
    let svg = GGPlot::new(grouped(&[("a", 0.0), ("b", 10.0)]))
        .aes(Aes::new().x("grp").y("val"))
        .geom_boxplot()
        .stat_compare_means()
        .render_svg()
        .expect("render");
    assert!(
        svg.contains("Wilcoxon"),
        "expected Wilcoxon label for 2 groups"
    );
    assert!(
        svg.contains("p =") || svg.contains("p &lt;") || svg.contains("p <"),
        "expected a p-value"
    );
}

#[test]
fn three_groups_render_kruskal_label() {
    let svg = GGPlot::new(grouped(&[("a", 0.0), ("b", 5.0), ("c", 12.0)]))
        .aes(Aes::new().x("grp").y("val"))
        .geom_boxplot()
        .stat_compare_means()
        .render_svg()
        .expect("render");
    assert!(
        svg.contains("Kruskal-Wallis"),
        "expected Kruskal-Wallis label for 3 groups"
    );
}

#[test]
fn method_override_uses_anova() {
    let stat = StatCompareMeans::new(CompareMethod::Anova);
    let svg = GGPlot::new(grouped(&[("a", 0.0), ("b", 5.0), ("c", 12.0)]))
        .aes(Aes::new().x("grp").y("val"))
        .geom_boxplot()
        .stat_compare_means_with(stat)
        .render_svg()
        .expect("render");
    assert!(svg.contains("Anova"), "expected ANOVA label");
}
