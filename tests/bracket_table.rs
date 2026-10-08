//! Table-driven significance brackets (`geom_bracket_table`) on the
//! plotters-free native SVG path, fed the anofox `test` contract schema.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

fn grouped() -> Vec<(String, Vec<Value>)> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for (name, base) in [("ctrl", 5.0), ("trt1", 8.0), ("trt2", 6.5)] {
        for i in 0..8 {
            xs.push(Value::Str(name.to_string()));
            ys.push(Value::Float(base + (i % 4) as f64 - 1.5));
        }
    }
    vec![("grp".to_string(), xs), ("val".to_string(), ys)]
}

fn s(v: &str) -> Value {
    Value::Str(v.to_string())
}

/// Output of a pairwise post-hoc test in the `test` contract shape (every
/// optional column present, NULL where not computed).
fn contract_tests() -> Vec<(String, Vec<Value>)> {
    vec![
        ("method".into(), vec![s("t_test"), s("t_test"), s("t_test")]),
        (
            "statistic".into(),
            vec![Value::Float(-4.2), Value::Float(-1.9), Value::Float(2.1)],
        ),
        (
            "p_value".into(),
            vec![
                Value::Float(0.0009),
                Value::Float(0.08),
                Value::Float(0.051),
            ],
        ),
        ("test_id".into(), vec![s("t1"), s("t2"), s("t3")]),
        ("group1".into(), vec![s("ctrl"), s("ctrl"), s("trt1")]),
        ("group2".into(), vec![s("trt1"), s("trt2"), s("trt2")]),
        (
            "p_adj".into(),
            vec![Value::Float(0.0027), Value::Float(0.16), Value::Na],
        ),
        ("df1".into(), vec![Value::Na, Value::Na, Value::Na]),
        ("df2".into(), vec![Value::Na, Value::Na, Value::Na]),
        (
            "estimate".into(),
            vec![Value::Float(-3.0), Value::Float(-1.5), Value::Float(1.5)],
        ),
        ("conf_low".into(), vec![Value::Na, Value::Na, Value::Na]),
        ("conf_high".into(), vec![Value::Na, Value::Na, Value::Na]),
        ("alternative".into(), vec![s("two.sided"); 3]),
        ("alpha".into(), vec![Value::Float(0.05); 3]),
    ]
}

fn base() -> GGPlot {
    GGPlot::new(grouped())
        .aes(Aes::new().x("grp").y("val"))
        .geom_boxplot()
}

#[test]
fn contract_table_renders_templated_labels() {
    let svg = base()
        .geom_bracket_table(contract_tests(), BracketTable::new().label("p = {p}"))
        .render_svg_native_with_size(500, 400)
        .unwrap();
    // p_adj where present, p_value where p_adj is NULL.
    assert!(svg.contains(">p = 0.0027<"), "{svg}");
    assert!(svg.contains(">p = 0.16<"));
    assert!(svg.contains(">p = 0.051<"));
    // Three bracket polylines, carrying host hover metadata.
    assert!(svg.contains("data-series=\"t1\""));
    assert!(svg.contains("data-value=\"0.0027\""));
    assert!(svg.contains("data-x=\"ctrl vs trt1\""));
}

#[test]
fn stars_and_hide_ns() {
    let svg = base()
        .geom_bracket_table(contract_tests(), BracketTable::new().stars().hide_ns(true))
        .render_svg_native()
        .unwrap();
    assert!(svg.contains(">**<"), "p_adj 0.0027 → **");
    assert!(!svg.contains(">ns<"), "ns rows hidden");
    assert!(svg.contains("data-series=\"t1\""));
    assert!(!svg.contains("data-series=\"t2\"") && !svg.contains("data-series=\"t3\""));
}

#[test]
fn auto_stacking_raises_the_y_domain() {
    let (svg, warnings) = base()
        .geom_bracket_table(contract_tests(), BracketTable::new())
        .render_svg_native_with_warnings(500, 400)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    // Data max is 9.5 and range 6; three stacked brackets end at
    // 9.5 + 3 · 0.72 = 11.66, so the trained y domain reaches above it.
    let dom = svg
        .split("data-ydomain=\"")
        .nth(1)
        .and_then(|r| r.split('"').next())
        .unwrap();
    let hi: f64 = dom.split(' ').nth(1).unwrap().parse().unwrap();
    assert!(hi > 11.66, "y domain {dom}");
}

#[test]
fn explicit_positions_and_label_column() {
    let tests = vec![
        ("group1".to_string(), vec![s("ctrl")]),
        ("group2".to_string(), vec![s("trt2")]),
        ("p_value".to_string(), vec![Value::Float(0.2)]),
        ("y_position".to_string(), vec![Value::Float(14.0)]),
        ("label".to_string(), vec![s("<b>&\"x\"")]),
    ];
    let svg = base()
        .geom_bracket_table(tests, BracketTable::new())
        .render_svg_native()
        .unwrap();
    // The label column is used verbatim but escaped.
    assert!(svg.contains("&lt;b&gt;&amp;&quot;x&quot;"));
    assert!(!svg.contains("<b>"));
}

#[test]
fn bad_rows_are_dropped_with_warnings_not_panics() {
    let tests = vec![
        ("group1".to_string(), vec![s("ctrl"), Value::Na, s("nope")]),
        ("group2".to_string(), vec![s("trt1"), s("trt1"), s("trt2")]),
        (
            "p_value".to_string(),
            vec![Value::Float(f64::NAN), Value::Float(0.1), Value::Float(0.1)],
        ),
    ];
    let (svg, warnings) = base()
        .geom_bracket_table(tests, BracketTable::new())
        .render_svg_native_with_warnings(400, 300)
        .unwrap();
    assert!(svg.contains(">p = NA<"), "NaN p prints NA");
    assert!(warnings.iter().any(|w| w.contains("missing group")));
    assert!(warnings.iter().any(|w| w.contains("nope")));
    // The unknown group did not leak onto the x axis.
    assert!(!svg.contains(">nope<"));
}

#[test]
fn missing_group_columns_warn() {
    let tests = vec![("p_value".to_string(), vec![Value::Float(0.1)])];
    let (_, warnings) = base()
        .geom_bracket_table(tests, BracketTable::new())
        .render_svg_native_with_warnings(400, 300)
        .unwrap();
    assert!(
        warnings.iter().any(|w| w.contains("group1")),
        "{warnings:?}"
    );
}

#[test]
fn ragged_table_is_rejected_with_a_warning() {
    let tests = vec![
        ("group1".to_string(), vec![s("ctrl"), s("ctrl")]),
        ("group2".to_string(), vec![s("trt1")]),
    ];
    let (_, warnings) = base()
        .geom_bracket_table(tests, BracketTable::new())
        .render_svg_native_with_warnings(400, 300)
        .unwrap();
    assert!(
        warnings.iter().any(|w| w.contains("rejected")),
        "{warnings:?}"
    );
}
