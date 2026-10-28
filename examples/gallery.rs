//! Generates the plot gallery shown in the README.
//!
//! Run with: `cargo run --features sf --example gallery`
//! Writes PNGs to `assets/gallery/`.
//!
//! Data is drawn from a seeded RNG with realistic distributions so each plot
//! resembles a genuine analysis (natural point clouds, real group differences)
//! rather than an obviously-synthetic wave.

use ggplot_rs::prelude::*;
use polars::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const W: u32 = 640;
const H: u32 = 480;
// Smaller thumbnails for the theme gallery.
const TW: u32 = 480;
const TH: u32 = 340;

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

/// Iris-like sample: 50 flowers per species with realistic sepal length/width
/// (means/SDs close to Fisher's iris), so the three species form distinct,
/// naturally-scattered clusters.
fn iris(seed: u64) -> (Vec<f64>, Vec<f64>, Vec<&'static str>) {
    // (species, len mean, len sd, width mean, width sd)
    let params = [
        ("setosa", 5.01, 0.35, 3.43, 0.38),
        ("versicolor", 5.94, 0.52, 2.77, 0.31),
        ("virginica", 6.59, 0.64, 2.97, 0.32),
    ];
    let mut r = seeded(seed);
    let (mut x, mut y, mut sp) = (Vec::new(), Vec::new(), Vec::new());
    for (name, lm, ls, wm, ws) in params {
        for _ in 0..50 {
            x.push(round2(lm + randn(&mut r) * ls));
            y.push(round2(wm + randn(&mut r) * ws));
            sp.push(name);
        }
    }
    (x, y, sp)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("assets/gallery")?;

    scatter()?;
    smooth()?;
    histogram()?;
    bar()?;
    boxplot()?;
    violin()?;
    continuous_color()?;
    facet()?;
    density()?;
    contour_filled()?;
    hexbin()?;
    flows()?;
    heatmap()?;
    jitter()?;
    ribbon()?;
    area_stack()?;
    polar()?;
    ecdf()?;
    qq()?;
    candlestick()?;
    waterfall()?;
    pareto()?;
    sankey()?;
    chord()?;
    arcdiagram()?;
    treemap()?;
    sunburst()?;
    funnel()?;
    stream()?;
    spline()?;
    dumbbell()?;
    radar()?;
    gauge()?;
    calendar()?;
    #[cfg(feature = "sf")]
    spatial()?;
    legend_wrap()?;
    boxplot_identity()?;
    themes()?;

    println!("Gallery written to assets/gallery/");
    Ok(())
}

/// Grouped scatter of iris sepal dimensions with a qualitative palette.
fn scatter() -> Result<(), Box<dyn std::error::Error>> {
    let (x, y, species) = iris(1);
    let df = df! { "x" => x, "y" => y, "species" => species }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("species"))
        .geom_point()
        .scale_color_brewer(PaletteName::Set1)
        .title("Iris Sepal Dimensions")
        .xlab("Sepal Length (cm)")
        .ylab("Sepal Width (cm)")
        .theme_minimal()
        .save_with_size(&out("scatter"), W, H)?;
    Ok(())
}

/// Points overlaid with a LOESS trend line and confidence band.
fn smooth() -> Result<(), Box<dyn std::error::Error>> {
    let n = 120;
    let mut r = seeded(7);
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&t| (t * 0.6).sin() * 3.0 + t * 0.2 + randn(&mut r) * 0.8)
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

