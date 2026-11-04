//! #428: precomputed (`stat = "identity"`) boxplots dodge in the fill
//! scale's level order — same as the legend and the raw-row path — and a
//! mapped `tooltip` doesn't take part in grouping.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::stat::identity::StatIdentity;

fn frame(reverse: bool) -> Vec<(String, Vec<Value>)> {
    let mut rows = Vec::new();
    for x in ["a", "b"] {
        for (k, d) in ["4", "f", "r"].iter().enumerate() {
            rows.push((x, *d, k as f64));
        }
    }
    if reverse {
        rows.reverse();
    }
    let s = |f: &dyn Fn(&(&str, &str, f64)) -> Value| rows.iter().map(f).collect::<Vec<_>>();
    vec![
        ("x".into(), s(&|r| Value::Str(r.0.into()))),
        ("drv".into(), s(&|r| Value::Str(r.1.into()))),
        ("lower".into(), s(&|r| Value::Float(r.2 + 1.0))),
        ("middle".into(), s(&|r| Value::Float(r.2 + 2.0))),
        ("upper".into(), s(&|r| Value::Float(r.2 + 3.0))),
        ("ymin".into(), s(&|r| Value::Float(r.2))),
        ("ymax".into(), s(&|r| Value::Float(r.2 + 4.0))),
        (
            "btip".into(),
            s(&|r| Value::Str(format!("{} / {} / {}", r.0, r.1, r.2 * 7.0 % 5.0))),
        ),
    ]
}

/// (x, drv, dodge offset) of each built row, sorted by x then offset.
fn dodged(reverse: bool, tooltip: bool) -> Vec<(String, String)> {
    let mut a = Aes::new()
        .x("x")
        .lower("lower")
        .middle("y")
        .upper("upper")
        .ymin("ymin")
        .ymax("ymax");
    if tooltip {
        a = a.tooltip("btip");
    }
    let built = GGPlot::new(frame(reverse))
        .aes(Aes::new().x("x").y("middle").fill("drv"))
        .geom_boxplot()
        .stat(StatIdentity)
        .layer_aes(a)
        // As anofox-visualization's category scale: explicit sorted levels.
        .scale_fill(
            ScaleColorDiscrete::new(Aesthetic::Fill)
                .sorted()
                .with_levels(vec!["4".into(), "f".into(), "r".into()]),
        )
        .build();
    let d = &built.layers[0].data;
    let off = d.column(".x_dodge_offset").expect("dodge offset column");
    let mut rows: Vec<(String, f64, String)> = (0..d.nrows())
        .map(|i| {
            (
                d.column("x").unwrap()[i].to_group_key(),
                off[i].as_f64().unwrap(),
                d.column("drv").unwrap()[i].to_group_key(),
            )
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    rows.into_iter().map(|r| (r.0, r.2)).collect()
}

#[test]
fn identity_boxplots_dodge_in_legend_order() {
    let legend = ["4", "f", "r"];
    let base: Vec<(String, String)> = ["a", "b"]
        .iter()
        .flat_map(|x| legend.iter().map(|d| (x.to_string(), d.to_string())))
        .collect();
    for (rev, tip) in [(false, false), (false, true), (true, false), (true, true)] {
        assert_eq!(dodged(rev, tip), base, "reverse={rev} tooltip={tip}");
    }
}
