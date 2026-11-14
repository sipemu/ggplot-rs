//! A smooth layer's fit per colour group is drawn as one line per group, in
//! that group's colour: the stat's output keeps the group's columns
//! (DataZooDE/anofox-visualization#1121, sample 3: lm and loess lost them,
//! and every group's fit became one zigzag path).
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let f = |v: &[f64]| v.iter().map(|x| Value::Float(*x)).collect::<Vec<_>>();
    let s = |v: &[&str]| {
        v.iter()
            .map(|x| Value::Str(x.to_string()))
            .collect::<Vec<_>>()
    };
    vec![
        (
            "x".to_string(),
            f(&[1., 2., 3., 4., 5., 1., 2., 3., 4., 5.]),
        ),
        (
            "y".to_string(),
            f(&[10., 12., 11., 14., 13., 50., 52., 51., 55., 54.]),
        ),
        (
            "g".to_string(),
            s(&["a", "a", "a", "a", "a", "b", "b", "b", "b", "b"]),
        ),
    ]
}

fn smooth_lines(method: SmoothMethod) -> Vec<String> {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_smooth_with(GeomSmooth {
            se: false,
            method,
            ..Default::default()
        })
        .theme_void()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    svg.split("<polyline")
        .skip(1)
        .filter(|p| p.contains("data-geom=\"smooth\""))
        .map(|p| {
            let stroke = p.split("stroke=\"").nth(1).unwrap_or("");
            stroke[..stroke.find('"').unwrap_or(0)].to_string()
        })
        .collect()
}

#[test]
fn lm_and_loess_draw_one_line_per_group() {
    for method in [
        SmoothMethod::Lm,
        SmoothMethod::Loess { span: 0.9 },
        SmoothMethod::Gam,
    ] {
        let lines = smooth_lines(method.clone());
        assert_eq!(lines.len(), 2, "{method:?}: {lines:?}");
        assert_ne!(lines[0], lines[1], "{method:?}: each group in its colour");
    }
}

/// #507: `aes(group = g)` alone splits a line, a path and a smooth per group.
#[test]
fn a_group_alone_splits_lines_paths_and_smooths() {
    let count = |plot: GGPlot| {
        plot.theme_void()
            .render_svg_native_with_size(300, 200)
            .unwrap()
            .matches("<polyline")
            .count()
    };
    let p = || GGPlot::new(data()).aes(Aes::new().x("x").y("y").group("g"));
    assert_eq!(count(p().geom_line()), 2);
    assert_eq!(count(p().geom_path()), 2);
    assert_eq!(
        count(p().geom_smooth_with(GeomSmooth {
            se: false,
            method: SmoothMethod::Lm,
            ..Default::default()
        })),
        2
    );
}
