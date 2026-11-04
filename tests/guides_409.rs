//! #409: per-aesthetic guides — hide one legend, title another.

use ggplot_rs::guide::config::GuideLegend;
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        ("x".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        (
            "g".into(),
            vec![Value::Str("a".into()), Value::Str("b".into())],
        ),
        (
            "h".into(),
            vec![Value::Str("p".into()), Value::Str("q".into())],
        ),
    ]
}

fn render(p: GGPlot) -> String {
    p.render_svg_native_with_size(500, 300).unwrap()
}

#[test]
fn guide_none_drops_only_that_legend_and_titles_apply() {
    let base = || {
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y").color("g").shape("h"))
            .geom_point()
    };
    let both = render(base());
    assert!(both.contains(">p<") && both.contains(">a<"));
    let svg = render(
        base()
            .guide_none(Aesthetic::Shape)
            .guide_title(Aesthetic::Color, "Group"),
    );
    assert!(!svg.contains(">p<"), "shape legend dropped");
    assert!(
        svg.contains(">a<") && svg.contains(">Group<"),
        "colour legend titled"
    );
    // A later `guides(..)` keeps them.
    let svg = render(
        base()
            .guide_none(Aesthetic::Shape)
            .guides(GuideLegend::new().reverse()),
    );
    assert!(!svg.contains(">p<"));
}