/// Histogram of a normally-distributed sample.
fn histogram() -> Result<(), Box<dyn std::error::Error>> {
    let mut r = seeded(11);
    let values: Vec<f64> = (0..1500).map(|_| randn(&mut r) * 2.0 + 10.0).collect();

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

/// Grouped boxplots — four treatments with distinct medians and spreads.
fn boxplot() -> Result<(), Box<dyn std::error::Error>> {
    // (group, mean, sd)
    let params = [
        ("A", 4.0, 0.8),
        ("B", 6.2, 1.3),
        ("C", 5.4, 0.6),
        ("D", 7.1, 1.0),
    ];
    let mut r = seeded(2);
    let (mut group, mut value) = (Vec::new(), Vec::new());
    for (name, mean, sd) in params {
        for _ in 0..60 {
            group.push(name);
            value.push(round2(mean + randn(&mut r) * sd));
        }
    }

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

/// Violin plots — bimodal distributions a boxplot would hide.
fn violin() -> Result<(), Box<dyn std::error::Error>> {
    // Each group mixes two normals so the violin shows two lobes.
    let params = [("X", -2.0, 2.5), ("Y", -1.0, 3.5), ("Z", 0.0, 4.0)];
    let mut r = seeded(3);
    let (mut group, mut value) = (Vec::new(), Vec::new());
    for (name, lo, hi) in params {
        for k in 0..120 {
            group.push(name);
            let center = if k % 2 == 0 { lo } else { hi };
            value.push(round2(center + randn(&mut r) * 0.7));
        }
    }

    let df = df! { "group" => group, "value" => value }?;
    GGPlot::new(df)
        .aes(Aes::new().x("group").y("value").fill("group"))
        .geom_violin()
        .scale_fill_brewer(PaletteName::Accent)
        .title("Violin (bimodal groups)")
        .xlab("Group")
        .ylab("Value")
        .theme_minimal()
        .save_with_size(&out("violin"), W, H)?;
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

/// Faceted iris scatter, one panel per species.
fn facet() -> Result<(), Box<dyn std::error::Error>> {
    let (x, y, species) = iris(4);
    let df = df! { "x" => x, "y" => y, "species" => species }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("species"))
        .geom_point()
        .facet_wrap("species", Some(3))
        .scale_color_brewer(PaletteName::Set1)
        .title("Facet Wrap")
        .xlab("Sepal Length (cm)")
        .ylab("Sepal Width (cm)")
        .theme_bw()
        .save_with_size(&out("facet"), W, H)?;
    Ok(())
}

/// Overlapping density curves for two separated groups.
fn density() -> Result<(), Box<dyn std::error::Error>> {
    let mut r = seeded(5);
    let (mut value, mut group) = (Vec::new(), Vec::new());
    for _ in 0..400 {
        value.push(round2(randn(&mut r) * 1.0 - 1.0));
        group.push("Group 1");
    }
    for _ in 0..400 {
        value.push(round2(randn(&mut r) * 1.2 + 2.0));
        group.push("Group 2");
    }

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

/// Filled contour bands from a gridded surface.
fn contour_filled() -> Result<(), Box<dyn std::error::Error>> {
    let (mut x, mut y, mut z) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..40 {
        for j in 0..40 {
            let xv = i as f64 * 0.25 - 5.0;
            let yv = j as f64 * 0.25 - 5.0;
            x.push(xv);
            y.push(yv);
            z.push((xv * 0.6).sin() * (yv * 0.6).cos() + (-(xv * xv + yv * yv) / 30.0).exp());
        }
    }
    let df = df! { "x" => x, "y" => y, "z" => z }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y"))
        .geom_contour_filled()
        .scale_fill_viridis_c()
        .title("Filled Contours")
        .theme_minimal()
        .save_with_size(&out("contour_filled"), W, H)?;
    Ok(())
}

/// Hexagonal binning of a dense bivariate-normal cloud.
fn hexbin() -> Result<(), Box<dyn std::error::Error>> {
    let n = 6000;
    let mut r = seeded(9);
    // Correlated 2-D Gaussian: y shares part of x's draw.
    let (mut x, mut y) = (Vec::new(), Vec::new());
    for _ in 0..n {
        let a = randn(&mut r);
        let b = randn(&mut r);
        x.push(a * 2.0);
        y.push(a * 1.1 + b * 1.4);
    }
    let df = df! { "x" => x, "y" => y }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y"))
        .geom_hex()
        .scale_fill_viridis_c()
        .title("Hex Binning")
        .theme_minimal()
        .save_with_size(&out("hexbin"), W, H)?;
    Ok(())
}

/// Directed flows between cities: curved arrows (B1-2/B1-3), sized by
/// `scale_linewidth` (B1-4) and coloured by `scale_colour_identity` (B1-6) —
/// the overlay style `anofox-visualization` draws on top of a basemap
/// (#308).
fn flows() -> Result<(), Box<dyn std::error::Error>> {
    let cities = df! {
        "x" => [0.0, 4.0, 2.0, 6.0, 1.0],
        "y" => [0.0, 1.0, 4.0, 3.5, 3.0],
        "name" => ["Berlin", "Warsaw", "Oslo", "Helsinki", "Copenhagen"],
    }?;
    let flows = df! {
        "x" => [0.0, 4.0, 2.0, 1.0],
        "y" => [0.0, 1.0, 4.0, 3.0],
        "xend" => [4.0, 2.0, 6.0, 0.0],
        "yend" => [1.0, 4.0, 3.5, 0.0],
        "size" => [12.0, 4.0, 7.0, 2.5],
        "trend" => ["#2f9e44", "#e03131", "#2f9e44", "#e03131"],
    }?;

    let mut theme = theme_minimal();
    theme.axis_text_x.visible = false;
    theme.axis_text_y.visible = false;

    GGPlot::new(flows)
        .aes(
            Aes::new()
                .x("x")
                .y("y")
                .xend("xend")
                .yend("yend")
                .color("trend")
                .linewidth("size"),
        )
        .geom_curve_with(
            GeomCurve::default()
                .curvature(0.2)
                .lineend(LineEnd::Round)
                .arrow(
                    arrow()
                        .closed()
                        .angle(26.565)
                        .length(5.0)
                        .length_per_linewidth(2.2)
                        .position(0.85),
                ),
        )
        .geom_point_with(GeomPoint {
            size: 5.0,
            color: (40, 44, 52),
            alpha: 1.0,
            ..Default::default()
        })
        .layer_data(cities)
        .layer_aes(Aes::new().x("x").y("y"))
        .scale_colour_identity()
        .scale_linewidth(ScaleLinewidthContinuous::new().with_range(0.75, 8.0))
        .title("Flows Between Cities")
        .theme(theme)
        .save_with_size(&out("flows"), W, H)?;
    Ok(())
}

/// Heatmap of a gridded value with `geom_tile`.
fn heatmap() -> Result<(), Box<dyn std::error::Error>> {
    let (mut x, mut y, mut fill) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..14 {
        for j in 0..14 {
            x.push(i as f64);
            y.push(j as f64);
            fill.push((i as f64 * 0.5).sin() * (j as f64 * 0.5).cos());
        }
    }
    let df = df! { "x" => x, "y" => y, "fill" => fill }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").fill("fill"))
        .geom_tile()
        .scale_fill_viridis_c()
        .title("Heatmap")
        .theme_minimal()
        .save_with_size(&out("heatmap"), W, H)?;
    Ok(())
}

/// Jittered dose–response scatter (`geom_jitter`).
fn jitter() -> Result<(), Box<dyn std::error::Error>> {
    let params = [("Control", 2.0, 0.6), ("Low", 3.6, 0.7), ("High", 5.1, 0.8)];
    let mut r = seeded(6);
    let (mut group, mut value) = (Vec::new(), Vec::new());
    for (name, mean, sd) in params {
        for _ in 0..100 {
            group.push(name);
            value.push(round2(mean + randn(&mut r) * sd));
        }
    }
    let df = df! { "group" => group, "value" => value }?;
    GGPlot::new(df)
        .aes(Aes::new().x("group").y("value").color("group"))
        .geom_jitter()
        .scale_color_brewer(PaletteName::Dark2)
        .title("Jittered Points")
        .xlab("Dose")
        .ylab("Response")
        .theme_minimal()
        .save_with_size(&out("jitter"), W, H)?;
    Ok(())
}

/// A confidence band (`geom_ribbon`) under a line.
fn ribbon() -> Result<(), Box<dyn std::error::Error>> {
    let n = 80;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.15).collect();
    let y: Vec<f64> = x.iter().map(|v| v.sin() + v * 0.1).collect();
    // Uncertainty widens with x.
    let ymin: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(t, v)| v - (0.25 + t * 0.05))
        .collect();
    let ymax: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(t, v)| v + (0.25 + t * 0.05))
        .collect();
    let df = df! { "x" => x, "y" => y, "ymin" => ymin, "ymax" => ymax }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").ymin("ymin").ymax("ymax"))
        .geom_ribbon()
        .geom_line()
        .primary_color((49, 130, 189))
        .title("Ribbon + Line")
        .theme_minimal()
        .save_with_size(&out("ribbon"), W, H)?;
    Ok(())
}

