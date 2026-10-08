//! Regression-diagnostic gallery on the plotters-free native SVG path (the
//! one the DuckDB extensions use): QQ with confidence bands, residuals vs
//! leverage with Cook's contours, Kaplan–Meier with a step CI ribbon and
//! censor marks, ECDF with a DKW band, a dodged multi-model coefficient
//! forest and data-mapped reference lines per facet.
//!
//! Run with: `cargo run --no-default-features --example diagnostics`
//! Writes SVGs to `assets/gallery/diagnostics/`. Input data are the R
//! validation fixtures in `validation/fixtures/diag/`.

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::prelude::*;

const OUT: &str = "assets/gallery/diagnostics";
const FIX: &str = "validation/fixtures/diag";

fn csv(name: &str) -> DataFrame {
    DataFrame::from_csv(&format!("{FIX}/{name}")).expect("fixture")
}

fn save(name: &str, plot: GGPlot) -> Result<(), GGError> {
    let (svg, warnings) = plot.render_svg_native_with_warnings(640, 440)?;
    for w in warnings {
        eprintln!("{name}: {w}");
    }
    std::fs::write(format!("{OUT}/{name}.svg"), svg).expect("write svg");
    Ok(())
}

fn floats(v: impl IntoIterator<Item = f64>) -> Vec<Value> {
    v.into_iter().map(Value::Float).collect()
}
fn strs<'a>(v: impl IntoIterator<Item = &'a str>) -> Vec<Value> {
    v.into_iter().map(|s| Value::Str(s.to_string())).collect()
}

/// Normal QQ of residuals with a pointwise 95% band and a KS band.
fn qq() -> Result<(), GGError> {
    let plot = GGPlot::new(csv("qq_input.csv"))
        .aes(Aes::new().y("resid"))
        .geom_qq_band_with(
            GeomQQBand {
                fill: (198, 219, 239),
                alpha: 0.8,
            },
            StatQQBand::default().band(QQBandType::Ks),
        )
        .geom_qq_band()
        .stat_qq(QQDistribution::normal())
        .stat_qq_line(QQDistribution::normal())
        .theme_bw()
        .title("Normal Q-Q with pointwise and KS bands")
        .xlab("Theoretical quantiles")
        .ylab("Sample quantiles");
    save("qq_band", plot)
}

/// The same residuals against a t(5) reference distribution.
fn qq_t() -> Result<(), GGError> {
    let plot = GGPlot::new(csv("qq_input.csv"))
        .aes(Aes::new().y("resid"))
        .stat_qq_band(StatQQBand::new(QQDistribution::t(5.0)))
        .stat_qq(QQDistribution::t(5.0))
        .stat_qq_line(QQDistribution::t(5.0))
        .theme_bw()
        .title("Q-Q against Student t(5)")
        .xlab("t(5) quantiles")
        .ylab("Sample quantiles");
    save("qq_t", plot)
}

/// Residuals vs leverage (plot.lm which = 5) for lm(mpg ~ wt + hp, mtcars).
fn residuals_leverage() -> Result<(), GGError> {
    let plot = GGPlot::new(csv("cooks_mtcars.csv"))
        .aes(Aes::new().x("h").y("r"))
        .geom_hline(0.0)
        .geom_point()
        .stat_cooks_contour(3, &[0.5, 1.0])
        .theme_bw()
        .title("Residuals vs leverage")
        .xlab("Leverage")
        .ylab("Standardized residuals");
    save("residuals_leverage", plot)
}

/// Kaplan–Meier for survival::lung: step curve, 95% CI step ribbon and
/// censor marks.
fn kaplan_meier() -> Result<(), GGError> {
    let plot = GGPlot::new(csv("km_lung.csv"))
        .aes(Aes::new().x("time").y("surv"))
        .geom_stepribbon()
        .layer_aes(Aes::new().x("time").ymin("lower").ymax("upper"))
        .geom_step()
        .geom_censor_marks("n_censor")
        .theme_bw()
        .title("Kaplan-Meier: survival::lung")
        .xlab("Days")
        .ylab("Survival probability");
    save("kaplan_meier", plot)
}

