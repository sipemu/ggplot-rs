//! `geom_waffle` / `geom_parliament` (#308).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "party".into(),
            ["Left", "Centre", "Right"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
        (
            "seats".into(),
            [41.0, 27.0, 32.0]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        ),
    ]
}

#[test]
fn waffle_draws_one_square_per_unit_with_a_legend() {
    let (svg, warnings) = GGPlot::new(data())
        .aes(Aes::new().fill("party").weight("seats"))
        .geom_waffle(10)
        .render_svg_native_with_warnings(600, 400)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(svg.matches("data-geom=\"waffle\"").count(), 100);
    assert!(svg.contains(">Centre<"), "fill legend");
    assert!(!svg.contains("NaN"));
}

#[test]
fn parliament_draws_one_seat_per_unit() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().fill("party").weight("seats"))
        .geom_parliament(0)
        .render_svg_native_with_size(600, 360)
        .unwrap();
    assert_eq!(svg.matches("data-geom=\"waffle\"").count(), 100);
    // Squares only for the waffle: seats are circles.
    assert!(svg.matches("<circle").count() >= 100);
}

#[test]
fn parliament_bottom_legend_is_drawn() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().fill("party").weight("seats"))
        .geom_parliament(0)
        .theme_void()
        .legend_position(ggplot_rs::theme::LegendPosition::Bottom)
        .render_svg_native_with_size(600, 360)
        .unwrap();
    assert!(
        svg.contains(">Centre<"),
        "{}",
        &svg[svg.len().saturating_sub(3000)..]
    );
}

#[test]
fn edge_cases_unmapped_fill_explicit_rows_and_bad_weights() {
    use ggplot_rs::geom::waffle::GeomWaffle;
    use ggplot_rs::stat::waffle::UnitLayout;
    // No fill mapping: one category, the geom's own fill; one unit per row.
    let svg = GGPlot::new(data())
        .geom_waffle(4)
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert_eq!(svg.matches("data-geom=\"waffle\"").count(), 3);
    // Explicit arcs, a single arc, rounded squares and a custom gap.
    for layout in [
        UnitLayout::Parliament {
            rows: 3,
            inner: 0.3,
        },
        UnitLayout::Parliament {
            rows: 1,
            inner: 0.5,
        },
        UnitLayout::Waffle { rows: 0 },
    ] {
        let svg = GGPlot::new(data())
            .aes(Aes::new().fill("party").weight("seats"))
            .geom_waffle_with(GeomWaffle {
                layout,
                gap: 0.2,
                roundness: 0.3,
                ..Default::default()
            })
            .render_svg_native_with_size(400, 300)
            .unwrap();
        assert_eq!(
            svg.matches("data-geom=\"waffle\"").count(),
            100,
            "{layout:?}"
        );
        assert!(!svg.contains("NaN"));
    }
    // Missing / non-positive weights contribute no units.
    let bad = vec![
        (
            "party".to_string(),
            ["A", "B", "C"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
        (
            "seats".to_string(),
            vec![Value::Float(3.0), Value::Na, Value::Float(-2.0)],
        ),
    ];
    let svg = GGPlot::new(bad)
        .aes(Aes::new().fill("party").weight("seats"))
        .geom_parliament(0)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(svg.matches("data-geom=\"waffle\"").count(), 3);
    let p = GeomWaffle::parliament(2);
    assert!(matches!(p.layout, UnitLayout::Parliament { rows: 2, .. }));
}
