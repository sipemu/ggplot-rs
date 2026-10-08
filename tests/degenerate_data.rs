//! Degenerate / hostile data must degrade gracefully: empty plots render an
//! empty panel, layers whose stat cannot estimate anything draw nothing (other
//! layers still render), non-finite positions are dropped with a warning, and
//! no `NaN`/`inf` ever reaches an SVG attribute.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::smooth::GeomSmooth;
use ggplot_rs::stat::smooth::SmoothMethod;
use ggplot_rs::GGPlot;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}

fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// Every attribute value in the document is free of non-finite numbers.
fn assert_finite_attrs(svg: &str) {
    for (i, _) in svg.match_indices("=\"") {
        let rest = &svg[i + 2..];
        let val = &rest[..rest.find('"').unwrap()];
        for tok in val.split([' ', ',', '(', ')']) {
            let t = tok.trim_start_matches('-');
            assert!(
                t != "NaN" && t != "inf" && t != "infinity",
                "non-finite attribute value {val:?} in svg"
            );
        }
    }
}

fn count(svg: &str, needle: &str) -> usize {
    svg.matches(needle).count()
}

#[test]
fn empty_frame_renders_empty_panel_with_title() {
    let data = frame(vec![("x", vec![]), ("y", vec![])]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_col()
        .title("Nothing here")
        .render_svg_native_with_size(400, 300)
        .expect("an empty plot renders");
    assert!(svg.contains("Nothing here"));
    assert!(svg.starts_with("<svg"));
    assert_finite_attrs(&svg);
}

#[test]
fn plot_without_columns_or_layers_renders() {
    let svg = GGPlot::new(ggplot_rs::data::DataFrame::new())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("T")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains(">T</text>"));
    assert_finite_attrs(&svg);

    let svg = GGPlot::new(ggplot_rs::data::DataFrame::new())
        .title("no layers")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("no layers"));
}

#[test]
fn missing_aesthetic_on_real_data_still_errors() {
    let data = frame(vec![("a", floats(&[1.0, 2.0]))]);
    let r = GGPlot::new(data)
        .aes(Aes::new().x("a"))
        .geom_point()
        .render_svg_native_with_size(300, 200);
    assert!(
        r.is_err(),
        "geom_point without y must still be a validation error"
    );
}

#[test]
fn single_value_density_draws_nothing_but_plot_renders() {
    let data = frame(vec![("x", floats(&[3.0])), ("y", floats(&[1.0]))]);
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_density()
        .render_svg_native_with_warnings(400, 300)
        .expect("a degenerate density layer must not fail the plot");
    assert!(count(&svg, "<circle") >= 1, "the point layer still renders");
    assert!(
        warnings.iter().any(|w| w.contains("stat_density")),
        "warning about the empty stat: {warnings:?}"
    );
    assert_finite_attrs(&svg);
}

#[test]
fn single_value_violin_and_qq_do_not_fail() {
    let data = frame(vec![
        ("g", vec![Value::Str("a".into())]),
        ("y", floats(&[2.0])),
    ]);
    let svg = GGPlot::new(data.clone())
        .aes(Aes::new().x("g").y("y"))
        .geom_violin()
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_finite_attrs(&svg);

    let svg = GGPlot::new(frame(vec![("y", floats(&[2.0]))]))
        .aes(Aes::new().y("y"))
        .geom_qq()
        .geom_qq_line()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_finite_attrs(&svg);
}

#[test]
fn loess_on_two_points_skips_only_the_smoother() {
    let data = frame(vec![("x", floats(&[1.0, 2.0])), ("y", floats(&[3.0, 5.0]))]);
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth {
            method: SmoothMethod::Loess { span: 0.75 },
            ..Default::default()
        })
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert!(count(&svg, "<circle") >= 2);
    assert!(warnings.iter().any(|w| w.contains("produced no data")));
    assert_finite_attrs(&svg);
}

#[test]
fn non_finite_rows_are_dropped_with_warning() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0, 4.0, f64::NAN])),
        ("y", floats(&[1.0, f64::NAN, f64::INFINITY, 4.0, 5.0])),
    ]);
    let built = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .try_build()
        .unwrap();
    assert_eq!(built.layers[0].data.nrows(), 2);
    assert_eq!(built.warnings().len(), 1);
    assert!(built.warnings()[0].contains("removed 3 rows containing non-finite values"));

    for plot in [
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_line(),
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_area(),
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_col(),
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_smooth(),
        GGPlot::new(data.clone())
            .aes(Aes::new().x("y"))
            .geom_histogram(),
        GGPlot::new(data.clone())
            .aes(Aes::new().x("y"))
            .geom_density(),
    ] {
        let svg = plot.render_svg_native_with_size(300, 200).unwrap();
        assert_finite_attrs(&svg);
    }
}

#[test]
fn all_non_finite_renders_empty_panel() {
    let data = frame(vec![
        ("x", floats(&[f64::NAN, f64::INFINITY])),
        ("y", floats(&[f64::NEG_INFINITY, f64::NAN])),
    ]);
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("all bad")
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert!(svg.contains("all bad"));
    assert_eq!(count(&svg, "<circle"), 0);
    assert_eq!(warnings.len(), 1);
    assert_finite_attrs(&svg);
}

#[test]
fn zero_spread_density_uses_positive_bandwidth() {
    let data = frame(vec![("x", floats(&[5.0, 5.0, 5.0, 5.0]))]);
    let built = GGPlot::new(data.clone())
        .aes(Aes::new().x("x"))
        .geom_density()
        .try_build()
        .unwrap();
    let ys = built.layers[0].data.column("y").unwrap();
    assert!(!ys.is_empty());
    assert!(ys.iter().all(|v| v.as_f64().unwrap().is_finite()));
    assert!(ys.iter().any(|v| v.as_f64().unwrap() > 0.0));

    // A constant zero sample falls back to bw = 0.9 · 1 · n^-0.2 (R's bw.nrd0).
    let zeros = frame(vec![("x", floats(&[0.0, 0.0]))]);
    let built = GGPlot::new(zeros)
        .aes(Aes::new().x("x"))
        .geom_density()
        .try_build()
        .unwrap();
    let xs = built.layers[0].data.column("x").unwrap();
    let lo = xs.first().unwrap().as_f64().unwrap();
    let bw = 0.9 * 2f64.powf(-0.2);
    assert!((lo + 3.0 * bw).abs() < 1e-9, "x range is ±3 bw: {lo}");

    // Violins of constant groups render without NaN.
    let v = frame(vec![
        ("g", vec![Value::Str("a".into()); 3]),
        ("y", floats(&[2.0, 2.0, 2.0])),
    ]);
    let svg = GGPlot::new(v)
        .aes(Aes::new().x("g").y("y"))
        .geom_violin()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_finite_attrs(&svg);
}

#[test]
fn log_of_zero_is_dropped_not_drawn() {
    let data = frame(vec![
        ("x", floats(&[0.0, 1.0, 10.0])),
        ("y", floats(&[1.0, 2.0, 3.0])),
    ]);
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_log10()
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert_eq!(count(&svg, "<circle"), 2);
    assert_eq!(warnings.len(), 1);
    assert_finite_attrs(&svg);
}
