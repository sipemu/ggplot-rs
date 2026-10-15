//! Locale-aware axis / legend labels (`GGPlot::locale`).

use ggplot_rs::prelude::*;

fn numeric() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".to_string(),
            vec![
                Value::Float(0.0),
                Value::Float(1500.0),
                Value::Float(3000.0),
            ],
        ),
        (
            "y".to_string(),
            vec![Value::Float(0.0), Value::Float(0.7), Value::Float(1.4)],
        ),
        (
            "z".to_string(),
            vec![
                Value::Float(1000.0),
                Value::Float(4000.0),
                Value::Float(9000.0),
            ],
        ),
    ]
}

/// Daily points 2026-10-01 .. 2027-01-31 (epoch seconds as `DateTime`).
fn daily() -> Vec<(String, Vec<Value>)> {
    let start = 1_790_812_800; // 2026-10-01 00:00 UTC
    let n = 123;
    let t = (0..n)
        .map(|d| Value::DateTime(start + d * 86_400))
        .collect();
    let y = (0..n)
        .map(|d| Value::Float((d as f64 / 9.0).sin()))
        .collect();
    vec![("t".to_string(), t), ("y".to_string(), y)]
}

fn tick_labels(built: &ggplot_rs::build::BuiltPlot, aes: Aesthetic) -> Vec<String> {
    built
        .scales
        .get(&aes)
        .expect("scale")
        .breaks()
        .into_iter()
        .map(|(_, l)| l)
        .collect()
}

fn plot(locale: Option<Locale>) -> GGPlot {
    let p = GGPlot::new(numeric())
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point();
    match locale {
        Some(l) => p.locale(l),
        None => p,
    }
}

#[test]
fn de_de_plot_uses_german_marks_on_axes_and_legend() {
    let built = plot(Some(Locale::from_tag("de-DE"))).build();
    let x = tick_labels(&built, Aesthetic::X);
    assert!(x.contains(&"1.000".to_string()), "{x:?}");
    assert!(x.contains(&"3.000".to_string()), "{x:?}");
    let y = tick_labels(&built, Aesthetic::Y);
    assert!(y.iter().any(|l| l.contains(',')), "{y:?}");
    assert!(y.iter().all(|l| !l.contains('.')), "{y:?}");
    let legend = tick_labels(&built, Aesthetic::Color);
    assert!(legend.iter().any(|l| l.contains('.')), "{legend:?}");

    let svg = plot(Some(Locale::de_de())).render_svg_native().unwrap();
    assert!(svg.contains(">1.000<"), "rendered 1.000 tick");
    assert!(
        svg.contains(">1,5<") || svg.contains(">0,5<"),
        "rendered decimal comma"
    );
}

#[test]
fn fr_and_en_locales() {
    let fr = tick_labels(&plot(Some(Locale::fr_fr())).build(), Aesthetic::X);
    assert!(fr.contains(&"1 000".to_string()), "{fr:?}");
    let us = tick_labels(&plot(Some(Locale::en_us())).build(), Aesthetic::X);
    assert!(us.contains(&"1,000".to_string()), "{us:?}");
}

#[test]
fn default_output_is_unchanged_without_locale() {
    let built = plot(None).build();
    let x = tick_labels(&built, Aesthetic::X);
    assert!(x.contains(&"1000".to_string()), "{x:?}");
    assert!(x.iter().all(|l| !l.contains(',')), "{x:?}");
    let y = tick_labels(&built, Aesthetic::Y);
    assert!(y.iter().any(|l| l.contains('.')), "{y:?}");
    let t = GGPlot::new(daily())
        .aes(Aes::new().x("t").y("y"))
        .geom_line()
        .build();
    let t = tick_labels(&t, Aesthetic::X);
    assert!(t.iter().all(|l| l.starts_with("20")), "ISO labels: {t:?}");
    let svg = plot(None).render_svg_native().unwrap();
    assert!(svg.contains(">1000<") && svg.contains(">0.5<"));
    assert!(!svg.contains(">1.000<") && !svg.contains(">0,5<"));
}

#[test]
fn de_de_date_axis_uses_german_month_names() {
    let p = || {
        GGPlot::new(daily())
            .aes(Aes::new().x("t").y("y"))
            .geom_line()
            .locale(Locale::de_de())
    };
    let labels = tick_labels(&p().build(), Aesthetic::X);
    assert!(labels.contains(&"Okt. 2026".to_string()), "{labels:?}");
    assert!(labels.contains(&"Dez. 2026".to_string()), "{labels:?}");
    let svg = p().render_svg_native().unwrap();
    assert!(svg.contains("Okt. 2026"));

    // Explicit strftime labels get German names too.
    let labels = tick_labels(
        &GGPlot::new(daily())
            .aes(Aes::new().x("t").y("y"))
            .geom_line()
            .scale_x_datetime(
                ScaleDateTime::new()
                    .with_date_breaks("1 month")
                    .with_date_labels("%-d. %B"),
            )
            .locale(Locale::de_de())
            .build(),
        Aesthetic::X,
    );
    assert!(labels.contains(&"1. Oktober".to_string()), "{labels:?}");
    assert!(labels.contains(&"1. Januar".to_string()), "{labels:?}");
}

#[test]
fn scale_locale_and_custom_formatter_take_precedence() {
    let built = GGPlot::new(numeric())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_continuous(ScaleContinuous::new().with_locale(Locale::fr_fr()))
        .scale_y_continuous(
            ScaleContinuous::new()
                .with_label_formatter(label_percent_locale(&Locale::en_us(), None)),
        )
        .locale(Locale::de_de())
        .build();
    let x = tick_labels(&built, Aesthetic::X);
    assert!(x.contains(&"1 000".to_string()), "{x:?}");
    let y = tick_labels(&built, Aesthetic::Y);
    assert!(
        y.iter().all(|l| l.ends_with('%') && !l.ends_with(" %")),
        "{y:?}"
    );
}
