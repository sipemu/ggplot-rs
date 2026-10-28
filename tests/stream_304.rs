//! Streamgraph stack offsets (#304).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::position::stack::{position_stack, StackOffset};
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    let (mut x, mut g, mut y) = (vec![], vec![], vec![]);
    for t in 0..12 {
        for (k, name) in ["Rock", "Pop", "Jazz"].iter().enumerate() {
            x.push(Value::Float(t as f64));
            g.push(Value::Str(name.to_string()));
            y.push(Value::Float(
                5.0 + 3.0 * ((t as f64) * 0.6 + k as f64 * 2.0).sin() + k as f64,
            ));
        }
    }
    vec![("t".into(), x), ("genre".into(), g), ("n".into(), y)]
}

fn y_domain(b: &ggplot_rs::build::BuiltPlot) -> (f64, f64) {
    b.scales
        .get(&ggplot_rs::aes::Aesthetic::Y)
        .unwrap()
        .domain()
        .unwrap()
}

#[test]
fn stream_straddles_zero_and_silhouette_is_symmetric() {
    let aes = || Aes::new().x("t").y("n").fill("genre");
    let stream = GGPlot::new(data()).aes(aes()).geom_stream().build();
    let (lo, hi) = y_domain(&stream);
    assert!(lo < 0.0 && hi > 0.0, "{lo} {hi}");

    let river = GGPlot::new(data())
        .aes(aes())
        .geom_area()
        .position(position_stack().with_offset(StackOffset::Silhouette))
        .build();
    let (lo, hi) = y_domain(&river);
    assert!((lo + hi).abs() < 1e-9, "symmetric: {lo} {hi}");

    let svg = GGPlot::new(data())
        .aes(aes())
        .geom_stream()
        .render_svg_native_with_size(600, 360)
        .unwrap();
    assert!(!svg.contains("NaN"));
    assert!(svg.contains(">Jazz<"), "fill legend");
}

#[test]
fn layers_stay_contiguous_at_every_x() {
    let b = GGPlot::new(data())
        .aes(Aes::new().x("t").y("n").fill("genre"))
        .geom_stream()
        .build();
    let d = &b.layers[0].data;
    let f = |c: &str| -> Vec<f64> {
        d.column(c)
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    };
    let (x, lo, hi) = (f("x"), f("ymin"), f("y"));
    let mut xs: Vec<f64> = x.clone();
    xs.sort_by(|a, b| a.total_cmp(b));
    xs.dedup();
    for xv in xs {
        let mut iv: Vec<(f64, f64)> = (0..x.len())
            .filter(|&i| x[i] == xv)
            .map(|i| (lo[i], hi[i]))
            .collect();
        iv.sort_by(|a, b| a.0.total_cmp(&b.0));
        for w in iv.windows(2) {
            assert!(
                (w[0].1 - w[1].0).abs() < 1e-9,
                "gap/overlap at x={xv}: {iv:?}"
            );
        }
    }
}
