//! Technical-indicator stats (#322).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::stat::indicator::{Indicator, MacdPart, StatIndicator};
use ggplot_rs::GGPlot;

fn series() -> Vec<(String, Vec<Value>)> {
    let n = 60;
    let close: Vec<f64> = (0..n)
        .map(|i| 100.0 + 5.0 * (i as f64 * 0.3).sin() + i as f64 * 0.2)
        .collect();
    let f = |v: Vec<f64>| v.into_iter().map(Value::Float).collect();
    vec![
        ("t".into(), f((0..n).map(f64::from).collect())),
        ("close".into(), f(close.clone())),
        ("high".into(), f(close.iter().map(|c| c + 1.0).collect())),
        ("low".into(), f(close.iter().map(|c| c - 1.0).collect())),
        ("vol".into(), f((0..n).map(|i| 100.0 + i as f64).collect())),
    ]
}

fn ys(ind: Indicator, aes: Aes) -> Vec<f64> {
    let b = GGPlot::new(series())
        .aes(aes)
        .geom_line()
        .stat(StatIndicator::new(ind))
        .build();
    b.layers[0]
        .data
        .column("y")
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

#[test]
fn every_indicator_produces_values_after_warm_up() {
    let price = || Aes::new().x("t").y("close");
    let hlc = || Aes::new().x("t").close("close").high("high").low("low");
    assert_eq!(ys(Indicator::Sma(10), price()).len(), 51);
    assert_eq!(ys(Indicator::Ema(10), price()).len(), 51);
    assert_eq!(ys(Indicator::Wma(10), price()).len(), 51);
    assert_eq!(ys(Indicator::Rsi(14), price()).len(), 46);
    for part in [MacdPart::Line, MacdPart::Signal, MacdPart::Histogram] {
        assert_eq!(
            ys(Indicator::Macd(12, 26, 9, part), price()).len(),
            60 - 25 - 8
        );
    }
    let atr = ys(Indicator::Atr(14), hlc());
    assert!(
        atr.iter().all(|a| *a >= 2.0 - 1e-9),
        "true range ≥ high − low = 2"
    );
    let st = ys(Indicator::Stochastic(14, 3), hlc());
    assert!(st.iter().all(|s| (0.0..=100.0).contains(s)));
    let vwap = ys(Indicator::Vwap, Aes::new().x("t").y("close").weight("vol"));
    assert_eq!(vwap.len(), 60);
    let zz = ys(Indicator::ZigZag(3.0), price());
    assert!(zz.len() >= 3 && zz.len() < 20, "only pivots: {}", zz.len());
}

#[test]
fn bollinger_bands_render_as_a_ribbon() {
    let b = GGPlot::new(series())
        .aes(Aes::new().x("t").y("close"))
        .geom_ribbon()
        .stat(StatIndicator::new(Indicator::Bollinger(20, 2.0)))
        .build();
    let d = &b.layers[0].data;
    let (lo, hi) = (d.column("ymin").unwrap(), d.column("ymax").unwrap());
    assert!(lo
        .iter()
        .zip(hi)
        .all(|(a, b)| a.as_f64().unwrap() < b.as_f64().unwrap()));
    let svg = GGPlot::new(series())
        .aes(Aes::new().x("t").y("close"))
        .geom_ribbon()
        .stat(StatIndicator::new(Indicator::Bollinger(20, 2.0)))
        .geom_line()
        .stat(StatIndicator::new(Indicator::Sma(20)))
        .render_svg_native_with_size(500, 300)
        .unwrap();
    assert!(!svg.contains("NaN"));
}
