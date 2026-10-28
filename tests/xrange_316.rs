//! `geom_xrange` / Gantt (#316).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::xrange::GeomXrange;
use ggplot_rs::GGPlot;

fn plan() -> Vec<(String, Vec<Value>)> {
    let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
    let f = |v: &[f64]| v.iter().map(|&x| Value::Float(x)).collect();
    vec![
        ("task".into(), s(&["Design", "Build", "Ship"])),
        ("start".into(), f(&[0.0, 4.0, 10.0])),
        ("end".into(), f(&[5.0, 10.0, 10.0])),
        ("done".into(), f(&[1.0, 0.5, 0.0])),
        (
            "after".into(),
            vec![
                Value::Na,
                Value::Str("Design".into()),
                Value::Str("Build".into()),
            ],
        ),
    ]
}

#[test]
fn bars_progress_milestone_and_dependencies() {
    let (svg, warnings) = GGPlot::new(plan())
        .aes(Aes::new().y("task").xmin("start").xmax("end"))
        .geom_xrange_with(
            GeomXrange::default()
                .with_progress("done")
                .with_dependencies("after"),
        )
        .render_svg_native_with_warnings(600, 300)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>Build: 4 – 10 (50% done)</title>"));
    assert!(svg.contains("<title>Ship: 10</title>"), "milestone");
    assert!(svg.contains("<title>Design → Build</title>"));
    assert!(svg.contains("<title>Build → Ship</title>"));
    assert_eq!(svg.matches("data-part=\"arrow\"").count(), 2);
}

#[test]
fn first_task_is_on_top() {
    let built = GGPlot::new(plan())
        .aes(Aes::new().y("task").xmin("start").xmax("end"))
        .geom_xrange()
        .build();
    let y = built.scales.get(&ggplot_rs::aes::Aesthetic::Y).unwrap();
    let order: Vec<String> = y.breaks().into_iter().map(|b| b.1).collect();
    assert_eq!(order, ["Ship", "Build", "Design"], "bottom to top");
}

#[test]
fn date_ranges_get_a_date_axis() {
    use ggplot_rs::stat::calendar::days_from_civil;
    let d = |m: u32| Value::DateTime(days_from_civil(2024, m, 1) * 86_400);
    let data = vec![
        (
            "task".to_string(),
            vec![Value::Str("A".into()), Value::Str("B".into())],
        ),
        ("s".to_string(), vec![d(1), d(3)]),
        ("e".to_string(), vec![d(4), d(6)]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().y("task").xmin("s").xmax("e"))
        .geom_xrange()
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert!(svg.contains("<title>A: 2024-01-01 – 2024-04-01</title>"));
    assert!(svg.contains(">2024-"), "date axis labels");
    assert!(!svg.contains(">17"), "no epoch-second labels");
}
