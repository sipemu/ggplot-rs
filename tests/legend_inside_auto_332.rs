//! sipemu/ggplot-rs#332: `legend_inside_auto` puts the legend in an empty
//! panel corner, else outside on the side that leaves the data area larger.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::theme::{LegendCorner, LegendPosition};

fn legend_at(svg: &str) -> Option<String> {
    let i = svg.find("data-legend-at=\"")? + 16;
    Some(svg[i..].split('"').next()?.to_string())
}

/// `x y w h` of the panel (`data-plot`).
fn panel(svg: &str) -> Vec<f64> {
    let i = svg.find("data-plot=\"").unwrap() + 11;
    svg[i..]
        .split('"')
        .next()
        .unwrap()
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect()
}

/// Points along the anti-diagonal from top-left: the bottom-right corner
/// stays empty, the other three hold data.
fn scatter() -> GGPlot {
    let mut x = vec![];
    let mut y = vec![];
    for i in 0..30 {
        let t = i as f64 / 29.0;
        x.push(Value::Float(t * 0.5));
        y.push(Value::Float(1.0 - t * 0.5));
    }
    // Also fill the bottom-left and top-right corners.
    x.extend([0.0, 0.02, 1.0, 0.98].map(Value::Float));
    y.extend([0.0, 0.02, 1.0, 0.98].map(Value::Float));
    let g = (0..x.len())
        .map(|i| Value::Str(["a", "b"][i % 2].into()))
        .collect();
    GGPlot::new(vec![("x".into(), x), ("y".into(), y), ("g".into(), g)])
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
}

#[test]
fn legend_lands_in_the_empty_corner_without_shrinking_the_panel() {
    let auto = scatter()
        .legend_inside_auto()
        .render_svg_native_with_size(500, 400)
        .unwrap();
    assert_eq!(legend_at(&auto).as_deref(), Some("inside-bottom-right"));
    let none = scatter()
        .legend_position(LegendPosition::None)
        .render_svg_native_with_size(500, 400)
        .unwrap();
    assert_eq!(panel(&auto), panel(&none), "inside: no space reserved");
    assert!(legend_at(&none).is_none());
}

#[test]
fn candidates_are_honoured_and_a_full_panel_falls_back_outside() {
    // Only the (occupied) top-left corner is allowed → outside.
    let svg = scatter()
        .legend_inside_auto_with(&[LegendCorner::TopLeft], Some(LegendPosition::Bottom))
        .render_svg_native_with_size(500, 400)
        .unwrap();
    assert_eq!(legend_at(&svg).as_deref(), Some("bottom"));
}

#[cfg(feature = "sf")]
mod maps {
    use super::*;

    /// One polygon covering the whole extent: no corner is free.
    fn covered(w: f64, h: f64) -> GGPlot {
        let geom = [
            format!("POLYGON ((0 0, {w} 0, {w} {h}, 0 0))"),
            format!("POLYGON ((0 0, {w} {h}, 0 {h}, 0 0))"),
        ];
        GGPlot::new(vec![
            (
                "geometry".into(),
                geom.iter().map(|g| Value::Str(g.clone())).collect(),
            ),
            ("v".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ])
        .aes(Aes::new().fill("v"))
        .geom_sf()
        .coord_sf()
        .legend_inside_auto()
    }

    fn viewport_area(svg: &str) -> f64 {
        let i = svg.find("data-viewport=\"").expect("data-viewport") + 15;
        let v: Vec<f64> = svg[i..]
            .split('"')
            .next()
            .unwrap()
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        v[2] * v[3]
    }

    /// A fully covered map with a fixed outside legend.
    fn covered_at(w: f64, h: f64, pos: LegendPosition) -> GGPlot {
        GGPlot::new(vec![
            (
                "geometry".into(),
                vec![Value::Str(format!(
                    "POLYGON ((0 0, {w} 0, {w} {h}, 0 {h}, 0 0))"
                ))],
            ),
            ("v".into(), vec![Value::Float(1.0)]),
        ])
        .aes(Aes::new().fill("v"))
        .geom_sf()
        .coord_sf()
        .legend_position(pos)
    }

    #[test]
    fn a_covered_map_goes_outside_on_the_side_that_keeps_it_larger() {
        // Map and canvas of similar aspect: no letterbox band the legend
        // could sit in, so every corner is covered.
        for ((mw, mh), (w, h)) in [((10.0, 6.0), (500, 340)), ((6.0, 10.0), (340, 500))] {
            let svg = covered(mw, mh).render_svg_native_with_size(w, h).unwrap();
            let at = legend_at(&svg).unwrap();
            let (chosen, other) = match at.as_str() {
                "top" => (LegendPosition::Top, LegendPosition::Right),
                "right" => (LegendPosition::Right, LegendPosition::Top),
                _ => panic!("expected an outside legend, got {at}"),
            };
            let area = |pos| {
                viewport_area(
                    &covered_at(mw, mh, pos)
                        .render_svg_native_with_size(w, h)
                        .unwrap(),
                )
            };
            let (a, b) = (area(chosen), area(other));
            assert!(a >= b, "{at}: {a} < {b} ({mw}x{mh} in {w}x{h})");
        }
    }

    #[test]
    fn a_map_with_an_empty_corner_keeps_the_legend_inside() {
        // An L-shaped country: the top-right is sea.
        let svg = GGPlot::new(vec![
            (
                "geometry".into(),
                vec![Value::Str(
                    "POLYGON ((0 0, 10 0, 10 3, 3 3, 3 10, 0 10, 0 0))".into(),
                )],
            ),
            ("v".into(), vec![Value::Float(1.0)]),
        ])
        .aes(Aes::new().fill("v"))
        .geom_sf()
        .coord_sf()
        .legend_inside_auto_with(&[LegendCorner::BottomRight, LegendCorner::TopRight], None)
        .render_svg_native_with_size(400, 400)
        .unwrap();
        assert_eq!(legend_at(&svg).as_deref(), Some("inside-top-right"));
    }
}

#[test]
fn every_corner_and_fallback_side_is_reported() {
    // A free plot (one point in the middle): each single candidate wins.
    let one = || {
        GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            (
                "g".into(),
                vec![Value::Str("a".into()), Value::Str("b".into())],
            ),
        ])
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_blank()
    };
    for corner in [
        LegendCorner::TopLeft,
        LegendCorner::TopRight,
        LegendCorner::BottomLeft,
        LegendCorner::BottomRight,
    ] {
        let svg = one()
            .legend_inside_auto_with(&[corner], None)
            .render_svg_native_with_size(400, 300)
            .unwrap();
        assert_eq!(legend_at(&svg), Some(format!("inside-{}", corner.name())));
    }
    // No candidates: straight to the fallback.
    for (pos, name) in [
        (LegendPosition::Top, "top"),
        (LegendPosition::Right, "right"),
        (LegendPosition::Bottom, "bottom"),
        (LegendPosition::Left, "left"),
    ] {
        let svg = one()
            .legend_inside_auto_with(&[], Some(pos))
            .render_svg_native_with_size(400, 300)
            .unwrap();
        assert_eq!(legend_at(&svg).as_deref(), Some(name));
    }
}

#[test]
fn faceted_plots_keep_their_legend() {
    let svg = scatter()
        .facet_wrap("g", None)
        .legend_inside_auto()
        .render_svg_native_with_size(500, 400)
        .unwrap();
    assert!(legend_at(&svg).is_none());
}
