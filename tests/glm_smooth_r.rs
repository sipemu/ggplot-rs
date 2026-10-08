//! `geom_smooth(method = "glm", family = …)` validated against R.
//!
//! Reference values were generated with R 4.6.1 (+ MASS for `glm.nb`) by
//! fitting `glm(y ~ x, family = …, control = glm.control(epsilon = 1e-14,
//! maxit = 100))` (`MASS::glm.nb` for the negative binomial) on the
//! deterministic data below and evaluating ggplot2's `predictdf.glm` recipe on
//! a 5-point grid. The tight `epsilon` makes R report the *converged* fit: at
//! R's default `1e-8` its `se.fit` still uses the IRLS weights of the
//! penultimate iteration (binomial bands then differ in the 6th digit), while
//! anofox-regression evaluates the covariance at the final estimate.
//!
//!
//! ```r
//! i <- 0:39; x <- i * 0.25
//! binom  <- as.numeric(((i*7) %% 11) < (i/40*11))
//! gamma  <- exp(0.5 + 0.15*x) * (0.6 + ((i*7) %% 5)*0.2)
//! negbin <- round(exp(0.3 + 0.25*x) * (0.3 + ((i*13) %% 7)*0.3))
//! pois   <- round(exp(0.2 + 0.2*x)) + ((i*3) %% 4)
//! grid <- data.frame(x = seq(min(x), max(x), length.out = 5))
//! p <- predict(m, newdata = grid, type = "link", se.fit = TRUE)
//! fit <- m$family$linkinv(p$fit)
//! lo  <- m$family$linkinv(p$fit - qnorm(0.975) * p$se.fit)
//! hi  <- m$family$linkinv(p$fit + qnorm(0.975) * p$se.fit)
//! ```
//!
//! (For the inverse link `lo`/`hi` come out swapped; the smooth reports the
//! band as `ymin ≤ ymax`.)
#![cfg(feature = "regression")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::prelude::*;
use ggplot_rs::scale::ScaleSet;
use ggplot_rs::stat::smooth::StatSmooth;
use ggplot_rs::stat::Stat;

const LOGIT_FIT: [f64; 5] = [
    0.0697025663731,
    0.214900790475,
    0.5,
    0.785099209525,
    0.930297433627,
];
const LOGIT_LO: [f64; 5] = [
    0.0127455040571,
    0.0833808213302,
    0.31725468897,
    0.548346834981,
    0.696943517064,
];
const LOGIT_HI: [f64; 5] = [
    0.303056482936,
    0.451653165019,
    0.68274531103,
    0.91661917867,
    0.987254495943,
];
const PROBIT_FIT: [f64; 5] = [
    0.0591219048082,
    0.216054572142,
    0.496414517488,
    0.778641251043,
    0.938731554338,
];
const PROBIT_LO: [f64; 5] = [
    0.00563565016462,
    0.0796219864974,
    0.324306089809,
    0.559242929848,
    0.717916901982,
];
const PROBIT_HI: [f64; 5] = [
    0.277527643252,
    0.435040208468,
    0.669203855566,
    0.917153222901,
    0.993993468462,
];
const CLOGLOG_FIT: [f64; 5] = [
    0.11077759567,
    0.235561305648,
    0.45911649144,
    0.754880792494,
    0.959915387,
];
const CLOGLOG_LO: [f64; 5] = [
    0.0317217308503,
    0.106991177111,
    0.301388769253,
    0.556900609859,
    0.736797726317,
];
const CLOGLOG_HI: [f64; 5] = [
    0.347939989571,
    0.471455135304,
    0.651113860906,
    0.911848468389,
    0.999570099213,
];
const GAMMA_INV_FIT: [f64; 5] = [
    1.93142643628,
    2.39843266934,
    3.16329715541,
    4.64440683779,
    8.73365137603,
];
const GAMMA_INV_LO: [f64; 5] = [
    2.23387231778,
    2.72196021129,
    3.51209643879,
    5.18462917961,
    12.0863003228,
];
const GAMMA_INV_HI: [f64; 5] = [
    1.7011114974,
    2.14364293156,
    2.87752000411,
    4.20614004102,
    6.83709309501,
];
const GAMMA_LOG_FIT: [f64; 5] = [
    1.60083725851,
    2.34155023809,
    3.42499369523,
    5.00975021656,
    7.32777910433,
];
const GAMMA_LOG_LO: [f64; 5] = [
    1.34193112183,
    2.08168688548,
    3.13062247153,
    4.4537721872,
    6.14264490767,
];
const GAMMA_LOG_HI: [f64; 5] = [
    1.90969557718,
    2.63385312928,
    3.74704453157,
    5.63513268695,
    8.7415677463,
];
const POISSON_FIT: [f64; 5] = [
    2.38818718001,
    3.39347655068,
    4.82193489537,
    6.85169200022,
    9.73586004052,
];
const POISSON_LO: [f64; 5] = [
    1.72241461257,
    2.71504792121,
    4.16335688199,
    5.90216945647,
    7.75389109057,
];
const POISSON_HI: [f64; 5] = [
    3.31130377388,
    4.24142904072,
    5.58468966131,
    7.95397075804,
    12.2244392682,
];
// glm.nb: theta = 8.418842289 (estimated by ML, as MASS does).
const NEGBIN_FIT: [f64; 5] = [
    1.57752155863,
    2.96953946497,
    5.58988533994,
    10.5224458143,
    19.8075379336,
];
const NEGBIN_LO: [f64; 5] = [
    1.04373957577,
    2.24305355629,
    4.6591071242,
    8.75131153628,
    14.8986433973,
];
const NEGBIN_HI: [f64; 5] = [
    2.38428658422,
    3.93132148328,
    6.70661079059,
    12.652031122,
    26.333844534,
];

