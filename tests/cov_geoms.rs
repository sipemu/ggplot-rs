//! Rendering-coverage tests for every geom in `src/geom/`.
//!
//! A geom's `draw()` only executes when a plot is actually rendered to SVG/PNG,
//! so these tests build a minimal valid plot for each geom and force a render,
//! asserting only that output is produced (`Ok` + non-empty). They intentionally
//! do not assert pixel/text content.

use ggplot_rs::prelude::*;

// ─── Data helpers ────────────────────────────────────────────

/// A numeric column from `f64`s.
fn fcol(name: &str, v: &[f64]) -> (String, Vec<Value>) {
    (
        name.to_string(),
        v.iter().map(|f| Value::Float(*f)).collect(),
    )
}

/// A discrete (string) column.
fn scol(name: &str, v: &[&str]) -> (String, Vec<Value>) {
    (
        name.to_string(),
        v.iter().map(|s| Value::Str((*s).to_string())).collect(),
    )
}

/// Basic x/y scatter data plus a discrete grouping column `g`.
fn xy() -> Vec<(String, Vec<Value>)> {
    vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0, 5.0]),
        fcol("y", &[2.0, 4.0, 3.0, 5.0, 4.0]),
        scol("g", &["a", "b", "a", "b", "a"]),
        fcol("s", &[1.0, 2.0, 3.0, 4.0, 5.0]),
    ]
}

/// A single numeric column suitable for 1-D stats (histogram, density, ...).
fn dist(name: &str) -> Vec<(String, Vec<Value>)> {
    let v: Vec<f64> = (0..60)
        .map(|i| (i as f64 * 0.37).sin() * 3.0 + 5.0)
        .collect();
    vec![fcol(name, &v)]
}

/// A discrete group column + numeric y (boxplot / violin).
fn grouped() -> Vec<(String, Vec<Value>)> {
    let mut g = Vec::new();
    let mut y = Vec::new();
    for grp in ["A", "B", "C"] {
        let base = match grp {
            "A" => 10.0,
            "B" => 20.0,
            _ => 15.0,
        };
        for i in 0..20 {
            g.push(Value::Str(grp.to_string()));
            y.push(Value::Float(base + (i as f64 * 0.5) - 5.0));
        }
    }
    vec![("g".to_string(), g), ("y".to_string(), y)]
}

/// A gridded x/y/z surface (contour).
fn grid() -> Vec<(String, Vec<Value>)> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut zs = Vec::new();
    for ix in 0..10 {
        for iy in 0..10 {
            let x = ix as f64;
            let y = iy as f64;
            xs.push(Value::Float(x));
            ys.push(Value::Float(y));
            zs.push(Value::Float(x * x + y * y));
        }
    }
    vec![
        ("x".to_string(), xs),
        ("y".to_string(), ys),
        ("z".to_string(), zs),
    ]
}

/// A 2-D point cloud (bin2d / hex / density2d).
fn cloud() -> Vec<(String, Vec<Value>)> {
    let x: Vec<f64> = (0..120)
        .map(|i| (i as f64 * 0.1).sin() * 3.0 + 5.0)
        .collect();
    let y: Vec<f64> = (0..120)
        .map(|i| (i as f64 * 0.13).cos() * 2.0 + 4.0)
        .collect();
    vec![fcol("x", &x), fcol("y", &y)]
}

/// Range data: x, y, ymin, ymax (+ discrete group).
fn ranges() -> Vec<(String, Vec<Value>)> {
    vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0]),
        fcol("y", &[2.0, 3.0, 2.5, 4.0]),
        fcol("ymin", &[1.0, 2.0, 1.5, 3.0]),
        fcol("ymax", &[3.0, 4.0, 3.5, 5.0]),
        scol("g", &["a", "b", "a", "b"]),
    ]
}

// ─── Assertion helpers ───────────────────────────────────────

/// Render to SVG and assert non-empty success.
fn ok_svg(plot: GGPlot, what: &str) {
    let r = plot.render_svg();
    assert!(r.is_ok(), "{what}: render_svg errored: {:?}", r.err());
    assert!(!r.unwrap().is_empty(), "{what}: empty SVG");
}