/// Stacked areas — three regions over time.
fn area_stack() -> Result<(), Box<dyn std::error::Error>> {
    let n = 40;
    let mut r = seeded(8);
    let (mut x, mut y, mut g) = (Vec::new(), Vec::new(), Vec::new());
    for (grp, base, amp) in [("North", 2.0, 1.2), ("South", 3.0, 1.6), ("East", 1.5, 0.9)] {
        // A smooth seasonal swell plus mild noise, always positive.
        for i in 0..n {
            x.push(i as f64);
            let seasonal = (i as f64 * 0.3).sin().abs() * amp;
            y.push(round2((base + seasonal + randn(&mut r) * 0.2).max(0.1)));
            g.push(grp);
        }
    }
    let df = df! { "x" => x, "y" => y, "g" => g }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").fill("g"))
        .geom_area()
        .scale_fill_brewer(PaletteName::Set2)
        .title("Stacked Area")
        .xlab("Month")
        .ylab("Volume")
        .theme_minimal()
        .save_with_size(&out("area"), W, H)?;
    Ok(())
}

/// Radial "rose" chart — bars in polar coordinates (`coord_polar`).
fn polar() -> Result<(), Box<dyn std::error::Error>> {
    let day = vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let value = vec![18.0, 23.0, 20.0, 28.0, 33.0, 45.0, 39.0];
    let df = df! { "day" => day, "value" => value }?;
    // Hide the angular axis text (it overlaps at the base of a rose chart);
    // the legend already maps day → colour.
    let mut theme = theme_minimal();
    theme.axis_text_x.visible = false;
    GGPlot::new(df)
        .aes(Aes::new().x("day").y("value").fill("day"))
        .geom_col()
        .coord_polar()
        .scale_fill_brewer(PaletteName::Set3)
        .title("Polar Bars (rose chart)")
        .theme(theme)
        .save_with_size(&out("polar"), W, H)?;
    Ok(())
}

