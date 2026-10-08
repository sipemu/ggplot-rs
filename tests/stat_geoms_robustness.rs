//! 0.16 robustness rules for the 0.17 statistical layers: degenerate input
//! never panics, the SVG never contains NaN/inf attributes, and data-derived
//! text is escaped.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

type Frame = Vec<(String, Vec<Value>)>;

fn frames() -> Vec<(&'static str, Frame)> {
    let col = |name: &str, v: Vec<Value>| (name.to_string(), v);
    let f = |v: &[f64]| v.iter().map(|&x| Value::Float(x)).collect::<Vec<_>>();
    let s = |v: &[&str]| {
        v.iter()
            .map(|x| Value::Str(x.to_string()))
            .collect::<Vec<_>>()
    };
    let xss = "<script>alert('x')</script>";
    vec![
        ("empty", vec![]),
        (
            "one row",
            vec![
                col("x", f(&[1.0])),
                col("y", f(&[2.0])),
                col("lo", f(&[1.5])),
                col("hi", f(&[2.5])),
                col("g", s(&["a"])),
            ],
        ),
        (
            "all NaN",
            vec![
                col("x", f(&[f64::NAN; 3])),
                col("y", f(&[f64::NAN; 3])),
                col("lo", f(&[f64::NAN; 3])),
                col("hi", f(&[f64::NAN; 3])),
                col("g", s(&["a", "b", "a"])),
            ],
        ),
        (
            "constant",
            vec![
                col("x", f(&[2.0; 4])),
                col("y", f(&[2.0; 4])),
                col("lo", f(&[2.0; 4])),
                col("hi", f(&[2.0; 4])),
                col("g", s(&["a", "a", "b", "b"])),
            ],
        ),
        (
            "infinite + huge",
            vec![
                col("x", f(&[f64::NEG_INFINITY, 1e300, -1e300, 3.0])),
                col("y", f(&[1.0, f64::INFINITY, 1e-300, 4.0])),
                col("lo", f(&[0.0, 1.0, f64::NEG_INFINITY, 2.0])),
                col("hi", f(&[1.0, 2.0, 3.0, f64::INFINITY])),
                col("g", s(&[xss, "b", xss, "b"])),
            ],
        ),
    ]
}

fn plots(data: Frame) -> Vec<(&'static str, GGPlot)> {
    let base = || GGPlot::new(data.clone());
    vec![
        (
            "hline/vline/abline aes",
            base()
                .aes(Aes::new().x("x").y("y"))
                .geom_point()
                .geom_hline_aes(Aes::new().yintercept("lo").color("g"))
                .geom_vline_aes(Aes::new().xintercept("hi"))
                .geom_abline_aes(Aes::new().slope("lo").intercept("hi")),
        ),
        (
            "qq + band",
            base()
                .aes(Aes::new().y("y").color("g"))
                .stat_qq_band(StatQQBand::new(QQDistribution::t(2.0)).band(QQBandType::Ks))
                .stat_qq(QQDistribution::half_normal())
                .stat_qq_line(QQDistribution::exponential()),
        ),
        (
            "cooks",
            base()
                .aes(Aes::new().x("x").y("y"))
                .geom_point()
                .stat_cooks_contour(3, &[0.5, 1.0]),
        ),
        (
            "km",
            base()
                .aes(Aes::new().x("x").y("y").color("g"))
                .geom_stepribbon()
                .layer_aes(Aes::new().x("x").ymin("lo").ymax("hi"))
                .geom_step()
                .geom_censor_marks("lo"),
        ),
        (
            "ecdf band",
            base()
                .aes(Aes::new().x("x").fill("g"))
                .stat_ecdf_band(0.9)
                .stat_ecdf(),
        ),
        (
            "errorbarh",
            base()
                .aes(Aes::new().y("g").xmin("lo").xmax("hi"))
                .geom_errorbarh(),
        ),
        (
            "dodged forest",
            base()
                .aes(Aes::new().x("g").y("y").ymin("lo").ymax("hi").color("g"))
                .geom_pointrange()
                .position(position_dodge(0.5))
                .geom_errorbar()
                .position(PositionDodge2::default().with_reverse(true))
                .coord_flip(),
        ),
    ]
}

#[test]
fn new_layers_survive_degenerate_input() {
    for (dname, data) in frames() {
        for (pname, plot) in plots(data.clone()) {
            let res = plot.render_svg_native_with_warnings(320, 240);
            let (svg, _) = res.unwrap_or_else(|e| panic!("{pname} / {dname}: {e}"));
            assert!(!svg.contains("NaN"), "{pname} / {dname}: NaN in SVG");
            assert!(
                !svg.contains("=\"inf") && !svg.contains("=\"-inf") && !svg.contains(",inf"),
                "{pname} / {dname}: inf in SVG"
            );
            assert!(!svg.contains("<script>"), "{pname} / {dname}: unescaped");
            assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        }
    }
}