/// Render to a small PNG and assert non-empty success.
fn ok_png(plot: GGPlot, what: &str) {
    let r = plot.render_png_with_size(200, 150);
    assert!(r.is_ok(), "{what}: render_png errored: {:?}", r.err());
    assert!(!r.unwrap().is_empty(), "{what}: empty PNG");
}

// ─── Core x/y geoms ──────────────────────────────────────────

#[test]
fn point_variants() {
    // default
    ok_svg(
        GGPlot::new(xy()).aes(Aes::new().x("x").y("y")).geom_point(),
        "point",
    );
    // color + size + alpha mapped
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y").color("g").size("s").alpha("s"))
            .geom_point(),
        "point mapped",
    );
    // styled struct
    ok_png(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_point_with(GeomPoint {
                size: 6.0,
                color: (200, 30, 30),
                alpha: 0.5,
            }),
        "point_with",
    );
}

#[test]
fn line_and_path_family() {
    ok_svg(
        GGPlot::new(xy()).aes(Aes::new().x("x").y("y")).geom_line(),
        "line",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_line_with(GeomLine {
                color: (10, 100, 200),
                width: 2.0,
                alpha: 0.8,
            }),
        "line_with",
    );
    ok_svg(
        GGPlot::new(xy()).aes(Aes::new().x("x").y("y")).geom_path(),
        "path",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_path_with(GeomPath {
                color: (20, 20, 20),
                width: 1.5,
                alpha: 0.9,
            }),
        "path_with",
    );
    ok_svg(
        GGPlot::new(xy()).aes(Aes::new().x("x").y("y")).geom_step(),
        "step",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_step_with(GeomStep {
                color: (0, 0, 0),
                width: 1.0,
                alpha: 1.0,
                direction: StepDirection::Vh,
            }),
        "step_with",
    );
}

#[test]
fn bar_col_area_positions() {
    // bar: default stat=count over discrete x
    ok_svg(GGPlot::new(xy()).aes(Aes::new().x("g")).geom_bar(), "bar");
    // bar stacked by fill
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("g").fill("g"))
            .geom_bar()
            .position(PositionStack),
        "bar stack",
    );
    // bar filled
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("g").fill("g"))
            .geom_bar()
            .position(PositionFill),
        "bar fill",
    );
    // bar dodged
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("g").fill("g"))
            .geom_bar()
            .position(PositionDodge),
        "bar dodge",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("g"))
            .geom_bar_with(GeomBar {
                fill: (100, 150, 200),
                color: (0, 0, 0),
                alpha: 0.7,
                width: 0.6,
            }),
        "bar_with",
    );
    // col: identity x/y
    ok_svg(
        GGPlot::new(xy()).aes(Aes::new().x("x").y("y")).geom_col(),
        "col",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y").fill("g"))
            .geom_col_with(GeomCol {
                fill: (50, 200, 100),
                color: (10, 10, 10),
                alpha: 0.9,
                width: 0.8,
            }),
        "col_with",
    );
    // area
    ok_svg(
        GGPlot::new(xy()).aes(Aes::new().x("x").y("y")).geom_area(),
        "area",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y").fill("g"))
            .geom_area_with(GeomArea {
                fill: (200, 100, 50),
                color: (0, 0, 0),
                alpha: 0.5,
                line_width: 1.0,
            })
            .position(PositionStack),
        "area_with stack",
    );
}

// ─── 1-D distribution geoms ──────────────────────────────────