/// Empirical cumulative distribution of a normal sample.
fn ecdf() -> Result<(), Box<dyn std::error::Error>> {
    let mut r = seeded(12);
    let x: Vec<f64> = (0..300).map(|_| round2(randn(&mut r) * 1.5)).collect();
    let df = df! { "x" => x }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x"))
        .geom_step()
        .stat(ggplot_rs::stat::ecdf::StatEcdf)
        .primary_color((49, 130, 189))
        .title("Empirical CDF (stat_ecdf)")
        .xlab("Value")
        .ylab("F(x)")
        .theme_minimal()
        .save_with_size(&out("ecdf"), W, H)?;
    Ok(())
}

/// Normal quantile-quantile plot with a reference line (`geom_qq`).
fn qq() -> Result<(), Box<dyn std::error::Error>> {
    // A heavy-tailed (Student-t-like) sample so the points bow away from the
    // reference line at both ends.
    let mut r = seeded(13);
    let y: Vec<f64> = (0..200)
        .map(|_| {
            let z = randn(&mut r);
            // Inflate the tails.
            round2(z * (1.0 + 0.35 * z * z))
        })
        .collect();
    let df = df! { "y" => y }?;
    GGPlot::new(df)
        .aes(Aes::new().y("y"))
        .geom_qq()
        .geom_qq_line()
        .primary_color((197, 90, 17))
        .title("Q-Q Plot (stat_qq)")
        .xlab("Theoretical")
        .ylab("Sample")
        .theme_bw()
        .save_with_size(&out("qq"), W, H)?;
    Ok(())
}

/// A simple-features choropleth (`geom_sf`) — provinces filled by a value.
#[cfg(feature = "sf")]
fn spatial() -> Result<(), Box<dyn std::error::Error>> {
    let geometry = vec![
        "POLYGON ((0 0, 3 0, 3 2, 1 2.5, 0 2, 0 0))",
        "POLYGON ((3 0, 6 0, 6 3, 3 2, 3 0))",
        "POLYGON ((0 2, 1 2.5, 3 2, 3 5, 0 5, 0 2))",
        "POLYGON ((3 2, 6 3, 6 5, 3 5, 3 2))",
        "POLYGON ((6 0, 9 1, 8 4, 6 3, 6 0))",
        "POLYGON ((6 3, 8 4, 9 6, 6 5, 6 3))",
        "POLYGON ((0 5, 3 5, 4 7, 1 8, 0 7, 0 5))",
        "POLYGON ((3 5, 6 5, 6 7, 4 7, 3 5))",
    ];
    let population = vec![4.2, 7.8, 3.1, 9.5, 5.4, 2.7, 6.6, 8.9];
    let df = df! { "geometry" => geometry, "population" => population }?;

    // Clean map look: drop the axis text (coordinates aren't meaningful here).
    let mut theme = theme_minimal();
    theme.axis_text_x.visible = false;
    theme.axis_text_y.visible = false;
    GGPlot::new(df)
        .aes(Aes::new().fill("population"))
        .geom_sf()
        .scale_fill_viridis_c()
        .title("Choropleth")
        .theme(theme)
        .save_with_size(&out("spatial"), W, H)?;
    Ok(())
}

/// An overflowing horizontal legend auto-wraps (B3-2, 0.22): too many
/// groups for one row at this width, so it grows to a second row instead
/// of running off the panel.
fn legend_wrap() -> Result<(), Box<dyn std::error::Error>> {
    let mut r = seeded(7);
    let n_groups = 14;
    let (mut x, mut y, mut group) = (Vec::new(), Vec::new(), Vec::new());
    for g in 0..n_groups {
        for i in 0..6 {
            x.push(i as f64);
            y.push(round2(
                (g as f64 * 0.3).sin() * 3.0 + i as f64 * 0.4 + randn(&mut r) * 0.3,
            ));
            group.push(format!("Series {:02}", g + 1));
        }
    }
    let df = df! { "x" => x, "y" => y, "group" => group }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("group"))
        .geom_line()
        .geom_point()
        .title("Auto-wrapped legend (14 series)")
        .xlab("x")
        .ylab("y")
        .theme_bw()
        .legend_position(LegendPosition::Bottom)
        .save_with_size(&out("legend_wrap"), W, H)?;
    Ok(())
}

/// `geom_boxplot(stat = "identity")` (B3-5, 0.22): the five-number summary
/// supplied directly instead of computed from raw observations, with
/// outliers that widen the y axis to fit them.
fn boxplot_identity() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "group" => vec!["A", "B", "C"],
        "lower" => vec![3.2, 5.0, 2.1],
        "middle" => vec![4.5, 6.4, 3.3],
        "upper" => vec![5.8, 7.9, 4.6],
        "ymin" => vec![1.9, 3.6, 0.8],
        "ymax" => vec![7.1, 9.5, 6.0],
        "outliers" => vec!["9.8, 10.4", "", "7.2"],
    }?;
    GGPlot::new(df)
        .aes(
            Aes::new()
                .x("group")
                .lower("lower")
                .middle("middle")
                .upper("upper")
                .ymin("ymin")
                .ymax("ymax")
                .outliers("outliers"),
        )
        .geom_boxplot_with(GeomBoxplot {
            fill: (255, 165, 79),
            ..Default::default()
        })
        .stat(StatIdentity)
        .tooltip("{group}: {lower}–{middle}–{upper}")
        .title("Boxplot, stat = \"identity\"")
        .xlab("Group")
        .ylab("Value")
        .theme_bw()
        .save_with_size(&out("boxplot_identity"), W, H)?;
    Ok(())
}

