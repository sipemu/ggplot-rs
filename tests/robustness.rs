//! Degenerate / hostile data must neither panic, hang, nor inject markup.
//! Hosts (e.g. a DuckDB extension) render user data straight into HTML and may
//! run with `panic = "abort"`, so these are hard guarantees, not niceties.
use std::time::{Duration, Instant};

use ggplot_rs::prelude::*;

fn col(name: &str, v: Vec<Value>) -> (String, Vec<Value>) {
    (name.to_string(), v)
}

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}

fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

#[test]
fn hover_attributes_escape_quotes() {
    let evil = "a\" onmouseover=\"alert(1)";
    let svg = GGPlot::new(vec![
        col("x", strs(&[evil, "b"])),
        col("y", floats(&[1.0, 2.0])),
        col("lab", strs(&["x'y", "<b>"])),
    ])
    .aes(Aes::new().x("x").y("y").label("lab"))
    .geom_col()
    .render_svg_native_with_size(300, 200)
    .expect("render");
    assert!(
        !svg.contains("onmouseover=\""),
        "attribute injection: {svg}"
    );
    assert!(svg.contains("&quot;"), "quote escaped");
    assert!(!svg.contains("<b>"), "markup escaped");
}

#[test]
fn control_characters_are_dropped() {
    let svg = GGPlot::new(vec![
        col("x", strs(&["a\u{1}b", "c"])),
        col("y", floats(&[1.0, 2.0])),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_col()
    .title("t\u{7}itle")
    .render_svg_native_with_size(300, 200)
    .expect("render");
    assert!(!svg
        .chars()
        .any(|c| (c as u32) < 0x20 && !"\t\n\r".contains(c)));
}

/// 1-ulp differences (float SUM/AVG noise) used to spin `extended_breaks`
/// forever once `max / step` exceeded 2^53.
#[test]
fn near_constant_data_terminates() {
    let start = Instant::now();
    for ys in [
        vec![1234.5, 1234.5, 1_234.500_000_000_000_2],
        vec![1e15, 1e15 + 0.125, 1e15],
        vec![-7.0, -7.0 + 1e-14, -7.0],
    ] {
        let x = floats(&[0.0, 1.0, 2.0]);
        GGPlot::new(vec![col("x", x.clone()), col("y", floats(&ys))])
            .aes(Aes::new().x("x").y("y"))
            .geom_line()
            .render_svg_native_with_size(300, 200)
            .expect("line render");
        // Same values through a continuous colour legend.
        GGPlot::new(vec![
            col("x", x.clone()),
            col("y", x),
            col("c", floats(&ys)),
        ])
        .aes(Aes::new().x("x").y("y").color("c"))
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .expect("colour render");
    }
    assert!(start.elapsed() < Duration::from_secs(10));
}

#[test]
fn extreme_datetimes_terminate() {
    let start = Instant::now();
    let x = vec![Value::DateTime(0), Value::DateTime(i64::MAX / 2)];
    let _ = GGPlot::new(vec![col("x", x), col("y", floats(&[1.0, 2.0]))])
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .render_svg_native_with_size(300, 200);
    assert!(start.elapsed() < Duration::from_secs(10));
}

/// `sort_by(partial_cmp…unwrap_or(Equal))` panics on NaN since Rust 1.81
/// ("user-provided comparison function does not correctly implement a total order").
#[test]
fn nan_x_does_not_panic() {
    let xs: Vec<f64> = (0..200)
        .map(|i| {
            if i % 7 == 0 {
                f64::NAN
            } else {
                (i * 37 % 101) as f64
            }
        })
        .collect();
    let ys: Vec<f64> = (0..200).map(|i| i as f64).collect();
    for layer in ["line", "area", "step"] {
        let p = GGPlot::new(vec![col("x", floats(&xs)), col("y", floats(&ys))])
            .aes(Aes::new().x("x").y("y"));
        let p = match layer {
            "line" => p.geom_line(),
            "area" => p.geom_area(),
            _ => p.geom_step(),
        };
        let _ = p.render_svg_native_with_size(300, 200);
    }
}

#[test]
fn many_categories_render_in_linear_time() {
    let n = 20_000;
    let x: Vec<Value> = (0..n).map(|i| Value::Str(format!("c{i}"))).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let fill: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("g{}", i % 500)))
        .collect();
    let start = Instant::now();
    GGPlot::new(vec![col("x", x), col("y", y), col("f", fill)])
        .aes(Aes::new().x("x").y("y").fill("f"))
        .geom_col()
        .theme_void()
        .render_svg_native_with_size(800, 400)
        .expect("render");
    // Was ~15 s (quadratic); now well under a second in release builds.
    assert!(
        start.elapsed() < Duration::from_secs(20),
        "{:?}",
        start.elapsed()
    );
}