#[test]
fn distribution_geoms() {
    ok_svg(
        GGPlot::new(dist("x"))
            .aes(Aes::new().x("x"))
            .geom_histogram(),
        "histogram",
    );
    ok_png(
        GGPlot::new(dist("x"))
            .aes(Aes::new().x("x"))
            .geom_histogram_with(GeomHistogram {
                bins: 8,
                binwidth: None,
                fill: (80, 120, 200),
                color: (255, 255, 255),
                alpha: 0.8,
            }),
        "histogram_with",
    );
    ok_svg(
        GGPlot::new(dist("x")).aes(Aes::new().x("x")).geom_density(),
        "density",
    );
    ok_svg(
        GGPlot::new(dist("x"))
            .aes(Aes::new().x("x"))
            .geom_density_with(GeomDensity {
                fill: (100, 200, 100),
                color: (0, 80, 0),
                alpha: 0.4,
                line_width: 1.5,
            }),
        "density_with",
    );
    ok_svg(
        GGPlot::new(dist("x"))
            .aes(Aes::new().x("x"))
            .geom_freqpoly(),
        "freqpoly",
    );
    ok_svg(
        GGPlot::new(dist("x"))
            .aes(Aes::new().x("x"))
            .geom_freqpoly_with(GeomFreqpoly {
                color: (200, 0, 0),
                width: 1.2,
                alpha: 1.0,
            }),
        "freqpoly_with",
    );
    ok_svg(
        GGPlot::new(dist("x")).aes(Aes::new().x("x")).geom_dotplot(),
        "dotplot",
    );
    ok_svg(
        GGPlot::new(dist("x"))
            .aes(Aes::new().x("x"))
            .geom_dotplot_with(GeomDotplot {
                size: 4.0,
                color: (0, 0, 0),
                fill: (150, 150, 250),
                alpha: 0.9,
            }),
        "dotplot_with",
    );
    // rug on x/y scatter
    ok_svg(
        GGPlot::new(xy()).aes(Aes::new().x("x").y("y")).geom_rug(),
        "rug",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_rug_with(GeomRug {
                color: (50, 50, 50),
                alpha: 0.7,
                length: 0.04,
                sides: "bl".to_string(),
            }),
        "rug_with",
    );
    // jitter
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_jitter(),
        "jitter",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_jitter_with(GeomJitter {
                size: 3.0,
                color: (0, 0, 0),
                alpha: 0.6,
                width: 0.3,
                height: 0.3,
            }),
        "jitter_with",
    );
}

// ─── Box/violin over discrete groups ─────────────────────────

#[test]
fn boxplot_and_violin() {
    ok_svg(
        GGPlot::new(grouped())
            .aes(Aes::new().x("g").y("y"))
            .geom_boxplot(),
        "boxplot",
    );
    ok_svg(
        GGPlot::new(grouped())
            .aes(Aes::new().x("g").y("y").fill("g"))
            .geom_boxplot_with(GeomBoxplot {
                fill: (200, 200, 100),
                color: (0, 0, 0),
                width: 0.5,
                alpha: 0.8,
            }),
        "boxplot_with",
    );
    ok_svg(
        GGPlot::new(grouped())
            .aes(Aes::new().x("g").y("y"))
            .geom_violin(),
        "violin",
    );
    ok_svg(
        GGPlot::new(grouped())
            .aes(Aes::new().x("g").y("y").fill("g"))
            .geom_violin_with(GeomViolin {
                fill: (120, 180, 220),
                color: (0, 0, 0),
                alpha: 0.7,
                line_width: 1.0,
            }),
        "violin_with",
    );
}

// ─── Smooth / trend ──────────────────────────────────────────

#[test]
fn smooth_geoms() {
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .geom_smooth(),
        "smooth",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_smooth_with(GeomSmooth {
                color: (200, 0, 0),
                fill: (200, 150, 150),
                line_width: 1.5,
                alpha: 0.3,
                se: true,
                n_points: 40,
                method: SmoothMethod::Lm,
            }),
        "smooth_with",
    );
}

// ─── Range / interval geoms ──────────────────────────────────