/// The same iris scatter rendered under every built-in theme.
fn themes() -> Result<(), Box<dyn std::error::Error>> {
    let (x, y, g) = iris(14);
    let df = df! { "x" => x, "y" => y, "g" => g }?;

    type ThemeFn = fn() -> Theme;
    let variants: [(&str, ThemeFn); 8] = [
        ("gray", theme_gray),
        ("bw", theme_bw),
        ("minimal", theme_minimal),
        ("classic", theme_classic),
        ("light", theme_light),
        ("dark", theme_dark),
        ("linedraw", theme_linedraw),
        ("void", theme_void),
    ];
    for (name, make) in variants {
        GGPlot::new(df.clone())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_point()
            .theme(make())
            .scale_color_brewer(PaletteName::Dark2)
            .title(&format!("theme_{name}"))
            .save_with_size(&out(&format!("theme_{name}")), TW, TH)?;
    }
    Ok(())
}

/// Daily OHLC prices as a candlestick chart (`geom_candlestick`).
fn candlestick() -> Result<(), Box<dyn std::error::Error>> {
    use ggplot_rs::stat::calendar::days_from_civil;
    let start = days_from_civil(2024, 3, 1);
    let (mut date, mut o, mut h, mut l, mut c) = (vec![], vec![], vec![], vec![], vec![]);
    let mut price: f64 = 100.0;
    for i in 0..40i64 {
        // Deterministic wiggle: drift + two incommensurate waves.
        let open = price;
        let close = open + 1.6 * (i as f64 * 0.9).sin() + 0.7 * (i as f64 * 2.3).cos() + 0.15;
        let spread = 0.6 + 0.5 * (i as f64 * 1.7).sin().abs();
        date.push(Value::DateTime((start + i) * 86_400));
        o.push(Value::Float(round2(open)));
        c.push(Value::Float(round2(close)));
        h.push(Value::Float(round2(open.max(close) + spread)));
        l.push(Value::Float(round2(open.min(close) - spread * 0.8)));
        price = close;
    }
    let data = vec![
        ("date".to_string(), date),
        ("open".to_string(), o),
        ("high".to_string(), h),
        ("low".to_string(), l),
        ("close".to_string(), c),
    ];
    GGPlot::new(data)
        .aes(
            Aes::new()
                .x("date")
                .open("open")
                .high("high")
                .low("low")
                .close("close"),
        )
        .geom_candlestick()
        .title("Candlestick")
        .subtitle("Daily OHLC, March–April 2024")
        .ylab("Price")
        .theme_minimal()
        .save_with_size(&out("candlestick"), W, H)?;
    Ok(())
}

/// Profit bridge from revenue to net income (`geom_waterfall`).
fn waterfall() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "step" => ["Revenue", "Services", "COGS", "Gross profit", "Opex", "R&D", "Other", "Tax"],
        "amount" => [420.0, 85.0, -190.0, 0.0, -120.0, -60.0, 18.0, -38.0],
    }?;
    GGPlot::new(df)
        .aes(Aes::new().x("step").y("amount"))
        .geom_waterfall_with(
            GeomWaterfall::default()
                .with_totals(["Gross profit"])
                .with_final_total("Net income"),
        )
        .title("Waterfall")
        .subtitle("From revenue to net income (EUR k)")
        .xlab("")
        .ylab("EUR k")
        .theme_minimal()
        .save_with_size(&out("waterfall"), W, H)?;
    Ok(())
}

/// Defect causes ranked with their cumulative share (`geom_pareto`).
fn pareto() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "cause" => ["Paint run", "Scratch", "Dent", "Misalignment", "Seal gap", "Weld spatter", "Other"],
        "count" => [142.0, 98.0, 61.0, 34.0, 22.0, 13.0, 9.0],
    }?;
    GGPlot::new(df)
        .aes(Aes::new().x("cause").y("count"))
        .geom_pareto_with(GeomPareto::default().with_threshold(0.8))
        .title("Pareto")
        .subtitle("Paint-shop defects by cause, dashed line at 80 %")
        .xlab("")
        .ylab("Defects")
        .theme_minimal()
        .save_with_size(&out("pareto"), W, H)?;
    Ok(())
}

