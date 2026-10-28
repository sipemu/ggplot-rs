//! Candlestick variants (#320): hollow candles, HLC bars, Heikin-Ashi.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::candlestick::{CandleStyle, GeomCandlestick};
use ggplot_rs::stat::heikin_ashi::heikin_ashi;
use ggplot_rs::GGPlot;

fn ohlc() -> Vec<(String, Vec<Value>)> {
    let f = |v: &[f64]| v.iter().map(|&x| Value::Float(x)).collect();
    vec![
        ("t".into(), f(&[1.0, 2.0, 3.0, 4.0])),
        ("o".into(), f(&[10.0, 12.0, 11.0, 13.0])),
        ("h".into(), f(&[13.0, 12.5, 14.0, 13.5])),
        ("l".into(), f(&[9.0, 10.0, 10.5, 11.0])),
        ("c".into(), f(&[12.0, 10.5, 13.5, 12.0])),
    ]
}

fn aes() -> Aes {
    Aes::new().x("t").open("o").high("h").low("l").close("c")
}

#[test]
fn hollow_candles_hollow_rising_bodies_and_colour_by_previous_close() {
    let svg = GGPlot::new(ohlc())
        .aes(aes())
        .geom_candlestick_with(GeomCandlestick {
            style: CandleStyle::Hollow,
            ..Default::default()
        })
        .render_svg_native_with_size(500, 300)
        .unwrap();
    // Rising bodies (t=1, t=3) are hollow (white fill).
    let bodies: Vec<&str> = svg
        .match_indices("<rect")
        .map(|(i, _)| &svg[i..i + svg[i..].find('>').unwrap()])
        .filter(|t| t.contains("data-geom=\"candlestick\""))
        .collect();
    assert_eq!(bodies.len(), 4);
    assert_eq!(
        bodies
            .iter()
            .filter(|t| t.contains("fill=\"#FFFFFF\""))
            .count(),
        2
    );
    // t=4 closes at 12 < previous close 13.5 → down colour although its
    // own body (13 → 12) falls too; t=2 likewise.
    assert!(bodies[3].contains("#E03131"));
}

#[test]
fn hlc_bars_need_no_open() {
    let svg = GGPlot::new(ohlc())
        .aes(Aes::new().x("t").high("h").low("l").close("c"))
        .geom_hlc()
        .render_svg_native_with_size(500, 300)
        .unwrap();
    assert!(svg.contains("<title>1 — H 13 L 9 C 12</title>"));
    // Two strokes per bar: high–low and the close tick.
    assert_eq!(svg.matches("data-geom=\"hlc\"").count(), 4 * 2);
}

#[test]
fn heikin_ashi_candles_use_the_transform() {
    let built = GGPlot::new(ohlc()).aes(aes()).geom_heikin_ashi().build();
    let d = &built.layers[0].data;
    let col = |c: &str| -> Vec<f64> {
        d.column(c)
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    };
    let want = heikin_ashi(&[
        (10.0, 13.0, 9.0, 12.0),
        (12.0, 12.5, 10.0, 10.5),
        (11.0, 14.0, 10.5, 13.5),
        (13.0, 13.5, 11.0, 12.0),
    ]);
    assert_eq!(col("open"), want.iter().map(|w| w.0).collect::<Vec<_>>());
    assert_eq!(col("close"), want.iter().map(|w| w.3).collect::<Vec<_>>());
    let svg = GGPlot::new(ohlc())
        .aes(aes())
        .geom_heikin_ashi()
        .render_svg_native_with_size(500, 300)
        .unwrap();
    assert!(svg.contains("<title>1 — O 11 H 13 L 9 C 11</title>"));
}