#[test]
fn range_geoms() {
    ok_svg(
        GGPlot::new(ranges())
            .aes(Aes::new().x("x").y("y").ymin("ymin").ymax("ymax"))
            .geom_errorbar(),
        "errorbar",
    );
    ok_svg(
        GGPlot::new(ranges())
            .aes(Aes::new().x("x").ymin("ymin").ymax("ymax"))
            .geom_errorbar_with(GeomErrorbar {
                color: (0, 0, 200),
                width: 1.0,
                cap_width: 0.3,
                alpha: 1.0,
            }),
        "errorbar_with",
    );
    ok_svg(
        GGPlot::new(ranges())
            .aes(Aes::new().x("x").ymin("ymin").ymax("ymax"))
            .geom_linerange(),
        "linerange",
    );
    ok_svg(
        GGPlot::new(ranges())
            .aes(Aes::new().x("x").ymin("ymin").ymax("ymax"))
            .geom_linerange_with(GeomLinerange {
                color: (100, 0, 100),
                width: 1.5,
                alpha: 0.9,
            }),
        "linerange_with",
    );
    ok_svg(
        GGPlot::new(ranges())
            .aes(Aes::new().x("x").y("y").ymin("ymin").ymax("ymax"))
            .geom_pointrange(),
        "pointrange",
    );
    ok_svg(
        GGPlot::new(ranges())
            .aes(
                Aes::new()
                    .x("x")
                    .y("y")
                    .ymin("ymin")
                    .ymax("ymax")
                    .color("g"),
            )
            .geom_pointrange_with(GeomPointrange {
                color: (0, 0, 0),
                width: 1.0,
                size: 3.0,
                alpha: 1.0,
            }),
        "pointrange_with",
    );
    ok_svg(
        GGPlot::new(ranges())
            .aes(Aes::new().x("x").y("y").ymin("ymin").ymax("ymax"))
            .geom_crossbar(),
        "crossbar",
    );
    ok_svg(
        GGPlot::new(ranges())
            .aes(Aes::new().x("x").y("y").ymin("ymin").ymax("ymax").fill("g"))
            .geom_crossbar_with(GeomCrossbar {
                fill: (220, 220, 180),
                color: (0, 0, 0),
                alpha: 0.8,
                bar_width: 0.5,
                line_width: 1.0,
            }),
        "crossbar_with",
    );
    // ribbon needs x, ymin, ymax
    ok_svg(
        GGPlot::new(ranges())
            .aes(Aes::new().x("x").ymin("ymin").ymax("ymax"))
            .geom_ribbon(),
        "ribbon",
    );
    ok_svg(
        GGPlot::new(ranges())
            .aes(Aes::new().x("x").ymin("ymin").ymax("ymax").fill("g"))
            .geom_ribbon_with(GeomRibbon {
                fill: (150, 200, 250),
                alpha: 0.4,
            }),
        "ribbon_with",
    );
}

// ─── Segment / spoke / curve ─────────────────────────────────

#[test]
fn segment_family() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0]),
        fcol("y", &[1.0, 2.0, 3.0]),
        fcol("xend", &[2.0, 3.0, 4.0]),
        fcol("yend", &[2.0, 1.0, 4.0]),
        scol("g", &["a", "b", "a"]),
    ];
    ok_svg(
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
            .geom_segment(),
        "segment",
    );
    ok_svg(
        GGPlot::new(data.clone())
            .aes(
                Aes::new()
                    .x("x")
                    .y("y")
                    .xend("xend")
                    .yend("yend")
                    .color("g"),
            )
            .geom_segment_with(GeomSegment {
                color: (0, 0, 0),
                width: 1.5,
                alpha: 0.9,
            }),
        "segment_with",
    );
    ok_svg(
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
            .geom_curve(),
        "curve",
    );
    ok_svg(
        GGPlot::new(data)
            .aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
            .geom_curve_with(GeomCurve {
                color: (120, 30, 30),
                width: 1.0,
                alpha: 1.0,
                curvature: 0.5,
                ncp: 8,
            }),
        "curve_with",
    );
    let spoke = vec![
        fcol("x", &[1.0, 2.0, 3.0]),
        fcol("y", &[1.0, 2.0, 3.0]),
        fcol("angle", &[0.0, 1.5, 3.0]),
        fcol("radius", &[0.5, 0.5, 0.5]),
    ];
    ok_svg(
        GGPlot::new(spoke.clone())
            .aes(Aes::new().x("x").y("y").angle("angle").radius("radius"))
            .geom_spoke(),
        "spoke",
    );
    ok_svg(
        GGPlot::new(spoke)
            .aes(Aes::new().x("x").y("y").angle("angle").radius("radius"))
            .geom_spoke_with(GeomSpoke {
                color: (0, 100, 0),
                width: 1.2,
                alpha: 0.8,
            }),
        "spoke_with",
    );
}

// ─── Rect / tile / polygon ───────────────────────────────────

