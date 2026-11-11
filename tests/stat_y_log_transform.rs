//! #490: a stat-computed `y` (a histogram's count, a bar's count) goes through
//! the y scale's transform, as ggplot2's `map_statistic` does — on
//! `scale_y_log10` the ticks read counts, not 10^count.
use ggplot_rs::prelude::*;

fn ys(built: &ggplot_rs::build::BuiltPlot) -> Vec<f64> {
    built.layers[0]
        .data
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect()
}

fn tick_labels(svg: &str) -> Vec<String> {
    svg.split("<text")
        .skip(1)
        .filter_map(|t| Some(t[t.find('>')? + 1..t.find("</text>")?].to_string()))
        .collect()
}

#[test]
fn histogram_counts_are_log_transformed() {
    // 1000 values over 10 equal-width bins: ~100 per bin.
    let x: Vec<Value> = (0..1000)
        .map(|i| Value::Float((i % 100) as f64 / 10.0))
        .collect();
    let plot = || {
        GGPlot::new(vec![("x".to_string(), x.clone())])
            .aes(Aes::new().x("x"))
            .geom_histogram_with(GeomHistogram {
                bins: 10,
                ..GeomHistogram::default()
            })
            .scale_y_log10()
    };
    let y = ys(&plot().build());
    assert!(!y.is_empty());
    assert!(
        y.iter().all(|v| (1.5..=2.5).contains(v)),
        "log10(~100): {y:?}"
    );
    let svg = plot().render_svg_native_with_size(600, 400).unwrap();
    let labels = tick_labels(&svg);
    assert!(
        labels.iter().any(|l| l == "100"),
        "a count tick: {labels:?}"
    );
    assert!(
        !labels
            .iter()
            .any(|l| l.len() > 6 && l.chars().all(|c| c.is_ascii_digit())),
        "no 10^count ticks: {labels:?}"
    );
}

#[test]
fn bar_count_and_after_stat_y_are_transformed() {
    let g: Vec<Value> = (0..110)
        .map(|i| Value::Str(if i < 100 { "a" } else { "b" }.into()))
        .collect();
    let built = GGPlot::new(vec![("g".to_string(), g.clone())])
        .aes(Aes::new().x("g"))
        .geom_bar()
        .scale_y_log10()
        .build();
    let mut y = ys(&built);
    y.sort_by(f64::total_cmp);
    assert!(
        (y[0] - 1.0).abs() < 1e-9 && (y[1] - 2.0).abs() < 1e-9,
        "{y:?}"
    );

    let built = GGPlot::new(vec![("g".to_string(), g)])
        .aes(Aes::new().x("g").after_stat_y("count / 10"))
        .geom_bar()
        .scale_y_log10()
        .build();
    let mut y = ys(&built);
    y.sort_by(f64::total_cmp);
    assert!(y[0].abs() < 1e-9 && (y[1] - 1.0).abs() < 1e-9, "{y:?}");
}

#[test]
fn mapped_y_is_transformed_once() {
    // geom_col: y is mapped and transformed before the (identity) stat only.
    let built = GGPlot::new(vec![
        (
            "g".to_string(),
            vec![Value::Str("a".into()), Value::Str("b".into())],
        ),
        (
            "v".to_string(),
            vec![Value::Float(10.0), Value::Float(1000.0)],
        ),
    ])
    .aes(Aes::new().x("g").y("v"))
    .geom_col()
    .scale_y_log10()
    .build();
    let mut y = ys(&built);
    y.sort_by(f64::total_cmp);
    assert_eq!(y, vec![1.0, 3.0]);
}