#[test]
fn jitter_is_deterministic() {
    let render = || {
        GGPlot::new(vec![
            col("x", strs(&["a", "a", "b", "b", "c"])),
            col("y", floats(&[1.0, 2.0, 3.0, 4.0, 5.0])),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_jitter()
        .render_svg_native_with_size(300, 200)
        .expect("render")
    };
    assert_eq!(render(), render());
}

/// Bars on a continuous x are sized from the data spacing, so daily bars
/// neither overlap nor collapse into slivers.
#[test]
fn continuous_bars_do_not_overlap() {
    let n = 60;
    let svg = GGPlot::new(vec![
        col("x", floats(&(0..n).map(f64::from).collect::<Vec<_>>())),
        col(
            "y",
            floats(&(0..n).map(|i| 1.0 + f64::from(i % 5)).collect::<Vec<_>>()),
        ),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_col()
    .render_svg_native_with_size(800, 300)
    .expect("render");
    let mut spans: Vec<(f64, f64)> = svg
        .split("<rect ")
        .skip(1)
        .filter(|r| r.contains("data-x="))
        .filter_map(|r| {
            let r = format!(" {r}");
            let attr = |k: &str| -> Option<f64> {
                let s = r.split(&format!(" {k}=\"")).nth(1)?;
                s.split('"').next()?.parse().ok()
            };
            Some((attr("x")?, attr("width")?))
        })
        .collect();
    assert!(spans.len() >= n as usize, "found {} bars", spans.len());
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    for w in spans.windows(2) {
        assert!(w[0].0 + w[0].1 <= w[1].0 + 0.01, "overlap: {w:?}");
    }
    assert!(spans[0].1 > 4.0, "bars too thin: {}", spans[0].1);
}

/// Breaks on a narrow range get enough decimals to be distinguishable
/// (they used to all read "0.99"), and unchanged output where 2 decimals do.
#[test]
fn narrow_range_axis_labels_are_distinct() {
    use ggplot_rs::scale::util::format_numbers;
    assert_eq!(
        format_numbers(&[0.99, 0.992, 0.994, 0.996]),
        ["0.99", "0.992", "0.994", "0.996"]
    );
    assert_eq!(
        format_numbers(&[0.0, 2.5, 5.0, 7.5]),
        ["0", "2.5", "5", "7.5"]
    );
    assert_eq!(
        format_numbers(&[0.001, 0.01, 0.1, 1.0]),
        ["0.001", "0.01", "0.1", "1"]
    );
    assert_eq!(format_numbers(&[-0.0001, 0.0]), ["-0.0001", "0"]);

    let svg = GGPlot::new(vec![
        col("x", floats(&[1.0, 2.0, 3.0])),
        col("y", floats(&[0.9905, 0.9921, 0.9938])),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .render_svg_native_with_size(300, 200)
    .expect("render");
    let texts: Vec<&str> = svg
        .split("<text")
        .skip(1)
        .filter_map(|t| t.split('>').nth(1)?.split('<').next())
        .filter(|t| t.starts_with("0.99"))
        .collect();
    let mut unique = texts.clone();
    unique.sort();
    unique.dedup();
    assert!(texts.len() >= 2 && unique.len() == texts.len(), "{texts:?}");
}
