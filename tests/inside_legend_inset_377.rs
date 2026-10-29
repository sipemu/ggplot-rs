//! sipemu/ggplot-rs#377: an inside legend sat flush on the panel edge
//! (ignoring `legend_margin`) and, in a top corner, its title — drawn
//! centred on the legend origin — stuck half outside the panel.

use ggplot_rs::data::Value;
use ggplot_rs::guide::config::GuideLegend;
use ggplot_rs::prelude::*;

fn plot(discrete: bool) -> GGPlot {
    let n = 12;
    let fill: Vec<Value> = (0..n)
        .map(|i| {
            if discrete {
                Value::Str(["a", "b", "c"][i % 3].into())
            } else {
                Value::Float(i as f64)
            }
        })
        .collect();
    GGPlot::new(vec![
        ("x".into(), (0..n).map(|i| Value::Float(i as f64)).collect()),
        (
            "y".into(),
            (0..n).map(|i| Value::Float((i * 7 % 5) as f64)).collect(),
        ),
        ("v".into(), fill),
    ])
    .aes(Aes::new().x("x").y("y").color("v"))
    .geom_point()
    .guides(GuideLegend::new().with_title("Sales (€)"))
}

/// `x y w h` of the panel.
fn panel(svg: &str) -> (f64, f64, f64, f64) {
    let i = svg.find("data-plot=\"").unwrap() + 11;
    let v: Vec<f64> = svg[i..]
        .split('"')
        .next()
        .unwrap()
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    (v[0], v[1], v[2], v[3])
}

/// `(body, x, y, font-size)` of every `<text>` after the panel's marks.
fn texts(svg: &str) -> Vec<(String, f64, f64, f64)> {
    svg.split("<text")
        .skip(1)
        .map(|t| {
            let open = t.split('>').next().unwrap();
            let a = |n: &str| -> f64 {
                open.split(&format!(" {n}=\""))
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap()
            };
            let body = t
                .split('>')
                .nth(1)
                .unwrap()
                .split('<')
                .next()
                .unwrap()
                .to_string();
            (body, a("x"), a("y"), a("font-size"))
        })
        .collect()
}

#[test]
fn inside_legends_are_inset_and_keep_their_title_inside() {
    // theme_gray's legend_margin: top 10, right 15, bottom 10, left 10.
    for discrete in [false, true] {
        for (fx, fy) in [(0.0, 1.0), (1.0, 1.0), (0.0, 0.0), (1.0, 0.0)] {
            let svg = plot(discrete)
                .legend_position_inside(fx, fy)
                .render_svg_native_with_size(360, 300)
                .unwrap();
            let (px, py, pw, ph) = panel(&svg);
            let t = texts(&svg);
            let title = t.iter().find(|x| x.0 == "Sales (€)").expect("title");
            let case = format!("discrete={discrete} ({fx},{fy})");
            // The title's top half stays inside the inset panel.
            assert!(
                title.2 - title.3 / 2.0 >= py + 10.0 - 0.01,
                "{case}: title top {title:?}"
            );
            if fx == 0.0 {
                assert!(
                    (title.1 - (px + 10.0)).abs() < 0.01,
                    "{case}: left inset {title:?}"
                );
            }
            // Every legend label stays above the bottom inset.
            // Legend labels follow the title (axes are drawn before it).
            let at = t.iter().position(|x| x.0 == "Sales (€)").unwrap();
            let labels = &t[at + 1..];
            assert!(!labels.is_empty(), "{case}");
            for l in labels {
                assert!(
                    l.2 + l.3 / 2.0 <= py + ph - 10.0 + 0.5,
                    "{case}: label {l:?} panel {:?} title {title:?}",
                    (px, py, pw, ph)
                );
                assert!(l.1 <= px + pw, "{case}: label {l:?}");
            }
        }
    }
}
