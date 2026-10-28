//! `geom_funnel` / `geom_pyramid` (#303).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::funnel::GeomFunnel;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "stage".into(),
            ["Visits", "Sign-ups", "Trials", "Paid"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
        (
            "n".into(),
            [5000.0, 1800.0, 900.0, 250.0]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        ),
    ]
}

#[test]
fn funnel_renders_stages_with_conversion_labels() {
    let (svg, warnings) = GGPlot::new(data())
        .aes(Aes::new().x("stage").y("n"))
        .geom_funnel()
        .render_svg_native_with_warnings(600, 400)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(svg.matches("<polygon").count(), 4);
    assert!(svg.contains(">Sign-ups: 1800 (36%)<"), "{svg}");
    assert!(svg.contains(">Paid: 250 (5%)<"));
    assert!(!svg.contains(">0.0<") && !svg.contains("NaN"));
    // Stages filled by name: four distinct fills.
    let fills: std::collections::BTreeSet<&str> = svg
        .match_indices("<polygon")
        .map(|(i, _)| {
            let tag = &svg[i..i + svg[i..].find('>').unwrap()];
            let f = tag.find("fill=\"#").unwrap();
            &tag[f + 6..f + 13]
        })
        .collect();
    assert_eq!(fills.len(), 4, "{fills:?}");
}

#[test]
fn silhouette_and_pyramid_show_shares_of_total() {
    for geom in [GeomFunnel::silhouette(0.3, 0.25), GeomFunnel::pyramid()] {
        let svg = GGPlot::new(data())
            .aes(Aes::new().x("stage").y("n"))
            .geom_funnel_with(geom)
            .render_svg_native_with_size(500, 500)
            .unwrap();
        assert!(svg.contains(">Visits: 5000 (62.9%)<"), "{svg}");
        assert_eq!(svg.matches("<polygon").count(), 4);
    }
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("stage").y("n"))
        .geom_pyramid()
        .render_svg_native_with_size(500, 500)
        .unwrap();
    assert!(svg.contains("(62.9%)"));
}