/// Energy flows from sources through conversion to end use (`geom_sankey`).
fn sankey() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "from" => ["Coal", "Gas", "Gas", "Nuclear", "Wind", "Solar", "Oil",
                   "Electricity", "Electricity", "Electricity", "Heat", "Heat", "Oil", "Gas"],
        "to" => ["Electricity", "Electricity", "Heat", "Electricity", "Electricity", "Electricity", "Transport",
                 "Homes", "Industry", "Losses", "Homes", "Industry", "Industry", "Homes"],
        "twh" => [62.0, 48.0, 30.0, 35.0, 41.0, 18.0, 70.0,
                  70.0, 64.0, 70.0, 18.0, 12.0, 15.0, 26.0],
    }?;
    GGPlot::new(df)
        .aes(Aes::new().from("from").to("to").weight("twh"))
        .geom_sankey_with(GeomSankey {
            label_values: true,
            ..Default::default()
        })
        .scale_fill_brewer(PaletteName::Set3)
        .title("Sankey")
        .subtitle("Energy flows, TWh (illustrative)")
        .theme_minimal()
        .save_with_size(&out("sankey"), W, H)?;
    Ok(())
}

/// Trade flows between regions as a dependency wheel (`geom_chord`).
fn chord() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "from" => ["Europe", "Europe", "Europe", "Asia", "Asia", "Asia", "Americas", "Americas", "Africa", "Oceania", "Asia", "Americas"],
        "to" => ["Asia", "Americas", "Africa", "Europe", "Americas", "Oceania", "Europe", "Asia", "Europe", "Asia", "Africa", "Africa"],
        "bn" => [310.0, 220.0, 90.0, 480.0, 390.0, 120.0, 260.0, 280.0, 110.0, 140.0, 95.0, 40.0],
    }?;
    GGPlot::new(df)
        .aes(Aes::new().from("from").to("to").weight("bn"))
        .geom_chord()
        .scale_fill_brewer(PaletteName::Dark2)
        .title("Chord diagram")
        .subtitle("Trade between regions, USD bn (illustrative)")
        .theme_minimal()
        .save_with_size(&out("chord"), W, H)?;
    Ok(())
}

/// Co-appearances between characters as an arc diagram (`geom_arcdiagram`).
fn arcdiagram() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "from" => ["Ana", "Ana", "Ana", "Ben", "Ben", "Cleo", "Cleo", "Dev", "Eli", "Eli", "Fay", "Gus", "Ana", "Hal"],
        "to" => ["Ben", "Cleo", "Gus", "Cleo", "Dev", "Dev", "Fay", "Eli", "Fay", "Hal", "Gus", "Hal", "Hal", "Ben"],
        "scenes" => [9.0, 6.0, 2.0, 7.0, 3.0, 5.0, 2.0, 8.0, 4.0, 3.0, 6.0, 5.0, 1.0, 2.0],
    }?;
    GGPlot::new(df)
        .aes(Aes::new().from("from").to("to").weight("scenes"))
        .geom_arcdiagram()
        .scale_fill_brewer(PaletteName::Dark2)
        .title("Arc diagram")
        .subtitle("Shared scenes between characters")
        .theme_minimal()
        .save_with_size(&out("arcdiagram"), W, H)?;
    Ok(())
}

/// GDP by country nested in regions (`geom_treemap`).
fn treemap() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "country" => ["USA", "Canada", "Mexico", "Brazil", "China", "Japan", "India", "Korea", "Indonesia",
                      "Germany", "UK", "France", "Italy", "Spain", "Netherlands", "Australia", "Nigeria", "Egypt"],
        "region" => ["Americas", "Americas", "Americas", "Americas", "Asia", "Asia", "Asia", "Asia", "Asia",
                     "Europe", "Europe", "Europe", "Europe", "Europe", "Europe", "Oceania", "Africa", "Africa"],
        "gdp" => [27.4, 2.1, 1.8, 2.2, 17.8, 4.2, 3.6, 1.7, 1.4, 4.5, 3.3, 3.0, 2.3, 1.6, 1.1, 1.7, 0.4, 0.4],
    }?;
    GGPlot::new(df)
        .aes(Aes::new().weight("gdp").subgroup("region").label("country"))
        .geom_treemap()
        .scale_fill_brewer(PaletteName::Set2)
        .title("Treemap")
        .subtitle("GDP by country and region, USD tn (2023, rounded)")
        .theme_minimal()
        .legend_position(LegendPosition::None)
        .save_with_size(&out("treemap"), W, H)?;
    Ok(())
}

/// A three-level hierarchy as a sunburst (`geom_sunburst`).
fn sunburst() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "dept" => ["Engineering", "Engineering", "Engineering", "Engineering", "Engineering",
                   "Sales", "Sales", "Sales", "Operations", "Operations", "Operations", "People"],
        "team" => ["Platform", "Platform", "Product", "Product", "Data",
                   "EMEA", "Americas", "APAC", "Logistics", "Facilities", "Logistics", "Recruiting"],
        "role" => ["Backend", "SRE", "Frontend", "Mobile", "Analytics",
                   "AE", "AE", "AE", "Planning", "Office", "Fleet", "Talent"],
        "fte" => [42.0, 14.0, 30.0, 12.0, 18.0, 25.0, 31.0, 12.0, 16.0, 8.0, 10.0, 9.0],
    }?;
    GGPlot::new(df)
        .aes(Aes::new().weight("fte"))
        .geom_sunburst(["dept", "team", "role"])
        .scale_fill_brewer(PaletteName::Dark2)
        .title("Sunburst")
        .subtitle("Headcount by department, team and role")
        .theme_void()
        .save_with_size(&out("sunburst"), W, H)?;
    Ok(())
}

