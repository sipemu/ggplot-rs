//! Edge cases of StatIndicator (#322).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::stat::indicator::{rsi, Indicator, StatIndicator};
use ggplot_rs::GGPlot;

fn rows(ind: Indicator, cols: Vec<(&str, Vec<f64>)>, aes: Aes) -> usize {
    let data: Vec<(String, Vec<Value>)> = cols
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.into_iter().map(Value::Float).collect()))
        .collect();
    GGPlot::new(data)
        .aes(aes)
        .geom_line()
        .stat(StatIndicator::new(ind))
        .build()
        .layers[0]
        .data
        .nrows()
}

#[test]
fn missing_inputs_flat_ranges_and_falling_zigzag() {
    let t: Vec<f64> = (0..30).map(f64::from).collect();
    let falling: Vec<f64> = (0..30)
        .map(|i| 100.0 - i as f64 * (1.0 + (i % 3) as f64))
        .collect();
    // ATR / stochastic without high/low produce nothing.
    let price = Aes::new().x("t").y("p");
    assert_eq!(
        rows(
            Indicator::Atr(5),
            vec![("t", t.clone()), ("p", falling.clone())],
            price.clone()
        ),
        0
    );
    assert_eq!(
        rows(
            Indicator::Stochastic(5, 1),
            vec![("t", t.clone()), ("p", falling.clone())],
            price.clone()
        ),
        0
    );
    // Flat high = low: stochastic is 50.
    let flat = vec![5.0; 30];
    let hlc = Aes::new().x("t").close("c").high("h").low("l");
    assert_eq!(
        rows(
            Indicator::Stochastic(5, 1),
            vec![
                ("t", t.clone()),
                ("c", flat.clone()),
                ("h", flat.clone()),
                ("l", flat)
            ],
            hlc
        ),
        26
    );
    // A falling series starts the zigzag downward.
    assert!(
        rows(
            Indicator::ZigZag(2.0),
            vec![("t", t.clone()), ("p", falling)],
            price.clone()
        ) >= 2
    );
    // VWAP without volume falls back to equal weights.
    assert_eq!(
        rows(Indicator::Vwap, vec![("t", t), ("p", vec![1.0; 30])], price),
        30
    );
    // Too-short RSI input.
    assert!(rsi(&[1.0, 2.0], 14).iter().all(Option::is_none));
}
