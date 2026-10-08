//! Regression diagnostics on the plotters-free native SVG path: a tagged 2×2
//! `PlotGrid` (residuals vs fitted, normal QQ, scale–location, residuals vs
//! leverage) with the most influential points labelled by `geom_text_repel`,
//! table-driven significance brackets, and — with the `regression` feature — a
//! logistic `geom_smooth(method = "glm")` with its link-scale band.
//!
//! ```sh
//! cargo run --no-default-features --features sf --example regression_diagnostics
//! cargo run --no-default-features --features sf,regression --example regression_diagnostics
//! ```
//!
//! Writes SVGs to `target/gallery-native/` (or the directory given as the
//! first argument).

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

fn f(v: impl IntoIterator<Item = f64>) -> Vec<Value> {
    v.into_iter().map(Value::Float).collect()
}

/// A deterministic simple-regression fit with two planted outliers, in the
/// `obs` (augment) contract shape: fitted, residual, std_residual, leverage.
fn augment() -> Vec<(String, Vec<Value>)> {
    let n = 60;
    let x: Vec<f64> = (0..n).map(|i| i as f64 / 6.0).collect();
    let mut y: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, x)| 2.0 + 0.8 * x + ((i * 37 % 17) as f64 - 8.0) / 6.0)
        .collect();
    y[7] += 6.0;
    y[52] -= 5.0;
    let xm = x.iter().sum::<f64>() / n as f64;
    let ym = y.iter().sum::<f64>() / n as f64;
    let sxx: f64 = x.iter().map(|v| (v - xm).powi(2)).sum();
    let b = x
        .iter()
        .zip(&y)
        .map(|(a, c)| (a - xm) * (c - ym))
        .sum::<f64>()
        / sxx;
    let a = ym - b * xm;
    let fitted: Vec<f64> = x.iter().map(|v| a + b * v).collect();
    let resid: Vec<f64> = y.iter().zip(&fitted).map(|(y, f)| y - f).collect();
    let sigma = (resid.iter().map(|r| r * r).sum::<f64>() / (n as f64 - 2.0)).sqrt();
    let lev: Vec<f64> = x
        .iter()
        .map(|v| 1.0 / n as f64 + (v - xm).powi(2) / sxx)
        .collect();
    let std: Vec<f64> = resid
        .iter()
        .zip(&lev)
        .map(|(r, h)| r / (sigma * (1.0 - h).sqrt()))
        .collect();
    let sqrt_abs: Vec<f64> = std.iter().map(|s| s.abs().sqrt()).collect();
    // Label the three largest |standardised residuals| (as plot.lm does).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| std[j].abs().total_cmp(&std[i].abs()));
    let top: Vec<usize> = order[..3].to_vec();
    let label: Vec<Value> = (0..n)
        .map(|i| {
            Value::Str(if top.contains(&i) {
                format!("{}", i + 1)
            } else {
                String::new()
            })
        })
        .collect();
    vec![
        ("row_id".into(), f((1..=n).map(|i| i as f64))),
        ("fitted".into(), f(fitted)),
        ("residual".into(), f(resid)),
        ("std_residual".into(), f(std)),
        ("sqrt_abs".into(), f(sqrt_abs)),
        ("leverage".into(), f(lev)),
        ("label".into(), label),
    ]
}

fn diagnostics() -> Result<String, GGError> {
    let p = |x: &str, y: &str| {
        GGPlot::new(augment())
            .aes(Aes::new().x(x).y(y).label("label"))
            .geom_point()
            .geom_text_repel_with(GeomTextRepel::default().nudge(0.0, 0.0))
            .theme_bw()
    };
    let resid_fitted = p("fitted", "residual")
        .geom_hline(0.0)
        .title("Residuals vs Fitted");
    let qq = GGPlot::new(augment())
        .aes(Aes::new().y("std_residual"))
        .geom_qq()
        .geom_qq_line()
        .theme_bw()
        .title("Normal Q-Q");
    let scale_loc = p("fitted", "sqrt_abs").title("Scale-Location");
    let leverage = p("leverage", "std_residual")
        .geom_hline(0.0)
        .title("Residuals vs Leverage");
    ((resid_fitted | qq) / (scale_loc | leverage))
        .title("lm(y ~ x): diagnostics")
        .caption("ggplot-rs PlotGrid · native SVG")
        .tag_levels(TagLevels::Lower)
        .spacing(6.0)
        .render_svg_native_with_size(1000, 820)
}

fn brackets() -> Result<String, GGError> {
    let mut g = Vec::new();
    let mut v = Vec::new();
    for (name, base) in [("ctrl", 5.0), ("trt1", 8.0), ("trt2", 6.5)] {
        for i in 0..12 {
            g.push(Value::Str(name.into()));
            v.push(Value::Float(base + ((i * 7) % 5) as f64 - 2.0));
        }
    }
    let s = |x: &str| Value::Str(x.into());
    // A `test`-contract table as produced by a post-hoc pairwise test.
    let tests = vec![
        ("method".to_string(), vec![s("t_test"); 3]),
        ("test_id".to_string(), vec![s("t1"), s("t2"), s("t3")]),
        ("group1".to_string(), vec![s("ctrl"), s("ctrl"), s("trt1")]),
        ("group2".to_string(), vec![s("trt1"), s("trt2"), s("trt2")]),
        ("p_value".to_string(), f([0.0002, 0.03, 0.04])),
        ("p_adj".to_string(), f([0.0006, 0.09, 0.08])),
    ];
    GGPlot::new(vec![("group".to_string(), g), ("value".to_string(), v)])
        .aes(Aes::new().x("group").y("value").fill("group"))
        .geom_boxplot()
        .geom_bracket_table(tests, BracketTable::new().label("p = {p_adj} {p.signif}"))
        .theme_pubr()
        .title("Pairwise t-tests (Holm)")
        .render_svg_native_with_size(560, 480)
}

#[cfg(feature = "regression")]
fn logistic() -> Result<String, GGError> {
    let x: Vec<f64> = (0..80).map(|i| i as f64 * 0.125).collect();
    let y: Vec<f64> = (0..80)
        .map(|i| {
            if (((i * 7) % 11) as f64) < (i as f64 / 80.0 * 11.0) {
                1.0
            } else {
                0.0
            }
        })
        .collect();
    GGPlot::new(vec![
        ("dose".to_string(), f(x)),
        ("response".to_string(), f(y)),
    ])
    .aes(Aes::new().x("dose").y("response"))
    .geom_point()
    .geom_smooth_with(GeomSmooth::default().glm(SmoothFamily::binomial()))
    .theme_bw()
    .title("glm(response ~ dose, family = binomial)")
    .render_svg_native_with_size(560, 420)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/gallery-native".to_string());
    std::fs::create_dir_all(&dir)?;
    std::fs::write(format!("{dir}/diagnostics_grid.svg"), diagnostics()?)?;
    std::fs::write(format!("{dir}/bracket_table.svg"), brackets()?)?;
    #[cfg(feature = "regression")]
    std::fs::write(format!("{dir}/glm_binomial.svg"), logistic()?)?;
    println!("wrote native SVG gallery to {dir}/");
    Ok(())
}