/// A sales funnel with conversion from the first stage (`geom_funnel`).
fn funnel() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "stage" => ["Website visits", "Product views", "Added to cart", "Checkout", "Purchased"],
        "n" => [12500.0, 7300.0, 2900.0, 1600.0, 1150.0],
    }?;
    GGPlot::new(df)
        .aes(Aes::new().x("stage").y("n"))
        .geom_funnel()
        .scale_fill_brewer(PaletteName::Set2)
        .title("Funnel")
        .subtitle("Sessions per stage, % of visits")
        .theme_void()
        .save_with_size(&out("funnel"), W, H)?;
    Ok(())
}

/// Listening share by genre over a year as a streamgraph (`geom_stream`).
fn stream() -> Result<(), Box<dyn std::error::Error>> {
    let genres = ["Pop", "Rock", "Hip-hop", "Jazz", "Electronic", "Classical"];
    let (mut week, mut genre, mut plays) = (vec![], vec![], vec![]);
    for w in 0..52 {
        for (k, g) in genres.iter().enumerate() {
            let t = w as f64 / 52.0 * std::f64::consts::TAU;
            let v = 8.0 - k as f64
                + 4.0 * (t * (1.0 + 0.3 * k as f64) + k as f64).sin()
                + 2.5 * (2.0 * t + 1.7 * k as f64).cos();
            week.push(w as f64);
            genre.push(*g);
            plays.push(v.max(0.4));
        }
    }
    let df = df! { "week" => week, "genre" => genre, "plays" => plays }?;
    GGPlot::new(df)
        .aes(Aes::new().x("week").y("plays").fill("genre"))
        .geom_stream()
        .scale_fill_brewer(PaletteName::Set2)
        .title("Streamgraph")
        .subtitle("Weekly plays by genre (synthetic)")
        .xlab("Week")
        .ylab("")
        .theme_minimal()
        .save_with_size(&out("stream"), W, H)?;
    Ok(())
}

/// Monthly temperatures as spline line + range band (`Interpolate::MonotoneCubic`).
fn spline() -> Result<(), Box<dyn std::error::Error>> {
    let month: Vec<f64> = (1..=12).map(f64::from).collect();
    let mean = [
        1.2, 2.1, 5.8, 9.9, 14.2, 17.4, 19.5, 19.1, 15.2, 10.4, 5.5, 2.3,
    ];
    let lo: Vec<f64> = mean.iter().map(|m| m - 4.5).collect();
    let hi: Vec<f64> = mean.iter().map(|m| m + 5.0).collect();
    let df = df! { "month" => month, "mean" => mean.to_vec(), "lo" => lo, "hi" => hi }?;
    GGPlot::new(df)
        .aes(Aes::new().x("month").y("mean"))
        .geom_ribbon_with(GeomRibbon {
            interpolate: Interpolate::MonotoneCubic,
            alpha: 0.25,
            ..Default::default()
        })
        .layer_aes(Aes::new().x("month").ymin("lo").ymax("hi"))
        .geom_line_with(GeomLine {
            interpolate: Interpolate::MonotoneCubic,
            color: (31, 119, 180),
            width: 2.0,
            ..Default::default()
        })
        .geom_point()
        .title("Spline")
        .subtitle("Mean daily temperature with typical range, °C (monotone cubic)")
        .xlab("Month")
        .ylab("°C")
        .theme_minimal()
        .save_with_size(&out("spline"), W, H)?;
    Ok(())
}

/// Life expectancy gains 1990 → 2022 as a dumbbell chart (`geom_dumbbell`).
fn dumbbell() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "country" => ["Ethiopia", "Rwanda", "India", "Bangladesh", "China", "Brazil", "Germany", "Japan"],
        "y1990" => [47.0, 33.4, 58.6, 58.2, 69.1, 66.3, 75.4, 78.9],
        "y2022" => [65.0, 66.1, 67.7, 73.7, 78.6, 73.4, 80.7, 84.0],
    }?;
    GGPlot::new(df)
        .aes(Aes::new().x("country").ymin("y1990").ymax("y2022"))
        .geom_dumbbell()
        .coord_flip()
        .title("Dumbbell")
        .subtitle("Life expectancy at birth, 1990 (grey) → 2022 (blue)")
        .xlab("")
        .ylab("Years")
        .theme_minimal()
        .save_with_size(&out("dumbbell"), W, H)?;
    Ok(())
}

