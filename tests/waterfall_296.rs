//! `geom_waterfall` (#296): floating steps from the running total, subtotal
//! and closing total bars, direction fill legend and connectors.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::waterfall::GeomWaterfall;
use ggplot_rs::scale::color::RGBAColor;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    let steps = ["Revenue", "COGS", "Gross", "Opex", "Tax"];
    let vals = [100.0, -40.0, 0.0, -25.0, -10.0];
    vec![
        (
            "step".into(),
            steps.iter().map(|s| Value::Str(s.to_string())).collect(),
        ),
        ("v".into(), vals.iter().map(|&f| Value::Float(f)).collect()),
    ]
}

fn waterfall() -> GeomWaterfall {
    GeomWaterfall::default()
        .with_totals(["Gross"])
        .with_final_total("Net")
}

fn floats(df: &ggplot_rs::data::DataFrame, col: &str) -> Vec<f64> {
    df.column(col)
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

#[test]
fn build_computes_floating_extents_and_totals() {
    let built = GGPlot::new(data())
        .aes(Aes::new().x("step").y("v"))
        .geom_waterfall_with(waterfall())
        .build();
    let d = &built.layers[0].data;
    assert_eq!(d.nrows(), 6);
    assert_eq!(floats(d, "ymin"), vec![0.0, 60.0, 0.0, 35.0, 25.0, 0.0]);
    assert_eq!(
        floats(d, "ymax"),
        vec![100.0, 100.0, 60.0, 60.0, 35.0, 25.0]
    );
    let fill: Vec<String> = d
        .column("fill")
        .unwrap()
        .iter()
        .map(|v| v.key_str().into_owned())
        .collect();
    assert_eq!(
        fill,
        ["Increase", "Decrease", "Total", "Decrease", "Decrease", "Total"]
    );
    // The appended total is a new x level, last on the axis.
    let x = built.scales.get(&ggplot_rs::aes::Aesthetic::X).unwrap();
    assert_eq!(x.breaks().len(), 6);
}

#[test]
fn renders_direction_colours_legend_tooltips_and_connectors() {
    let (svg, warnings) = GGPlot::new(data())
        .aes(Aes::new().x("step").y("v"))
        .geom_waterfall_with(waterfall())
        .render_svg_native_with_warnings(600, 400)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("#0CA678") && svg.contains("#E03131") && svg.contains("#4C6EA8"));
    for key in ["Increase", "Decrease", "Total"] {
        assert!(svg.contains(&format!(">{key}<")), "legend key {key}");
    }
    assert!(svg.contains("<title>COGS: -40 (total 60)</title>"), "{svg}");
    assert!(svg.contains("<title>Net: 25</title>"));
    assert!(svg.contains("<title>Revenue: +100 (total 100)</title>"));
    // 5 dashed connectors between 6 bars.
    assert_eq!(svg.matches("stroke-dasharray").count(), 5);
}

#[test]
fn no_totals_means_no_total_key_and_user_scale_wins() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("step").y("v"))
        .geom_waterfall_with(GeomWaterfall {
            connectors: false,
            ..Default::default()
        })
        .render_svg_native_with_size(600, 400)
        .unwrap();
    assert!(!svg.contains(">Total<"));
    assert!(!svg.contains("stroke-dasharray"));

    let svg = GGPlot::new(data())
        .aes(Aes::new().x("step").y("v"))
        .geom_waterfall()
        .scale_fill_manual(vec![
            ("Increase", RGBAColor::new(1, 2, 3)),
            ("Decrease", RGBAColor::new(4, 5, 6)),
        ])
        .render_svg_native_with_size(600, 400)
        .unwrap();
    assert!(svg.contains("#010203") && svg.contains("#040506"));
}

#[test]
fn coord_flip_draws_horizontal_waterfall() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("step").y("v"))
        .geom_waterfall_with(waterfall())
        .coord_flip()
        .render_svg_native_with_size(600, 400)
        .unwrap();
    assert!(svg.contains("<title>Net: 25</title>"));
    assert!(!svg.contains("NaN"));
}