fn xs() -> Vec<f64> {
    (0..40).map(|i| i as f64 * 0.25).collect()
}

fn binom() -> Vec<f64> {
    (0..40)
        .map(|i| {
            if (((i * 7) % 11) as f64) < (i as f64 / 40.0 * 11.0) {
                1.0
            } else {
                0.0
            }
        })
        .collect()
}

fn gamma() -> Vec<f64> {
    (0..40)
        .map(|i| {
            let x = i as f64 * 0.25;
            (0.5 + 0.15 * x).exp() * (0.6 + ((i * 7) % 5) as f64 * 0.2)
        })
        .collect()
}

fn negbin() -> Vec<f64> {
    (0..40)
        .map(|i| {
            let x = i as f64 * 0.25;
            ((0.3 + 0.25 * x).exp() * (0.3 + ((i * 13) % 7) as f64 * 0.3)).round()
        })
        .collect()
}

fn pois() -> Vec<f64> {
    (0..40)
        .map(|i| {
            let x = i as f64 * 0.25;
            (0.2 + 0.2 * x).exp().round() + ((i * 3) % 4) as f64
        })
        .collect()
}

fn frame(y: Vec<f64>) -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column("x".into(), xs().into_iter().map(Value::Float).collect());
    df.add_column("y".into(), y.into_iter().map(Value::Float).collect());
    df
}

