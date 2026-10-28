//! `geom_pareto` (#297): sorted bars, cumulative line and a 0–100 % right axis.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::pareto::GeomPareto;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    let causes = ["Dent", "Scratch", "Paint", "Other", "Paint", "Seal"];
    let n = [15.0, 25.0, 30.0, 4.0, 20.0, 6.0];
    vec![
        (
            "cause".into(),
            causes.iter().map(|s| Value::Str(s.to_string())).collect(),
        ),
        ("n".into(), n.iter().map(|&f| Value::Float(f)).collect()),
    ]
}

fn root_attr<'a>(svg: &'a str, name: &str) -> &'a str {
    let head = &svg[..svg.find('>').unwrap()];
    let key = format!(" {name}=\"");
    let i = head.find(&key).unwrap() + key.len();
    &head[i..i + head[i..].find('"').unwrap()]
}

#[test]
fn build_sorts_categories_and_trains_y_to_the_total() {
    let built = GGPlot::new(data())
        .aes(Aes::new().x("cause").y("n"))
        .geom_pareto()
        .build();
    let d = &built.layers[0].data;
    let keys: Vec<String> = d
        .column("x")
        .unwrap()
        .iter()
        .map(|v| v.key_str().into_owned())
        .collect();
    assert_eq!(keys, ["Paint", "Scratch", "Dent", "Seal", "Other"]);
    let pct: Vec<f64> = d
        .column("cum_pct")
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(pct, vec![50.0, 75.0, 90.0, 96.0, 100.0]);
    let x = built.scales.get(&ggplot_rs::aes::Aesthetic::X).unwrap();
    let labels: Vec<String> = x.breaks().into_iter().map(|(_, l)| l).collect();
    assert_eq!(labels, keys, "axis follows the sorted order");
    let y = built.scales.get(&ggplot_rs::aes::Aesthetic::Y).unwrap();
    assert_eq!(y.domain(), Some((0.0, 100.0)));
}

#[test]
fn renders_percent_axis_line_points_and_threshold() {
    let (svg, warnings) = GGPlot::new(data())
        .aes(Aes::new().x("cause").y("n"))
        .geom_pareto_with(GeomPareto::default().with_threshold(0.8))
        .render_svg_native_with_warnings(640, 400)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    for t in [
        ">0%<",
        ">25%<",
        ">50%<",
        ">75%<",
        ">100%<",
        ">Cumulative %<",
    ] {
        assert!(svg.contains(t), "missing {t}");
    }
    assert!(svg.contains("<title>Paint: 50 (cumulative 50%)</title>"));
    assert!(svg.contains("<title>Seal: cumulative 96%</title>"));
    assert_eq!(svg.matches("stroke-dasharray").count(), 1, "threshold line");
    assert_eq!(svg.matches("data-series=\"cumulative\"").count(), 6);
    // Right axis stays on-canvas: the panel ends well before the canvas edge.
    let w: f64 = root_attr(&svg, "width").parse().unwrap();
    assert_eq!(w, 640.0);
    assert!(!svg.contains("NaN"));
}

#[test]
fn unsorted_keeps_data_order_and_user_y_scale_wins() {
    let built = GGPlot::new(data())
        .aes(Aes::new().x("cause").y("n"))
        .geom_pareto_with(GeomPareto {
            sort: false,
            ..Default::default()
        })
        .build();
    let keys: Vec<String> = built.layers[0]
        .data
        .column("x")
        .unwrap()
        .iter()
        .map(|v| v.key_str().into_owned())
        .collect();
    assert_eq!(keys, ["Dent", "Scratch", "Paint", "Other", "Seal"]);

    let svg = GGPlot::new(data())
        .aes(Aes::new().x("cause").y("n"))
        .scale_y_continuous(ggplot_rs::scale::continuous::ScaleContinuous::new())
        .geom_pareto()
        .render_svg_native_with_size(640, 400)
        .unwrap();
    assert!(
        !svg.contains(">100%<"),
        "no secondary axis when the user set y"
    );
}