/// Two products compared on five attributes (`coord_radar`).
fn radar() -> Result<(), Box<dyn std::error::Error>> {
    let axes = ["Speed", "Power", "Range", "Value", "Comfort", "Safety"];
    let series = [
        ("Model A", [8.0, 6.0, 9.0, 5.0, 7.0, 8.0]),
        ("Model B", [5.0, 9.0, 6.0, 8.0, 6.0, 7.0]),
    ];
    let (mut ax, mut se, mut v) = (vec![], vec![], vec![]);
    for (name, vals) in series {
        for (a, x) in axes.iter().zip(vals) {
            ax.push(*a);
            se.push(name);
            v.push(x);
        }
    }
    let df = df! { "axis" => ax, "model" => se, "score" => v }?;
    GGPlot::new(df)
        .aes(Aes::new().x("axis").y("score").color("model").fill("model"))
        .geom_polygon_with(GeomPolygon {
            alpha: 0.18,
            line_width: 2.0,
            ..Default::default()
        })
        .geom_point()
        .scale_color_brewer(PaletteName::Set1)
        .scale_fill_brewer(PaletteName::Set1)
        .coord_radar()
        .title("Radar")
        .theme_minimal()
        .save_with_size(&out("radar"), W, H)?;
    Ok(())
}

/// A half-donut gauge: `geom_rect` zone bands + a needle under
/// `coord_polar` with a 180° span.
fn gauge() -> Result<(), Box<dyn std::error::Error>> {
    use std::f64::consts::PI;
    let value = 72.0;
    let bands = df! {
        "x0" => [0.0, 60.0, 85.0],
        "x1" => [60.0, 85.0, 100.0],
        "y0" => [0.62, 0.62, 0.62],
        "y1" => [1.0, 1.0, 1.0],
        "zone" => ["ok", "warn", "critical"],
    }?;
    let needle = df! { "v" => [value], "r0" => [0.0], "r1" => [0.86] }?;
    GGPlot::new(bands)
        .geom_rect_with(GeomRect {
            line_width: 0.0,
            alpha: 1.0,
            ..Default::default()
        })
        .layer_aes(
            Aes::new()
                .xmin("x0")
                .xmax("x1")
                .ymin("y0")
                .ymax("y1")
                .fill("zone"),
        )
        .geom_segment_with(GeomSegment {
            color: (40, 44, 52),
            width: 4.0,
            alpha: 1.0,
            ..Default::default()
        })
        .layer_data(needle)
        .layer_aes(Aes::new().x("v").xend("v").y("r0").yend("r1"))
        .scale_fill_manual(vec![
            ("ok", RGBAColor::new(0x2f, 0x9e, 0x44)),
            ("warn", RGBAColor::new(0xf5, 0x9f, 0x00)),
            ("critical", RGBAColor::new(0xe0, 0x31, 0x31)),
        ])
        .scale_x_continuous(
            ScaleContinuous::new()
                .with_limits(0.0, 100.0)
                .with_expand(0.0, 0.0),
        )
        .scale_y_continuous(
            ScaleContinuous::new()
                .with_limits(0.0, 1.0)
                .with_expand(0.0, 0.0),
        )
        .coord_polar_with(CoordPolar::new().with_span(-PI / 2.0, PI / 2.0))
        .annotate(Annotation::Text {
            label: format!("{value}%"),
            x: 50.0,
            y: 0.3,
            size: 28.0,
            color: (31, 41, 55),
        })
        .title("Gauge")
        .subtitle("CPU utilisation")
        .theme_void()
        .legend_position(LegendPosition::None)
        .save_with_size(&out("gauge"), W, H)?;
    Ok(())
}

/// A year of daily activity as a calendar heatmap (`geom_calendar`).
fn calendar() -> Result<(), Box<dyn std::error::Error>> {
    use ggplot_rs::stat::calendar::days_from_civil;
    let start = days_from_civil(2024, 1, 1);
    let (mut date, mut n) = (vec![], vec![]);
    for d in 0..366i64 {
        let weekday = (d + 1) % 7; // 2024-01-01 is a Monday
        let weekend = weekday == 6 || weekday == 0;
        let season = 4.0 + 3.0 * (d as f64 / 366.0 * 2.0 * std::f64::consts::PI).sin();
        let noise = ((d * 7919) % 13) as f64 / 3.0;
        date.push(Value::DateTime((start + d) * 86_400));
        n.push(Value::Float(
            if weekend { noise * 0.4 } else { season + noise }.round(),
        ));
    }
    let data = vec![("date".to_string(), date), ("commits".to_string(), n)];
    GGPlot::new(data)
        .aes(Aes::new().x("date").fill("commits"))
        .geom_calendar()
        .scale_fill_gradient(
            RGBAColor::new(0xeb, 0xed, 0xf0),
            RGBAColor::new(0x21, 0x6e, 0x39),
        )
        .title("Calendar heatmap")
        .subtitle("Daily commits, 2024")
        .xlab("")
        .ylab("")
        .theme_minimal()
        .save_with_size(&out("calendar"), W + 260, 300)?;
    Ok(())
}
