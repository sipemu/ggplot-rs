//! `SecAxis::linear` + `with_labeller`: a second measure's own right axis
//! (anofox-visualization #919). Its ticks sit at nice secondary values.
use ggplot_rs::aes::Aesthetic;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::sec_axis::SecAxis;

/// The text of every `<text>` element, trimmed.
fn texts(svg: &str) -> Vec<String> {
    svg.split("<text")
        .skip(1)
        .filter_map(|t| {
            let body = &t[t.find('>')? + 1..];
            Some(body[..body.find("</text>")?].trim().to_string())
        })
        .collect()
}

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".into(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".into(),
            vec![Value::Float(0.0), Value::Float(500.0), Value::Float(1000.0)],
        ),
    ]
}

#[test]
fn linear_axis_ticks_at_nice_secondary_values() {
    // primary 0..1000 drawn; secondary = primary / 1000 * 40 (a 0–40 % margin)
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_continuous(
            ScaleContinuous::new()
                .for_aesthetic(Aesthetic::Y)
                .with_sec_axis(
                    SecAxis::linear(0.04, 0.0)
                        .with_name("Margin")
                        .with_labeller(|v| format!("{v:.0}%")),
                ),
        )
        .render_svg()
        .unwrap();
    let t = texts(&svg);
    for want in ["0%", "10%", "20%", "30%", "40%", "Margin"] {
        assert!(t.iter().any(|x| x == want), "missing {want} in {t:?}");
    }
}

#[test]
fn linear_axis_with_offset_and_default_labels() {
    let sa = SecAxis::linear(2.0, 100.0);
    assert_eq!(sa.transform_value(5.0), 110.0);
    // a zero scale can't be inverted: identity
    assert_eq!(SecAxis::linear(0.0, 3.0).transform_value(7.0), 7.0);
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_continuous(
            ScaleContinuous::new()
                .for_aesthetic(Aesthetic::Y)
                .with_sec_axis(SecAxis::linear(2.0, 100.0)),
        )
        .render_svg()
        .unwrap();
    // secondary range ~100..2100: nice values, not 2 × the primary breaks + 100
    let t = texts(&svg);
    assert!(t.iter().any(|x| x == "1000" || x == "1,000"), "{t:?}");
    assert!(!t.iter().any(|x| x == "600"), "{t:?}");
}