#[test]
fn rect_tile_polygon() {
    let rects = vec![
        fcol("xmin", &[1.0, 3.0]),
        fcol("xmax", &[2.0, 5.0]),
        fcol("ymin", &[1.0, 2.0]),
        fcol("ymax", &[3.0, 4.0]),
        scol("g", &["a", "b"]),
    ];
    ok_svg(
        GGPlot::new(rects.clone())
            .aes(
                Aes::new()
                    .xmin("xmin")
                    .xmax("xmax")
                    .ymin("ymin")
                    .ymax("ymax"),
            )
            .geom_rect(),
        "rect",
    );
    ok_svg(
        GGPlot::new(rects)
            .aes(
                Aes::new()
                    .xmin("xmin")
                    .xmax("xmax")
                    .ymin("ymin")
                    .ymax("ymax")
                    .fill("g"),
            )
            .geom_rect_with(GeomRect {
                fill: (200, 180, 100),
                color: (0, 0, 0),
                alpha: 0.6,
                line_width: 1.0,
            }),
        "rect_with",
    );
    // tile
    let tiles = vec![
        fcol("x", &[1.0, 2.0, 1.0, 2.0]),
        fcol("y", &[1.0, 1.0, 2.0, 2.0]),
        fcol("z", &[0.1, 0.5, 0.9, 0.3]),
    ];
    ok_svg(
        GGPlot::new(tiles.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_tile(),
        "tile",
    );
    ok_svg(
        GGPlot::new(tiles)
            .aes(Aes::new().x("x").y("y").fill("z"))
            .geom_tile_with(GeomTile {
                fill: (100, 100, 200),
                color: (255, 255, 255),
                alpha: 0.9,
                width: 1.0,
                height: 1.0,
                line_width: 0.5,
            }),
        "tile_with",
    );
    // polygon (a triangle); geom_polygon requires a group aesthetic
    let poly = vec![
        fcol("x", &[0.0, 1.0, 0.5]),
        fcol("y", &[0.0, 0.0, 1.0]),
        scol("g", &["a", "a", "a"]),
    ];
    ok_svg(
        GGPlot::new(poly.clone())
            .aes(Aes::new().x("x").y("y").group("g"))
            .geom_polygon(),
        "polygon",
    );
    ok_svg(
        GGPlot::new(poly)
            .aes(Aes::new().x("x").y("y").group("g"))
            .geom_polygon_with(GeomPolygon {
                fill: (180, 120, 200),
                color: (0, 0, 0),
                alpha: 0.7,
                line_width: 1.0,
            }),
        "polygon_with",
    );
}

// ─── 2-D binning / density / count / contour ─────────────────

#[test]
fn twod_geoms() {
    ok_svg(
        GGPlot::new(cloud())
            .aes(Aes::new().x("x").y("y"))
            .geom_bin2d(),
        "bin2d",
    );
    ok_svg(
        GGPlot::new(cloud())
            .aes(Aes::new().x("x").y("y"))
            .geom_bin2d_with(GeomBin2d {
                color: (255, 255, 255),
                alpha: 0.9,
                line_width: 0.5,
            }),
        "bin2d_with",
    );
    ok_svg(
        GGPlot::new(cloud())
            .aes(Aes::new().x("x").y("y"))
            .geom_hex(),
        "hex",
    );
    ok_svg(
        GGPlot::new(cloud())
            .aes(Aes::new().x("x").y("y"))
            .geom_hex_with(GeomHex {
                color: (255, 255, 255),
                alpha: 0.9,
                line_width: 0.5,
            }),
        "hex_with",
    );
    ok_png(
        GGPlot::new(cloud())
            .aes(Aes::new().x("x").y("y"))
            .geom_density2d(),
        "density2d",
    );
    ok_svg(
        GGPlot::new(cloud())
            .aes(Aes::new().x("x").y("y"))
            .geom_density2d_with(GeomDensity2d {
                color: (30, 30, 160),
                alpha: 0.9,
                width: 1.0,
                n_grid: 20,
                n_levels: 5,
            }),
        "density2d_with",
    );
    // count: overlapping integer coords
    let cnt = vec![
        fcol("x", &[1.0, 1.0, 2.0, 2.0, 2.0, 3.0]),
        fcol("y", &[1.0, 1.0, 2.0, 2.0, 2.0, 3.0]),
    ];
    ok_svg(
        GGPlot::new(cnt.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_count(),
        "count",
    );
    ok_svg(
        GGPlot::new(cnt)
            .aes(Aes::new().x("x").y("y"))
            .geom_count_with(GeomCount {
                color: (200, 50, 50),
                alpha: 0.8,
                min_size: 2.0,
                max_size: 8.0,
            }),
        "count_with",
    );
    ok_svg(
        GGPlot::new(grid())
            .aes(Aes::new().x("x").y("y"))
            .geom_contour(),
        "contour",
    );
    ok_svg(
        GGPlot::new(grid())
            .aes(Aes::new().x("x").y("y"))
            .geom_contour_with(GeomContour {
                color: (0, 0, 0),
                alpha: 1.0,
                width: 1.0,
                bins: 6,
                n_levels: 6,
            }),
        "contour_with",
    );
}

// ─── QQ ──────────────────────────────────────────────────────

#[test]
fn qq_geoms() {
    let sample: Vec<f64> = (0..80).map(|i| ((i as f64) - 40.0) * 0.1).collect();
    let data = vec![fcol("sample", &sample)];
    ok_svg(
        GGPlot::new(data.clone())
            .aes(Aes::new().y("sample"))
            .geom_qq()
            .geom_qq_line(),
        "qq + qq_line",
    );
    ok_svg(
        GGPlot::new(data.clone())
            .aes(Aes::new().y("sample"))
            .geom_qq_with(GeomQQ {
                size: 3.0,
                color: (10, 10, 120),
                alpha: 0.8,
            }),
        "qq_with",
    );
    ok_svg(
        GGPlot::new(data)
            .aes(Aes::new().y("sample"))
            .geom_qq_line_with(GeomQQLine {
                color: (200, 0, 0),
                width: 1.5,
                alpha: 1.0,
            }),
        "qq_line_with",
    );
}

// ─── Text / label ────────────────────────────────────────────

#[test]
fn text_and_label() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0]),
        fcol("y", &[1.0, 2.0, 3.0]),
        scol("lab", &["p1", "p2", "p3"]),
    ];
    ok_svg(
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y").label("lab"))
            .geom_text(),
        "text",
    );
    ok_svg(
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y").label("lab"))
            .geom_text_with(GeomText {
                size: 12.0,
                color: (0, 0, 0),
                alpha: 1.0,
                hjust: 0.5,
                vjust: 0.5,
                fontfamily: String::new(),
                check_overlap: true,
            }),
        "text_with",
    );
    ok_svg(
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y").label("lab"))
            .geom_label(),
        "label",
    );
    ok_svg(
        GGPlot::new(data)
            .aes(Aes::new().x("x").y("y").label("lab"))
            .geom_label_with(GeomLabel {
                size: 11.0,
                color: (0, 0, 0),
                fill: (240, 240, 240),
                alpha: 1.0,
                padding: 3.0,
                hjust: 0.5,
                vjust: 0.5,
                fontfamily: String::new(),
                check_overlap: false,
            }),
        "label_with",
    );
}

