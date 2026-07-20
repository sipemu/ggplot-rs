//! Generates the gallery images for the ggpubr-style / GAM features.
//!
//! Kept separate from `gallery.rs` (which uses polars `df!`) so it builds
//! polars-free — run with:
//!
//! ```text
//! cargo run --no-default-features --features regression,ggpubr --example ggpubr_gallery
//! ```
//!
//! Writes PNGs to `assets/gallery/`. Data comes from a seeded RNG with
//! realistic distributions so the plots look like genuine analyses.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const W: u32 = 640;
const H: u32 = 480;

fn out(name: &str) -> String {
    format!("assets/gallery/{name}.png")
}

fn seeded(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

/// Approximate standard-normal draw (Irwin–Hall: sum of 12 uniforms − 6).
fn randn(r: &mut StdRng) -> f64 {
    (0..12).map(|_| r.gen::<f64>()).sum::<f64>() - 6.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn col_f(v: Vec<f64>) -> Vec<Value> {
    v.into_iter().map(Value::Float).collect()
}
fn col_s(v: Vec<&str>) -> Vec<Value> {
    v.into_iter().map(|s| Value::Str(s.to_string())).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("assets/gallery")?;
    palettes_ggsci()?;
    theme_pubr_demo()?;
    smooth_gam()?;
    stat_cor_demo()?;
    compare_means_demo()?;
    brackets_demo()?;
    arrange_demo()?;
    println!("ggpubr gallery written to assets/gallery/");
    Ok(())
}

/// Five well-separated Gaussian clusters, coloured with the Nature Publishing
/// Group (ggsci `npg`) palette.
fn palettes_ggsci() -> Result<(), Box<dyn std::error::Error>> {
    let centers = [
        ("A", 1.0, 1.2),
        ("B", 3.2, 2.6),
        ("C", 5.0, 1.4),
        ("D", 2.2, 4.2),
        ("E", 4.6, 4.4),
    ];
    let mut r = seeded(21);
    let (mut x, mut y, mut grp) = (Vec::new(), Vec::new(), Vec::new());
    for (name, cx, cy) in centers {
        for _ in 0..45 {
            x.push(round2(cx + randn(&mut r) * 0.45));
            y.push(round2(cy + randn(&mut r) * 0.45));
            grp.push(name);
        }
    }
    GGPlot::new(vec![
        ("x".to_string(), col_f(x)),
        ("y".to_string(), col_f(y)),
        ("grp".to_string(), col_s(grp)),
    ])
    .aes(Aes::new().x("x").y("y").color("grp"))
    .geom_point()
    .scale_color_brewer(PaletteName::Npg)
    .title("ggsci Journal Palette (npg)")
    .xlab("x")
    .ylab("y")
    .theme_pubr()
    .save_with_size(&out("palettes_ggsci"), W, H)?;
    Ok(())
}

/// The publication-ready `theme_pubr()` on a grouped boxplot.
fn theme_pubr_demo() -> Result<(), Box<dyn std::error::Error>> {
    let (xs, ys) = grouped_samples(&[("ctrl", 5.0, 0.8), ("trt1", 7.5, 1.1), ("trt2", 6.2, 0.9)], 31);
    GGPlot::new(vec![("grp".to_string(), xs), ("val".to_string(), ys)])
        .aes(Aes::new().x("grp").y("val").fill("grp"))
        .geom_boxplot()
        .scale_fill_brewer(PaletteName::Jco)
        .title("theme_pubr()")
        .xlab("group")
        .ylab("response")
        .theme_pubr()
        .save_with_size(&out("theme_pubr"), W, H)?;
    Ok(())
}

/// GAM (penalized B-spline) smoother over a noisy nonlinear signal.
fn smooth_gam() -> Result<(), Box<dyn std::error::Error>> {
    let n = 120;
    let mut r = seeded(41);
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&t| (t * 0.6).sin() * 3.0 + t * 0.15 + randn(&mut r) * 0.9)
        .collect();
    GGPlot::new(vec![("x".to_string(), col_f(x)), ("y".to_string(), col_f(y))])
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth::default().gam())
        .title("GAM Smoothing (P-spline, method = \"gam\")")
        .xlab("x")
        .ylab("y")
        .theme_bw()
        .save_with_size(&out("smooth_gam"), W, H)?;
    Ok(())
}

