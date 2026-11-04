//! #412: `facet_grid_levels` (fixed order, empty rows / columns kept —
//! `drop = FALSE`) and `facet_margins` (ggplot2's `margins = TRUE`).

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let n = 12;
    vec![
        ("x".into(), (0..n).map(|i| Value::Float(i as f64)).collect()),
        (
            "y".into(),
            (0..n).map(|i| Value::Float((i % 5) as f64)).collect(),
        ),
        (
            "r".into(),
            (0..n)
                .map(|i| Value::Str(["a", "b"][i % 2].into()))
                .collect(),
        ),
        (
            "c".into(),
            (0..n)
                .map(|i| Value::Str(["p", "q"][(i / 2) % 2].into()))
                .collect(),
        ),
    ]
}

fn grid() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .facet_grid(Some("r"), Some("c"))
}

#[test]
fn fixed_levels_order_and_keep_empty() {
    let built = grid()
        .facet_grid_levels(Some(&["b", "a", "z"]), None::<&[&str]>)
        .build();
    // 3 rows (z empty) × 2 cols.
    assert_eq!(built.panels.len(), 6);
    let rows: Vec<_> = built
        .panels
        .iter()
        .map(|p| p.row_label.clone().unwrap())
        .collect();
    assert_eq!(&rows[..2], &["b".to_string(), "b".to_string()]);
    assert_eq!(rows[4], "z");
    let z_rows: usize = built.panels_data[4].iter().map(|d| d.nrows()).sum();
    assert_eq!(z_rows, 0);
    // Unlisted values are dropped: listing only "a" leaves 2 panels.
    let only_a = grid()
        .facet_grid_levels(Some(&["a"]), None::<&[&str]>)
        .build();
    assert_eq!(only_a.panels.len(), 2);
    let svg = grid()
        .facet_grid_levels(Some(&["b", "a", "z"]), None::<&[&str]>)
        .render_svg_native_with_size(400, 400)
        .unwrap();
    assert!(svg.contains(">z<"));
}

#[test]
fn margins_pool_rows_and_columns() {
    let built = grid().facet_margins(true).build();
    // (a, b, (all)) × (p, q, (all)).
    assert_eq!(built.panels.len(), 9);
    let n = |i: usize| -> usize { built.panels_data[i].iter().map(|d| d.nrows()).sum() };
    assert_eq!(n(8), 12, "the (all, all) panel holds every row");
    assert_eq!(n(2), 6, "row a, column (all)");
    assert_eq!(built.panels[8].row_label.as_deref(), Some("(all)"));
    // A stat runs on the pooled data: one histogram per panel, the (all)
    // panel's counts summing to every row.
    let hist = GGPlot::new(data())
        .aes(Aes::new().x("x"))
        .geom_histogram()
        .facet_grid(Some("r"), None)
        .facet_margins(true)
        .build();
    let total: f64 = hist.panels_data[2][0]
        .column("count")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .sum();
    assert_eq!(total, 12.0);
}
