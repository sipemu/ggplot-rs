//! #411: layers are clipped to the panel (ggplot2's `clip = "on"`) when they
//! reach past it — a zoomed coord — and `coord_clip(false)` (`clip = "off"`)
//! lets them draw past it. Plots whose data fits emit no clip group, so
//! their output is unchanged.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".to_string(),
            (0..10).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..10).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "g".to_string(),
            (0..10)
                .map(|i| Value::Str(if i < 5 { "a" } else { "b" }.into()))
                .collect(),
        ),
    ]
}

fn zoomed() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .geom_point()
        .coord_cartesian_zoom(Some((2.0, 6.0)), None)
}

fn render(p: GGPlot) -> String {
    p.render_svg_native_with_size(400, 300).unwrap()
}

#[test]
fn zoomed_layers_are_clipped_to_the_panel() {
    let svg = render(zoomed());
    let plot: Vec<f64> = {
        let i = svg.find("data-plot=\"").unwrap() + 11;
        svg[i..]
            .split('"')
            .next()
            .unwrap()
            .split(' ')
            .map(|n| n.parse().unwrap())
            .collect()
    };
    let clip = format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\"/></clipPath>",
        plot[0], plot[1], plot[2], plot[3]
    );
    assert!(svg.contains(&clip), "{svg}");
    // Every point and the line sit inside the clip group.
    let group = svg.find("<g clip-path=\"url(#").unwrap();
    assert!(svg.find("<circle").unwrap() > group);
}

#[test]
fn clip_off_draws_past_the_panel() {
    let svg = render(zoomed().coord_clip(false));
    assert!(!svg.contains("clip-path"));
    assert_eq!(svg.matches("<circle").count(), 10);
}

#[test]
fn fitting_data_gets_no_clip_group() {
    let svg = render(
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point(),
    );
    assert!(!svg.contains("clip-path"));
}

#[test]
fn zoomed_facet_panels_are_clipped_each() {
    let svg = render(zoomed().facet_wrap("g", None));
    assert_eq!(svg.matches("<g clip-path=\"url(#").count(), 2, "{svg}");
}

#[test]
fn segment_ends_train_the_position_scales() {
    let built = GGPlot::new(vec![
        ("x".to_string(), vec![Value::Float(0.0)]),
        ("y".to_string(), vec![Value::Float(0.0)]),
        ("xe".to_string(), vec![Value::Float(10.0)]),
        ("ye".to_string(), vec![Value::Float(-4.0)]),
    ])
    .aes(Aes::new().x("x").y("y").xend("xe").yend("ye"))
    .geom_segment()
    .build();
    let (x0, x1) = built.scales.get(&Aesthetic::X).unwrap().domain().unwrap();
    let (y0, y1) = built.scales.get(&Aesthetic::Y).unwrap().domain().unwrap();
    assert!(x0 <= 0.0 && x1 >= 10.0, "{x0} {x1}");
    assert!(y0 <= -4.0 && y1 >= 0.0, "{y0} {y1}");
}
