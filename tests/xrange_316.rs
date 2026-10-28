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

#[test]
fn overlapping_dependency_missing_predecessor_fill_and_primary() {
    let s = |v: &[&str]| {
        v.iter()
            .map(|x| Value::Str(x.to_string()))
            .collect::<Vec<_>>()
    };
    let f = |v: &[f64]| v.iter().map(|&x| Value::Float(x)).collect::<Vec<_>>();
    let data = vec![
        ("task".to_string(), s(&["A", "B", "C"])),
        ("start".to_string(), f(&[0.0, 2.0, 6.0])),
        ("end".to_string(), f(&[5.0, 8.0, 9.0])),
        ("team".to_string(), s(&["x", "y", "x"])),
        // B starts before A ends (S-shaped route); C names an unknown task.
        ("after".to_string(), s(&["", "A", "Nope"])),
    ];
    let svg = GGPlot::new(data.clone())
        .aes(Aes::new().y("task").xmin("start").xmax("end").fill("team"))
        .geom_xrange_with(GeomXrange::default().with_dependencies("after"))
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert!(svg.contains("<title>A → B</title>"));
    assert!(!svg.contains("Nope →"), "unknown predecessor is skipped");
    // The overlapping route bends back: a 6-point path.
    let path = svg
        .match_indices("<polyline")
        .map(|(i, _)| &svg[i..i + svg[i..].find('>').unwrap()])
        .find(|t| t.contains("data-geom=\"xrange\""))
        .unwrap();
    let p = path.find("points=\"").unwrap() + 8;
    assert_eq!(
        path[p..p + path[p..].find('"').unwrap()].split(' ').count(),
        6
    );
    let svg = GGPlot::new(data)
        .aes(Aes::new().y("task").xmin("start").xmax("end"))
        .geom_xrange()
        .primary_color((200, 0, 100))
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert!(svg.contains("#C80064"));
}
