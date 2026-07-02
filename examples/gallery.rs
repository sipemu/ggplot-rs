//! Generates the plot gallery shown in the README.
//!
//! Run with: `cargo run --example gallery`
//! Writes PNGs to `assets/gallery/`.

use ggplot_rs::prelude::*;
use polars::prelude::*;
use std::f64::consts::PI;

const W: u32 = 640;
const H: u32 = 480;

fn out(name: &str) -> String {
    format!("assets/gallery/{name}.png")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("assets/gallery")?;

    scatter()?;
    smooth()?;
    histogram()?;
    bar()?;
    boxplot()?;
    continuous_color()?;
    facet()?;
    density()?;

    println!("Gallery written to assets/gallery/");
    Ok(())
}

/// Grouped scatter with a qualitative Brewer palette.
fn scatter() -> Result<(), Box<dyn std::error::Error>> {
    let n = 150;
    let x: Vec<f64> = (0..n).map(|i| 4.5 + i as f64 * 0.02).collect();
    let y: Vec<f64> = (0..n)
        .map(|i| 2.5 + (i as f64 * 0.15).sin() + (i % 3) as f64 * 0.6)
        .collect();
    let species: Vec<&str> = (0..n)
        .map(|i| ["setosa", "versicolor", "virginica"][i % 3])
        .collect();

    let df = df! { "x" => x, "y" => y, "species" => species }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("species"))
        .geom_point()
        .scale_color_brewer(PaletteName::Set1)
        .title("Grouped Scatter")
        .xlab("Sepal Length")
        .ylab("Sepal Width")
        .theme_minimal()
        .save_with_size(&out("scatter"), W, H)?;
    Ok(())
}

/// Points overlaid with a LOESS trend line and confidence band.
fn smooth() -> Result<(), Box<dyn std::error::Error>> {
    let n = 120;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 * 0.1;
            (t * 0.6).sin() * 3.0 + t * 0.2 + ((i * 7919 % 100) as f64 / 100.0 - 0.5) * 1.5
        })
        .collect();

    let df = df! { "x" => x, "y" => y }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth {
            method: SmoothMethod::Loess { span: 0.5 },
            ..Default::default()
        })
        .title("LOESS Smoothing")
        .xlab("x")
        .ylab("y")
        .theme_bw()
        .save_with_size(&out("smooth"), W, H)?;
    Ok(())
}

/// Histogram of an approximately-normal sample.
fn histogram() -> Result<(), Box<dyn std::error::Error>> {
    let values: Vec<f64> = (0..1500)
        .map(|i: i32| {
            let r: f64 = (0..6)
                .map(|k| ((i * (1237 + k * 311) + 5678) % 1000) as f64 / 1000.0)
                .sum();
            (r - 3.0) * 2.0
        })
        .collect();

    let df = df! { "measurement" => values }?;
    GGPlot::new(df)
        .aes(Aes::new().x("measurement"))
        .geom_histogram_with(GeomHistogram {
            bins: 30,
            ..Default::default()
        })
        .title("Histogram")
        .xlab("Value")
        .ylab("Count")
        .theme_minimal()
        .save_with_size(&out("histogram"), W, H)?;
    Ok(())
}

/// Bar chart of category counts with a fill palette.
fn bar() -> Result<(), Box<dyn std::error::Error>> {
    let mut fruit: Vec<&str> = Vec::new();
    for (f, c) in [
        ("Apple", 8),
        ("Banana", 5),
        ("Cherry", 11),
        ("Date", 3),
        ("Elder", 7),
    ] {
        for _ in 0..c {
            fruit.push(f);
        }
    }
    let df = df! { "fruit" => fruit }?;
    GGPlot::new(df)
        .aes(Aes::new().x("fruit").fill("fruit"))
        .geom_bar()
        .scale_fill_brewer(PaletteName::Set2)
        .title("Bar Chart")
        .xlab("Fruit")
        .ylab("Count")
        .theme_minimal()
        .save_with_size(&out("bar"), W, H)?;
    Ok(())
}

/// Grouped boxplots.
fn boxplot() -> Result<(), Box<dyn std::error::Error>> {
    let n = 240;
    let group: Vec<&str> = (0..n).map(|i| ["A", "B", "C", "D"][i % 4]).collect();
    let value: Vec<f64> = (0..n)
        .map(|i| {
            let base = (i % 4) as f64 * 1.5;
            base + (i as f64 * 0.4).sin() * 1.2 + ((i * 6151 % 100) as f64 / 100.0 - 0.5) * 2.0
        })
        .collect();

    let df = df! { "group" => group, "value" => value }?;
    GGPlot::new(df)
        .aes(Aes::new().x("group").y("value"))
        .geom_boxplot_with(GeomBoxplot {
            fill: (70, 130, 180),
            ..Default::default()
        })
        .title("Boxplot")
        .xlab("Group")
        .ylab("Value")
        .theme_bw()
        .save_with_size(&out("boxplot"), W, H)?;
    Ok(())
}

/// Spiral scatter coloured by a continuous variable (viridis).
fn continuous_color() -> Result<(), Box<dyn std::error::Error>> {
    let n = 400;
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 * 0.05;
            t.cos() * (1.0 + t * 0.12)
        })
        .collect();
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 * 0.05;
            t.sin() * (1.0 + t * 0.12)
        })
        .collect();
    let z: Vec<f64> = (0..n).map(|i| i as f64 * 0.05).collect();

    let df = df! { "x" => x, "y" => y, "z" => z }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .scale_color_viridis_c()
        .title("Continuous Color (viridis)")
        .xlab("x")
        .ylab("y")
        .theme_minimal()
        .save_with_size(&out("continuous_color"), W, H)?;
    Ok(())
}

/// Faceted scatter, one panel per group.
fn facet() -> Result<(), Box<dyn std::error::Error>> {
    let n = 180;
    let x: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).cos() * 3.0).collect();
    let y: Vec<f64> = (0..n)
        .map(|i| (i as f64 * 0.1).sin() * 3.0 + (i % 3) as f64)
        .collect();
    let species: Vec<&str> = (0..n)
        .map(|i| ["setosa", "versicolor", "virginica"][i % 3])
        .collect();

    let df = df! { "x" => x, "y" => y, "species" => species }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("species"))
        .geom_point()
        .facet_wrap("species", Some(3))
        .scale_color_brewer(PaletteName::Set1)
        .title("Facet Wrap")
        .xlab("x")
        .ylab("y")
        .theme_bw()
        .save_with_size(&out("facet"), W, H)?;
    Ok(())
}

/// Overlapping density curves by group.
fn density() -> Result<(), Box<dyn std::error::Error>> {
    let n = 600;
    let group: Vec<&str> = (0..n).map(|i| ["Group 1", "Group 2"][i % 2]).collect();
    let value: Vec<f64> = (0..n)
        .map(|i| {
            let shift = (i % 2) as f64 * 2.5;
            let t = i as f64 * 0.05;
            shift + (t.sin() + (t * 1.7).cos()) + ((i * 3319 % 100) as f64 / 100.0 - 0.5) * PI
        })
        .collect();

    let df = df! { "value" => value, "group" => group }?;
    GGPlot::new(df)
        .aes(Aes::new().x("value").fill("group").color("group"))
        .geom_density()
        .scale_fill_brewer(PaletteName::Set1)
        .scale_color_brewer(PaletteName::Set1)
        .title("Density by Group")
        .xlab("Value")
        .ylab("Density")
        .theme_minimal()
        .save_with_size(&out("density"), W, H)?;
    Ok(())
}
