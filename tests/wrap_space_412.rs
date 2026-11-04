//! #412: `facet_wrap(space = "free_x")` in a single row, and
//! `plot.caption.position` (#413).

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    // Panel a spans x 0–10, panel b 0–30.
    let xs = [0.0, 10.0, 0.0, 30.0];
    vec![
        ("x".into(), xs.iter().map(|&v| Value::Float(v)).collect()),
        ("y".into(), xs.iter().map(|&v| Value::Float(v)).collect()),
        (
            "g".into(),
            ["a", "a", "b", "b"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
    ]
}

fn panel_widths(svg: &str) -> Vec<f64> {
    svg.split("<rect")
        .skip(1)
        .filter(|t| t.contains("fill=\"#EBEBEB\""))
        .map(|t| {
            let a = t.find(" width=\"").unwrap() + 8;
            t[a..].split('"').next().unwrap().parse().unwrap()
        })
        .collect()
}

#[test]
fn single_row_wrap_sizes_by_x_range() {
    let p = || {
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .facet_wrap_free("g", Some(2), FacetScales::FreeX)
    };
    let fixed = panel_widths(&p().render_svg_native_with_size(500, 300).unwrap());
    let free = panel_widths(
        &p().facet_wrap_space(FacetSpace::FreeX)
            .render_svg_native_with_size(500, 300)
            .unwrap(),
    );
    assert!((fixed[0] - fixed[1]).abs() < 0.05, "{fixed:?}");
    assert!((free[1] / free[0] - 3.0).abs() < 0.05, "{free:?}");
}

#[test]
fn caption_position_plot() {
    let p = |t: Theme| {
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .caption("Source")
            .theme(t)
            .render_svg_native_with_size(400, 300)
            .unwrap()
    };
    let x_of = |s: &str| -> f64 {
        let t = s.split("<text").find(|t| t.contains(">Source<")).unwrap();
        let a = t.find(" x=\"").unwrap() + 4;
        t[a..].split('"').next().unwrap().parse().unwrap()
    };
    // Left-aligned: to the plot's left edge, not the panel's (past the y axis).
    let mut panel = theme_gray();
    panel.caption.hjust = 0.0;
    let mut plot = panel.clone();
    plot.caption_position = Some(TitlePosition::Plot);
    assert!(x_of(&p(plot)) < x_of(&p(panel)));
}