fn col(df: &DataFrame, name: &str) -> Vec<f64> {
    df.column(name)
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

fn check(family: SmoothFamily, y: Vec<f64>, fit: [f64; 5], lo: [f64; 5], hi: [f64; 5]) {
    let stat = StatSmooth {
        n_points: 5,
        se: true,
        method: SmoothMethod::Glm { family },
    };
    let out = stat.compute_group(&frame(y), &ScaleSet::new());
    assert_eq!(out.nrows(), 5, "{family:?}: 5 grid points");
    let (f, mn, mx) = (col(&out, "y"), col(&out, "ymin"), col(&out, "ymax"));
    for k in 0..5 {
        let (rlo, rhi) = (lo[k].min(hi[k]), lo[k].max(hi[k]));
        // Fitted means agree to 1e-6; bands to 1e-5 (the IRLS stops at the
        // SQL extension's tolerance 1e-8, which leaves non-canonical links'
        // standard errors a few 1e-6 from R's fully converged values).
        let tol = |r: f64| 1e-5 * r.abs().max(1.0);
        assert!(
            (f[k] - fit[k]).abs() < 1e-6 * fit[k].abs().max(1.0),
            "{family:?} fit[{k}]: {} vs R {}",
            f[k],
            fit[k]
        );
        assert!(
            (mn[k] - rlo).abs() < tol(rlo),
            "{family:?} ymin[{k}]: {} vs R {}",
            mn[k],
            rlo
        );
        assert!(
            (mx[k] - rhi).abs() < tol(rhi),
            "{family:?} ymax[{k}]: {} vs R {}",
            mx[k],
            rhi
        );
    }
}

#[test]
fn binomial_logit_matches_r() {
    check(
        SmoothFamily::binomial(),
        binom(),
        LOGIT_FIT,
        LOGIT_LO,
        LOGIT_HI,
    );
}

#[test]
fn binomial_probit_matches_r() {
    check(
        SmoothFamily::Binomial(SmoothBinomialLink::Probit),
        binom(),
        PROBIT_FIT,
        PROBIT_LO,
        PROBIT_HI,
    );
}

#[test]
fn binomial_cloglog_matches_r() {
    check(
        SmoothFamily::Binomial(SmoothBinomialLink::Cloglog),
        binom(),
        CLOGLOG_FIT,
        CLOGLOG_LO,
        CLOGLOG_HI,
    );
}

#[test]
fn gamma_inverse_matches_r() {
    check(
        SmoothFamily::gamma(),
        gamma(),
        GAMMA_INV_FIT,
        GAMMA_INV_LO,
        GAMMA_INV_HI,
    );
}

#[test]
fn gamma_log_matches_r() {
    check(
        SmoothFamily::Gamma(SmoothGammaLink::Log),
        gamma(),
        GAMMA_LOG_FIT,
        GAMMA_LOG_LO,
        GAMMA_LOG_HI,
    );
}

#[test]
fn poisson_matches_r() {
    check(
        SmoothFamily::Poisson,
        pois(),
        POISSON_FIT,
        POISSON_LO,
        POISSON_HI,
    );
}

#[test]
fn negative_binomial_matches_r_glm_nb() {
    check(
        SmoothFamily::NegativeBinomial,
        negbin(),
        NEGBIN_FIT,
        NEGBIN_LO,
        NEGBIN_HI,
    );
}

#[test]
fn binomial_band_stays_in_unit_interval() {
    let stat = StatSmooth {
        n_points: 50,
        se: true,
        method: SmoothMethod::Glm {
            family: SmoothFamily::binomial(),
        },
    };
    let out = stat.compute_group(&frame(binom()), &ScaleSet::new());
    for v in col(&out, "ymin").into_iter().chain(col(&out, "ymax")) {
        assert!((0.0..=1.0).contains(&v), "band value {v} outside [0, 1]");
    }
}

#[test]
fn invalid_response_skips_layer_with_warning() {
    // A binomial response outside [0, 1] cannot be fitted: the layer is
    // skipped with a warning, the rest of the plot still renders.
    let y: Vec<Value> = (0..20).map(|i| Value::Float(i as f64 * 3.0)).collect();
    let x: Vec<Value> = (0..20).map(|i| Value::Float(i as f64)).collect();
    let (svg, warnings) = GGPlot::new(vec![("x".to_string(), x), ("y".to_string(), y)])
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth::default().glm(SmoothFamily::binomial()))
        .render_svg_native_with_warnings(400, 300)
        .expect("plot still renders");
    assert!(svg.contains("<circle"));
    assert!(
        warnings.iter().any(|w| w.contains("geom_smooth")),
        "{warnings:?}"
    );
}

#[test]
fn glm_smooth_renders_natively_with_band() {
    let data = vec![
        (
            "x".to_string(),
            xs().into_iter().map(Value::Float).collect::<Vec<_>>(),
        ),
        (
            "y".to_string(),
            binom().into_iter().map(Value::Float).collect::<Vec<_>>(),
        ),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth::default().glm(SmoothFamily::binomial()))
        .render_svg_native_with_size(500, 350)
        .unwrap();
    assert!(svg.contains("<polygon"), "confidence band drawn");
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
}
