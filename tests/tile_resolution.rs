//! Tiles are sized by the data resolution and train continuous scales on
//! their extents, so heatmaps need no manual expansion.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}
fn root_attr<'a>(svg: &'a str, name: &str) -> &'a str {
    let head = &svg[..svg.find('>').unwrap()];
    let key = format!(" {name}=\"");
    let i = head.find(&key).unwrap() + key.len();
    &head[i..i + head[i..].find('"').unwrap()]
}
fn nums(s: &str) -> Vec<f64> {
    s.split(' ').map(|t| t.parse().unwrap()).collect()
}
/// `(x, y, w, h)` of every `<rect>` carrying a `data-value` (the tiles).
fn tiles(svg: &str) -> Vec<(f64, f64, f64, f64)> {
    svg.split("<rect ")
        .skip(1)
        .filter(|t| t.contains("data-value"))
        .map(|tag| {
            let get = |k: &str| -> f64 {
                let key = format!("{k}=\"");
                let i = tag.find(&key).unwrap() + key.len();
                tag[i..i + tag[i..].find('"').unwrap()].parse().unwrap()
            };
            (get("x"), get("y"), get("width"), get("height"))
        })
        .collect()
}

fn grid() -> Vec<(String, Vec<Value>)> {
    // x on a 10-unit grid, y on a 0.5-unit grid.
    let mut xs = vec![];
    let mut ys = vec![];
    let mut fs = vec![];
    for (i, x) in [0.0, 10.0, 20.0].iter().enumerate() {
        for (j, y) in [1.0, 1.5].iter().enumerate() {
            xs.push(*x);
            ys.push(*y);
            fs.push((i * 2 + j) as f64);
        }
    }
    frame(vec![
        ("x", floats(&xs)),
        ("y", floats(&ys)),
        ("v", floats(&fs)),
    ])
}

#[test]
fn tiles_abut_and_stay_inside_the_panel() {
    let svg = GGPlot::new(grid())
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_tile()
        .render_svg_native_with_size(500, 300)
        .unwrap();
    // Scales train on the extents [-5, 25] × [0.75, 1.75], then expand 5%.
    let xd = nums(root_attr(&svg, "data-xdomain"));
    assert!(
        (xd[0] + 6.5).abs() < 1e-9 && (xd[1] - 26.5).abs() < 1e-9,
        "{xd:?}"
    );
    let yd = nums(root_attr(&svg, "data-ydomain"));
    assert!(
        (yd[0] - 0.7).abs() < 1e-9 && (yd[1] - 1.8).abs() < 1e-9,
        "{yd:?}"
    );

    let p = nums(root_attr(&svg, "data-plot"));
    let t = tiles(&svg);
    assert_eq!(t.len(), 6);
    let w0 = t[0].2;
    for &(x, y, w, h) in &t {
        assert!((w - w0).abs() < 0.02, "equal widths");
        assert!(x >= p[0] - 0.01 && x + w <= p[0] + p[2] + 0.01, "inside x");
        assert!(y >= p[1] - 0.01 && y + h <= p[1] + p[3] + 0.01, "inside y");
    }
    // Adjacent columns touch: width = one grid step in pixels.
    let step_px = 10.0 / (xd[1] - xd[0]) * p[2];
    assert!((w0 - step_px).abs() < 0.05, "{w0} vs {step_px}");
}

#[test]
fn single_tile_uses_unit_resolution() {
    let data = frame(vec![("x", floats(&[3.0])), ("y", floats(&[7.0]))]);
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_tile()
        .try_build()
        .unwrap();
    let d = &built.layers[0].data;
    assert_eq!(d.column("xmin").unwrap()[0].as_f64(), Some(2.5));
    assert_eq!(d.column("ymax").unwrap()[0].as_f64(), Some(7.5));
}

#[test]
fn discrete_tiles_are_unchanged() {
    let data = frame(vec![
        ("x", vec![Value::Str("a".into()), Value::Str("b".into())]),
        ("y", vec![Value::Str("u".into()), Value::Str("v".into())]),
        ("v", floats(&[1.0, 2.0])),
    ]);
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_tile()
        .try_build()
        .unwrap();
    assert!(!built.layers[0].data.has_column("xmin"));
}