/// Scatter annotated with a Pearson correlation label (`stat_cor`).
fn stat_cor_demo() -> Result<(), Box<dyn std::error::Error>> {
    let n = 80;
    let mut r = seeded(42);
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&t| round2(1.5 * t + 2.0 + randn(&mut r) * 2.2))
        .collect();
    GGPlot::new(vec![("x".to_string(), col_f(x)), ("y".to_string(), col_f(y))])
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .stat_cor()
        .title("stat_cor(): Pearson R + p-value")
        .xlab("x")
        .ylab("y")
        .theme_pubr()
        .save_with_size(&out("stat_cor"), W, H)?;
    Ok(())
}

/// Grouped boxplot annotated with a group-comparison p-value
/// (`stat_compare_means`, Kruskal-Wallis for >2 groups).
fn compare_means_demo() -> Result<(), Box<dyn std::error::Error>> {
    let (xs, ys) = grouped_samples(&[("ctrl", 5.0, 0.9), ("trt1", 7.8, 1.1), ("trt2", 6.4, 1.0)], 31);
    GGPlot::new(vec![("grp".to_string(), xs), ("val".to_string(), ys)])
        .aes(Aes::new().x("grp").y("val").fill("grp"))
        .geom_boxplot()
        .stat_compare_means()
        .scale_fill_brewer(PaletteName::Npg)
        .title("stat_compare_means()")
        .xlab("group")
        .ylab("response")
        .theme_pubr()
        .save_with_size(&out("compare_means"), W, H)?;
    Ok(())
}

/// Grouped boxplot with pairwise significance brackets (auto p-values).
fn brackets_demo() -> Result<(), Box<dyn std::error::Error>> {
    let (xs, ys) = grouped_samples(&[("ctrl", 5.0, 0.9), ("trt1", 8.0, 1.1), ("trt2", 6.4, 1.0)], 31);
    GGPlot::new(vec![("grp".to_string(), xs), ("val".to_string(), ys)])
        .aes(Aes::new().x("grp").y("val").fill("grp"))
        .geom_boxplot()
        // Auto pairwise brackets: each labelled with its computed Wilcoxon p-value.
        .stat_compare_means_pairwise(&[("ctrl", "trt1"), ("trt1", "trt2"), ("ctrl", "trt2")])
        .scale_fill_brewer(PaletteName::Npg)
        .title("stat_compare_means(comparisons): pairwise p-values")
        .xlab("group")
        .ylab("response")
        .theme_pubr()
        .save_with_size(&out("brackets"), W, H)?;
    Ok(())
}

/// Four plots composed into a 2×2 grid with `ggarrange` (native PNG).
fn arrange_demo() -> Result<(), Box<dyn std::error::Error>> {
    let n = 70;
    let mut r = seeded(51);
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&t| round2((t * 0.6).sin() * 2.0 + randn(&mut r) * 0.5))
        .collect();
    let xy = || {
        vec![
            ("x".to_string(), col_f(x.clone())),
            ("y".to_string(), col_f(y.clone())),
        ]
    };
    let (gx, gy) = grouped_samples(&[("a", 5.0, 0.8), ("b", 7.5, 1.0), ("c", 6.0, 0.9)], 31);

    let plots = vec![
        ggscatter(xy(), "x", "y", None).title("scatter"),
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .geom_smooth_with(GeomSmooth::default().gam())
            .title("gam")
            .theme_pubr(),
        ggboxplot(
            vec![("g".to_string(), gx), ("val".to_string(), gy)],
            "g",
            "val",
            Some("g"),
        )
        .scale_fill_brewer(PaletteName::Npg)
        .title("boxplot"),
        ggdensity(xy(), "y", None).title("density"),
    ];
    // Native PNG output — no external rasteriser needed.
    ggarrange_save_png(plots, 2, 340, 250, &out("arrange"))?;
    Ok(())
}

/// Build a grouped sample: for each `(name, mean, sd)`, `per_group` normal
/// draws, returned as parallel (category, value) `Value` columns.
fn grouped_samples(groups: &[(&str, f64, f64)], per_group: usize) -> (Vec<Value>, Vec<Value>) {
    let mut r = seeded(101);
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for &(name, mean, sd) in groups {
        for _ in 0..per_group {
            xs.push(Value::Str(name.to_string()));
            ys.push(Value::Float(round2(mean + randn(&mut r) * sd)));
        }
    }
    (xs, ys)
}
