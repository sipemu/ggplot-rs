//! `geom_lollipop` / `geom_dumbbell` (#306).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
    let f = |v: &[f64]| v.iter().map(|&x| Value::Float(x)).collect();
    vec![
        ("k".into(), s(&["A", "B", "C"])),
        ("v".into(), f(&[3.0, 7.0, 5.0])),
        ("lo".into(), f(&[1.0, 4.0, 2.0])),
        ("hi".into(), f(&[6.0, 9.0, 3.0])),
    ]
}

#[test]
fn lollipop_draws_a_stem_and_dot_per_row_from_zero() {
    let built = GGPlot::new(data())
        .aes(Aes::new().x("k").y("v"))
        .geom_lollipop()
        .build();
    let y = built.scales.get(&ggplot_rs::aes::Aesthetic::Y).unwrap();
    assert_eq!(y.domain().unwrap().0, 0.0, "baseline included");
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("k").y("v"))
        .geom_lollipop()
        .coord_flip()
        .render_svg_native_with_size(500, 300)
        .unwrap();
    assert!(svg.contains("<title>B: 7</title>"));
    assert_eq!(
        svg.matches("data-geom=\"lollipop\"").count(),
        6,
        "3 stems + 3 dots"
    );
}

#[test]
fn dumbbell_draws_both_ends_and_trains_on_the_range() {
    let built = GGPlot::new(data())
        .aes(Aes::new().x("k").ymin("lo").ymax("hi"))
        .geom_dumbbell()
        .build();
    let (lo, hi) = built
        .scales
        .get(&ggplot_rs::aes::Aesthetic::Y)
        .unwrap()
        .domain()
        .unwrap();
    assert_eq!((lo, hi), (1.0, 9.0));
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("k").ymin("lo").ymax("hi"))
        .geom_dumbbell()
        .render_svg_native_with_size(500, 300)
        .unwrap();
    assert!(svg.contains("<title>A: 1 → 6</title>"));
    assert!(svg.contains("#A0A0A0") && svg.contains("#1F77B4"));
    assert_eq!(svg.matches("data-geom=\"dumbbell\"").count(), 9);
}
