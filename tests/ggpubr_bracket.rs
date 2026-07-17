//! geom_bracket: significance brackets over a grouped boxplot (ggpubr).

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

fn grouped() -> Vec<(String, Vec<Value>)> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for (name, base) in [("ctrl", 5.0), ("trt1", 8.0), ("trt2", 6.5)] {
        for i in 0..8 {
            xs.push(Value::Str(name.to_string()));
            ys.push(Value::Float(base + (i % 4) as f64 - 1.5));
        }
    }
    vec![("grp".to_string(), xs), ("val".to_string(), ys)]
}

#[test]
fn single_bracket_renders_with_label() {
    let svg = GGPlot::new(grouped())
        .aes(Aes::new().x("grp").y("val"))
        .geom_boxplot()
        .geom_bracket("ctrl", "trt1", 11.0, "SIGA")
        .render_svg()
        .expect("render");
    assert!(svg.contains("SIGA"), "bracket label should render");
}

#[test]
fn multiple_brackets_render() {
    let svg = GGPlot::new(grouped())
        .aes(Aes::new().x("grp").y("val"))
        .geom_boxplot()
        .geom_bracket_many(
            GeomBracket::default(),
            &[
                ("ctrl", "trt1", 11.0, "SIGA"),
                ("ctrl", "trt2", 12.5, "SIGB"),
            ],
        )
        .render_svg()
        .expect("render");
    assert!(
        svg.contains("SIGA") && svg.contains("SIGB"),
        "both labels render"
    );
}

#[test]
fn bracket_pairs_with_compare_means() {
    // Common ggpubr composition: boxplot + overall p-value + pairwise bracket.
    let svg = GGPlot::new(grouped())
        .aes(Aes::new().x("grp").y("val").fill("grp"))
        .geom_boxplot()
        .geom_bracket("ctrl", "trt1", 11.0, "SIGA")
        .render_svg_native()
        .expect("render");
    assert!(svg.contains("SIGA"));
}
