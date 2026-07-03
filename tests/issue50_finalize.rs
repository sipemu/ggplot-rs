//! Issue #50 — plot.title.position, plot.tag.position (+ legend.direction).
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".to_string(),
            (0..10).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..10).map(|i| Value::Float((i % 3) as f64)).collect(),
        ),
    ]
}

fn tag_y(pos: TagPosition) -> f64 {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .tag("TAGMARK")
        .tag_position(pos)
        .render_svg_with_size(400, 300)
        .unwrap();
    let i = svg.find("TAGMARK").unwrap();
    let seg = &svg[svg[..i].rfind("<text").unwrap()..i];
    let yi = seg.find("y=\"").unwrap() + 3;
    seg[yi..].split('"').next().unwrap().parse().unwrap()
}

#[test]
fn tag_position_moves_the_tag() {
    assert!(
        tag_y(TagPosition::TopLeft) < tag_y(TagPosition::BottomLeft),
        "bottom tag should be lower than top tag"
    );
}

fn title_x(pos: TitlePosition) -> f64 {
    let mut t = theme_minimal();
    t.title.hjust = 0.0; // left-align so position matters
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme(t)
        .title_position(pos)
        .title("TITLEMARK")
        .render_svg_with_size(400, 300)
        .unwrap();
    let i = svg.find("TITLEMARK").unwrap();
    let seg = &svg[svg[..i].rfind("<text").unwrap()..i];
    let xi = seg.find("x=\"").unwrap() + 3;
    seg[xi..].split('"').next().unwrap().parse().unwrap()
}

#[test]
fn title_position_plot_is_left_of_panel() {
    // Plot-width title (left edge = plot margin) sits left of the panel-left title.
    assert!(
        title_x(TitlePosition::Plot) < title_x(TitlePosition::Panel),
        "plot-position title should start further left"
    );
}