/// ECDF per group with a simultaneous 95% DKW band.
fn ecdf_band() -> Result<(), GGError> {
    let input = csv("qq_input.csv");
    let n = input.nrows();
    let mut df = input;
    df.add_column(
        "half".into(),
        strs((0..n).map(|i| if i % 2 == 0 { "even" } else { "odd" })),
    );
    let plot = GGPlot::new(df)
        .aes(Aes::new().x("resid").color("half").fill("half"))
        .stat_ecdf_band(0.95)
        .stat_ecdf()
        .theme_bw()
        .title("ECDF with DKW 95% band")
        .ylab("F(x)");
    save("ecdf_band", plot)
}

/// Multi-model coefficient forest: dodged point ranges, zero line, flipped.
fn forest() -> Result<(), GGError> {
    let terms = ["(Intercept)", "wt", "hp", "qsec", "am"];
    let models = ["OLS", "Ridge", "Lasso"];
    let mut term = Vec::new();
    let mut model = Vec::new();
    let (mut est, mut lo, mut hi) = (Vec::new(), Vec::new(), Vec::new());
    for (i, t) in terms.iter().enumerate() {
        for (j, m) in models.iter().enumerate() {
            let e = [1.2, -2.8, -0.4, 0.9, 1.6][i] * (1.0 - 0.18 * j as f64);
            let se = 0.35 + 0.1 * i as f64 - 0.05 * j as f64;
            term.push(*t);
            model.push(*m);
            est.push(e);
            lo.push(e - 1.96 * se);
            hi.push(e + 1.96 * se);
        }
    }
    let data: Vec<(String, Vec<Value>)> = vec![
        ("term".into(), strs(term)),
        ("model_id".into(), strs(model)),
        ("estimate".into(), floats(est)),
        ("conf_low".into(), floats(lo)),
        ("conf_high".into(), floats(hi)),
    ];
    let plot = GGPlot::new(data)
        .aes(
            Aes::new()
                .x("term")
                .y("estimate")
                .ymin("conf_low")
                .ymax("conf_high")
                .color("model_id"),
        )
        .geom_hline(0.0)
        .geom_pointrange()
        .position(position_dodge(0.6))
        .coord_flip()
        .theme_bw()
        .title("Coefficient forest, three models")
        .xlab("")
        .ylab("Estimate (95% CI)");
    save("forest", plot)
}

/// ACF per model with per-panel ±1.96/√n bounds as data-mapped hlines.
fn acf_bounds() -> Result<(), GGError> {
    let mut lag = Vec::new();
    let mut acf = Vec::new();
    let mut model = Vec::new();
    for (m, phi) in [("AR(1) 0.7", 0.7f64), ("AR(1) -0.4", -0.4)] {
        for k in 1..=15 {
            lag.push(k as f64);
            acf.push(phi.powi(k));
            model.push(m);
        }
    }
    let data: Vec<(String, Vec<Value>)> = vec![
        ("lag".into(), floats(lag)),
        ("acf".into(), floats(acf)),
        ("model".into(), strs(model)),
    ];
    let bounds: Vec<(String, Vec<Value>)> = vec![
        ("bound".into(), floats([0.28, -0.28, 0.18, -0.18])),
        (
            "model".into(),
            strs(["AR(1) 0.7", "AR(1) 0.7", "AR(1) -0.4", "AR(1) -0.4"]),
        ),
    ];
    let plot = GGPlot::new(data)
        .aes(Aes::new().x("lag").y("acf"))
        .geom_col()
        .geom_hline(0.0)
        .geom_hline_aes_with(
            GeomHline {
                color: (33, 102, 172),
                ..GeomHline::mapped()
            },
            Aes::new().yintercept("bound"),
        )
        .layer_data(bounds)
        .facet_wrap("model", None)
        .theme_bw()
        .title("ACF with per-model significance bounds");
    save("acf_bounds", plot)
}

fn main() -> Result<(), GGError> {
    std::fs::create_dir_all(OUT).expect("output dir");
    qq()?;
    qq_t()?;
    residuals_leverage()?;
    kaplan_meier()?;
    ecdf_band()?;
    forest()?;
    acf_bounds()?;
    println!("Diagnostics gallery written to {OUT}/");
    Ok(())
}