// ─── Reference lines ─────────────────────────────────────────

#[test]
fn reference_lines() {
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .geom_hline(3.0),
        "hline",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .geom_hline_with(GeomHline {
                yintercept: 3.5,
                color: (200, 0, 0),
                width: 1.0,
                linetype: Linetype::Dashed,
                alpha: 0.8,
            }),
        "hline_with",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .geom_vline(2.5),
        "vline",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .geom_vline_with(GeomVline {
                xintercept: 2.0,
                color: (0, 0, 200),
                width: 1.0,
                linetype: Linetype::Dotted,
                alpha: 0.8,
            }),
        "vline_with",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .geom_abline(1.0, 0.0),
        "abline",
    );
    ok_svg(
        GGPlot::new(xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .geom_abline_with(GeomAbline {
                slope: 0.5,
                intercept: 1.0,
                color: (0, 150, 0),
                width: 1.0,
                linetype: Linetype::Solid,
                alpha: 1.0,
            }),
        "abline_with",
    );
}

// ─── Blank ───────────────────────────────────────────────────

#[test]
fn blank_geom() {
    // geom_blank draws nothing but trains scales; still renders an SVG.
    ok_svg(
        GGPlot::new(xy()).aes(Aes::new().x("x").y("y")).geom_blank(),
        "blank",
    );
}
