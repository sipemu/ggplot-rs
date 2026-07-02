//! Coverage-gap-targeting integration + unit tests.
//!
//! Each test exercises a specific still-uncovered area identified from a line
//! coverage report. Rendering tests assert `.render_svg().is_ok()`; pure unit
//! tests assert on returned values.

use ggplot_rs::facet::grid::compute_grid_panels;
use ggplot_rs::facet::wrap::compute_wrap_panels;
use ggplot_rs::prelude::*;
use ggplot_rs::render::backend::{
    LineStyle, Linetype, PointShape, PointStyle, RectStyle, TextStyle,
};
use ggplot_rs::render::Rect;
use ggplot_rs::scale::alpha::ScaleAlphaContinuous;
use ggplot_rs::scale::linetype::ScaleLinetypeDiscrete;
use ggplot_rs::scale::linetype_manual::ScaleLinetypeManual;
use ggplot_rs::scale::shape::ScaleShapeDiscrete;
use ggplot_rs::scale::shape_manual::ScaleShapeManual;
use ggplot_rs::scale::size::ScaleSizeContinuous;
use ggplot_rs::scale::Scale;
use ggplot_rs::stat::identity::StatIdentity;

// ── data helpers ──────────────────────────────────────────────────

fn fcol(name: &str, vals: &[f64]) -> (String, Vec<Value>) {
    (
        name.to_string(),
        vals.iter().map(|v| Value::Float(*v)).collect(),
    )
}

fn scol(name: &str, vals: &[&str]) -> (String, Vec<Value>) {
    (
        name.to_string(),
        vals.iter().map(|s| Value::Str(s.to_string())).collect(),
    )
}

// ════════════════════════════════════════════════════════════════
// 1. src/render/backend.rs — pure unit tests
// ════════════════════════════════════════════════════════════════

#[test]
fn backend_style_defaults() {
    let p = PointStyle::default();
    assert_eq!(p.color, (0, 0, 0));
    assert!(p.filled);
    assert_eq!(p.shape, PointShape::Circle);

    let l = LineStyle::default();
    assert_eq!(l.width, 1.0);
    assert_eq!(l.linetype, Linetype::Solid);

    let r = RectStyle::default();
    assert!(r.clip);
    assert_eq!(r.fill, Some((128, 128, 128)));

    let t = TextStyle::default();
    assert_eq!(t.size, 12.0);
    assert_eq!(t.angle, 0.0);
    assert!(t.family.is_none());
}

#[test]
fn backend_linetype_patterns_all_variants() {
    // Solid has empty pattern; every dashed variant has at least one pair.
    assert!(Linetype::Solid.pattern().is_empty());
    for lt in Linetype::ALL {
        let pat = lt.pattern();
        match lt {
            Linetype::Solid => assert!(pat.is_empty()),
            _ => assert!(!pat.is_empty(), "{lt:?} should have a dash pattern"),
        }
        // Each entry is a (draw, gap) pair with non-negative lengths.
        for (draw, gap) in pat {
            assert!(*draw >= 0.0 && *gap >= 0.0);
        }
    }
    // Spot-check a couple of specific patterns.
    assert_eq!(Linetype::Dashed.pattern(), &[(6.0, 3.0)]);
    assert_eq!(Linetype::DashDot.pattern(), &[(6.0, 2.0), (2.0, 2.0)]);
}

#[test]
fn backend_pointshape_all_present() {
    assert_eq!(PointShape::ALL.len(), 6);
    assert!(PointShape::ALL.contains(&PointShape::Diamond));
    assert!(PointShape::ALL.contains(&PointShape::Plus));
}

// ════════════════════════════════════════════════════════════════
// 2. src/facet/wrap.rs + grid.rs — dead-but-public layout fns
// ════════════════════════════════════════════════════════════════

fn area() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        width: 400.0,
        height: 300.0,
    }
}

#[test]
fn facet_wrap_panels_ncol_some_and_none() {
    let levels = vec![
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
        "d".to_string(),
        "e".to_string(),
    ];

    // ncol = Some
    let panels = compute_wrap_panels(&levels, Some(2), &area(), 20.0);
    assert_eq!(panels.len(), levels.len());
    assert_eq!(panels[0].col, 0);
    assert_eq!(panels[1].col, 1);
    assert_eq!(panels[2].row, 1);
    assert_eq!(panels[0].col_label.as_deref(), Some("a"));

    // ncol = None → sqrt heuristic
    let panels_auto = compute_wrap_panels(&levels, None, &area(), 20.0);
    assert_eq!(panels_auto.len(), levels.len());

    // empty levels → empty vec
    let empty = compute_wrap_panels(&[], Some(3), &area(), 20.0);
    assert!(empty.is_empty());
}

#[test]
fn facet_grid_panels_layout() {
    let rows = vec!["r1".to_string(), "r2".to_string()];
    let cols = vec!["c1".to_string(), "c2".to_string(), "c3".to_string()];
    let panels = compute_grid_panels(&rows, &cols, &area(), 20.0, 30.0);
    assert_eq!(panels.len(), rows.len() * cols.len());
    assert_eq!(panels[0].row_label.as_deref(), Some("r1"));
    assert_eq!(panels[0].col_label.as_deref(), Some("c1"));
    assert!(panels[0].label.contains("r1"));

    // Empty row/col levels: nrow/ncol clamp to 1 for sizing, but the nested
    // loops iterate over the (empty) level lists, so no panels are produced.
    let degenerate = compute_grid_panels(&[], &[], &area(), 20.0, 30.0);
    assert!(degenerate.is_empty());

    // Single row with columns still lays out one row of panels.
    let one_row = compute_grid_panels(&["only".to_string()], &cols, &area(), 20.0, 30.0);
    assert_eq!(one_row.len(), cols.len());
}

// ════════════════════════════════════════════════════════════════
// 3. src/geom/smooth.rs — loess, se on/off, grouped
// ════════════════════════════════════════════════════════════════

fn smooth_data() -> Vec<(String, Vec<Value>)> {
    let xs: Vec<f64> = (0..24).map(|i| i as f64).collect();
    let ys: Vec<f64> = xs.iter().map(|x| 2.0 * x + (x * 0.7).sin() * 5.0).collect();
    vec![fcol("x", &xs), fcol("y", &ys)]
}

#[test]
fn smooth_loess_with_se() {
    let plot = GGPlot::new(smooth_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_smooth_with(GeomSmooth {
            method: SmoothMethod::Loess { span: 0.5 },
            se: true,
            ..Default::default()
        });
    assert!(plot.render_svg().is_ok());
}

#[test]
fn smooth_loess_no_se() {
    let plot = GGPlot::new(smooth_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_smooth_with(GeomSmooth {
            method: SmoothMethod::Loess { span: 0.75 },
            se: false,
            ..Default::default()
        });
    assert!(plot.render_svg().is_ok());
}

#[test]
fn smooth_lm_no_se() {
    let plot = GGPlot::new(smooth_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_smooth_with(GeomSmooth {
            method: SmoothMethod::Lm,
            se: false,
            ..Default::default()
        });
    assert!(plot.render_svg().is_ok());
}

#[test]
fn smooth_grouped_by_color() {
    // Two groups → the per-group smooth branch (color_col present).
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut g = Vec::new();
    for grp in ["a", "b"] {
        for i in 0..16 {
            let x = i as f64;
            xs.push(x);
            ys.push(if grp == "a" { 1.5 * x } else { 30.0 - x });
            g.push(grp);
        }
    }
    let data = vec![fcol("x", &xs), fcol("y", &ys), scol("g", &g)];
    let plot = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_smooth_with(GeomSmooth {
            method: SmoothMethod::Lm,
            se: true,
            ..Default::default()
        });
    assert!(plot.render_svg().is_ok());
}

#[test]
fn smooth_loess_helper_method() {
    // Exercise the GeomSmooth::loess(span) builder helper.
    let plot = GGPlot::new(smooth_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_smooth_with(GeomSmooth::default().loess(0.6));
    assert!(plot.render_svg().is_ok());
}

#[test]
fn smooth_grouped_branch_via_identity_stat() {
    // The standard StatSmooth strips the color column, so the geom's per-group
    // draw branch only runs when a `color` column survives to draw time. Feed
    // pre-fitted x/y/ymin/ymax + color and override the stat to identity so the
    // grouped ribbon+line branch (color_col.or(fill_col) == Some) executes.
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut ymin = Vec::new();
    let mut ymax = Vec::new();
    let mut color = Vec::new();
    for grp in ["a", "b"] {
        for i in 0..10 {
            let xf = i as f64;
            let yf = if grp == "a" { xf } else { 12.0 - xf };
            x.push(xf);
            y.push(yf);
            ymin.push(yf - 1.5);
            ymax.push(yf + 1.5);
            color.push(grp);
        }
    }
    let data = vec![
        fcol("x", &x),
        fcol("y", &y),
        fcol("ymin", &ymin),
        fcol("ymax", &ymax),
        scol("color", &color),
    ];
    let plot = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("color").group("color"))
        .geom_smooth_with(GeomSmooth {
            se: true,
            ..Default::default()
        })
        .stat(StatIdentity);
    assert!(plot.render_svg().is_ok());
}

// ════════════════════════════════════════════════════════════════
// 4. src/position/jitterdodge.rs + dodge2.rs
// ════════════════════════════════════════════════════════════════

fn grouped_xy() -> Vec<(String, Vec<Value>)> {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut fill = Vec::new();
    for (xi, cat) in ["a", "b", "c"].iter().enumerate() {
        for grp in ["g1", "g2"] {
            for k in 0..5 {
                x.push(xi as f64);
                y.push((k as f64) + if grp == "g1" { 0.0 } else { 3.0 });
                fill.push(*cat); // keep x categorical too
                let _ = grp;
            }
        }
    }
    let _ = &fill;
    // Rebuild with real group column
    let mut xr = Vec::new();
    let mut yr = Vec::new();
    let mut g = Vec::new();
    let mut xcat = Vec::new();
    for cat in ["a", "b", "c"] {
        for grp in ["g1", "g2"] {
            for k in 0..5 {
                xcat.push(cat);
                xr.push(match cat {
                    "a" => 0.0,
                    "b" => 1.0,
                    _ => 2.0,
                });
                yr.push((k as f64) + if grp == "g1" { 0.0 } else { 3.0 });
                g.push(grp);
            }
        }
    }
    vec![scol("x", &xcat), fcol("y", &yr), scol("fill", &g)]
}

#[test]
fn position_jitterdodge_points() {
    let plot = GGPlot::new(grouped_xy())
        .aes(Aes::new().x("x").y("y").fill("fill"))
        .geom_point()
        .position(PositionJitterDodge::new(0.3, 0.1).with_dodge_width(0.8));
    assert!(plot.render_svg().is_ok());
}

#[test]
fn position_jitterdodge_default_no_group() {
    // No group column → pure-jitter fallback branch.
    let xs: Vec<f64> = (0..20).map(|i| (i % 4) as f64).collect();
    let ys: Vec<f64> = (0..20).map(|i| i as f64).collect();
    let plot = GGPlot::new(vec![fcol("x", &xs), fcol("y", &ys)])
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .position(PositionJitterDodge::default());
    assert!(plot.render_svg().is_ok());
}

#[test]
fn position_dodge2_bars() {
    let plot = GGPlot::new(grouped_xy())
        .aes(Aes::new().x("x").y("y").fill("fill"))
        .geom_col()
        .position(PositionDodge2::new(0.2));
    assert!(plot.render_svg().is_ok());
}

// ════════════════════════════════════════════════════════════════
// 5. Scales: size / alpha / shape / linetype (+ manual)
// ════════════════════════════════════════════════════════════════

fn n_data() -> Vec<(String, Vec<Value>)> {
    let xs: Vec<f64> = (0..12).map(|i| i as f64).collect();
    let ys: Vec<f64> = (0..12).map(|i| (i * 2 % 7) as f64).collect();
    let ns: Vec<f64> = (0..12).map(|i| (i + 1) as f64).collect();
    let g: Vec<&str> = (0..12).map(|i| ["p", "q", "r"][i % 3]).collect::<Vec<_>>();
    vec![
        fcol("x", &xs),
        fcol("y", &ys),
        fcol("n", &ns),
        scol("g", &g),
    ]
}

#[test]
fn scale_size_continuous_maps_and_legend() {
    let plot = GGPlot::new(n_data())
        .aes(Aes::new().x("x").y("y").size("n"))
        .geom_point();
    assert!(plot.render_svg().is_ok());
}

#[test]
fn scale_alpha_continuous_maps_and_legend() {
    let plot = GGPlot::new(n_data())
        .aes(Aes::new().x("x").y("y").alpha("n"))
        .geom_point();
    assert!(plot.render_svg().is_ok());
}

#[test]
fn scale_shape_discrete_maps_and_legend() {
    let plot = GGPlot::new(n_data())
        .aes(Aes::new().x("x").y("y").shape("g"))
        .geom_point();
    assert!(plot.render_svg().is_ok());
}

#[test]
fn scale_linetype_discrete_maps_and_legend() {
    // Multi-group lines so linetype scale trains + draws.
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut g = Vec::new();
    for grp in ["p", "q", "r"] {
        for i in 0..8 {
            x.push(i as f64);
            y.push(i as f64 + if grp == "q" { 2.0 } else { 0.0 });
            g.push(grp);
        }
    }
    let data = vec![fcol("x", &x), fcol("y", &y), scol("g", &g)];
    let plot = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").linetype("g").group("g"))
        .geom_line();
    assert!(plot.render_svg().is_ok());
}

#[test]
fn scale_shape_manual_explicit() {
    let plot = GGPlot::new(n_data())
        .aes(Aes::new().x("x").y("y").shape("g"))
        .geom_point()
        .scale_shape_manual(vec![
            ("p", PointShape::Triangle),
            ("q", PointShape::Square),
            ("r", PointShape::Diamond),
        ]);
    assert!(plot.render_svg().is_ok());
}

#[test]
fn scale_linetype_manual_explicit() {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut g = Vec::new();
    for grp in ["p", "q"] {
        for i in 0..8 {
            x.push(i as f64);
            y.push(i as f64 + if grp == "q" { 2.0 } else { 0.0 });
            g.push(grp);
        }
    }
    let data = vec![fcol("x", &x), fcol("y", &y), scol("g", &g)];
    let plot = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").linetype("g").group("g"))
        .geom_line()
        .scale_linetype_manual(vec![("p", Linetype::Dashed), ("q", Linetype::Dotted)]);
    assert!(plot.render_svg().is_ok());
}

// ════════════════════════════════════════════════════════════════
// 6. src/geom/path.rs + line.rs — multi-group branch
// ════════════════════════════════════════════════════════════════

fn multi_group_series() -> Vec<(String, Vec<Value>)> {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut g = Vec::new();
    for grp in ["a", "b", "c"] {
        for i in 0..10 {
            x.push(i as f64);
            y.push((i as f64) * if grp == "b" { -1.0 } else { 1.0 } + 5.0);
            g.push(grp);
        }
    }
    vec![fcol("x", &x), fcol("y", &y), scol("g", &g)]
}

#[test]
fn geom_path_multi_group_color() {
    let plot = GGPlot::new(multi_group_series())
        .aes(Aes::new().x("x").y("y").color("g").group("g"))
        .geom_path();
    assert!(plot.render_svg().is_ok());
}

#[test]
fn geom_line_multi_group_color() {
    let plot = GGPlot::new(multi_group_series())
        .aes(Aes::new().x("x").y("y").color("g").group("g"))
        .geom_line();
    assert!(plot.render_svg().is_ok());
}

// ════════════════════════════════════════════════════════════════
// 7. src/render/plotters_backend.rs — text rotation + shapes
// ════════════════════════════════════════════════════════════════

fn rotated_axis_plot(angle: f64) -> bool {
    let data = n_data();
    let theme = theme_gray().set_axis_text_x(ElementText {
        angle,
        ..Default::default()
    });
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme(theme)
        .render_svg()
        .is_ok()
}

#[test]
fn plotters_text_rotation_all_angles() {
    // Cover FontTransform::Rotate90 (80..=100), Rotate180 (170..=190),
    // and the default/Rotate270 arm (45, 270).
    for angle in [45.0, 90.0, 180.0, 270.0] {
        assert!(rotated_axis_plot(angle), "angle {angle} failed to render");
    }
}

#[test]
fn plotters_all_point_shapes_drawn() {
    // Discrete shape aesthetic with >= 6 categories exercises every
    // PointShape arm of draw_shape (Square/Triangle/Diamond/Cross/Plus).
    let cats = ["s0", "s1", "s2", "s3", "s4", "s5"];
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut g = Vec::new();
    for (i, c) in cats.iter().cycle().take(24).enumerate() {
        x.push((i % 6) as f64);
        y.push((i / 6) as f64);
        g.push(*c);
    }
    let data = vec![fcol("x", &x), fcol("y", &y), scol("g", &g)];
    let plot = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").shape("g"))
        .geom_point()
        .scale_shape_manual(vec![
            ("s0", PointShape::Circle),
            ("s1", PointShape::Square),
            ("s2", PointShape::Triangle),
            ("s3", PointShape::Diamond),
            ("s4", PointShape::Cross),
            ("s5", PointShape::Plus),
        ]);
    assert!(plot.render_svg().is_ok());
}

#[test]
fn plotters_geom_text_and_label_render() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0]),
        fcol("y", &[1.0, 2.0, 3.0]),
        scol("lab", &["alpha", "beta", "gamma"]),
    ];
    let t = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").label("lab"))
        .geom_text_with(GeomText::default().with_hjust(0.0).with_vjust(1.0));
    assert!(t.render_svg().is_ok());

    let l = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("lab"))
        .geom_label_with(GeomLabel::default().with_hjust(1.0).with_vjust(0.0));
    assert!(l.render_svg().is_ok());
}

// ════════════════════════════════════════════════════════════════
// 8. Geom variants: density (grouped), boxplot (outliers), rug, density2d
// ════════════════════════════════════════════════════════════════

#[test]
fn geom_density_grouped_fill() {
    let mut x = Vec::new();
    let mut f = Vec::new();
    for grp in ["a", "b"] {
        for i in 0..40 {
            let v = (i as f64) * 0.1 + if grp == "b" { 2.0 } else { 0.0 };
            x.push(v);
            f.push(grp);
        }
    }
    let data = vec![fcol("x", &x), scol("fill", &f)];
    let plot = GGPlot::new(data)
        .aes(Aes::new().x("x").fill("fill").group("fill"))
        .geom_density();
    assert!(plot.render_svg().is_ok());
}

#[test]
fn geom_boxplot_with_outliers() {
    // Each group carries a couple of extreme y values → outlier-drawing branch.
    let mut x = Vec::new();
    let mut y = Vec::new();
    for grp in ["a", "b"] {
        let mut ys = vec![1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0];
        ys.push(50.0); // high outlier
        ys.push(-40.0); // low outlier
        for v in ys {
            x.push(grp);
            y.push(v + if grp == "b" { 1.0 } else { 0.0 });
        }
    }
    let data = vec![scol("x", &x), fcol("y", &y)];
    let plot = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").group("x"))
        .geom_boxplot();
    assert!(plot.render_svg().is_ok());
}

#[test]
fn geom_rug_all_sides() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0, 5.0]),
        fcol("y", &[2.0, 3.0, 1.0, 5.0, 4.0]),
    ];
    let plot = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_rug_with(GeomRug {
            sides: "tblr".to_string(),
            ..Default::default()
        });
    assert!(plot.render_svg().is_ok());
}

#[test]
fn geom_density2d_renders() {
    // A modest point cloud for the 2D density contour estimator.
    let mut x = Vec::new();
    let mut y = Vec::new();
    for i in 0..60 {
        let a = i as f64 * 0.3;
        x.push(a.sin() * 3.0 + (i % 5) as f64);
        y.push(a.cos() * 3.0 + (i % 7) as f64);
    }
    let data = vec![fcol("x", &x), fcol("y", &y)];
    let plot = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_density2d();
    assert!(plot.render_svg().is_ok());
}

// ════════════════════════════════════════════════════════════════
// 9. Direct scale unit tests (map / breaks / set_name / clone / reset)
//    These trait methods are not reached through the rendering pipeline.
// ════════════════════════════════════════════════════════════════

fn strs(vals: &[&str]) -> Vec<Value> {
    vals.iter().map(|s| Value::Str(s.to_string())).collect()
}

#[test]
fn scale_shape_discrete_unit() {
    let mut s = ScaleShapeDiscrete::default();
    s.train(&strs(&["a", "b", "c", "a"]));
    assert!(s.is_discrete());
    assert_eq!(s.map(&Value::Str("a".into())), 0.0);
    assert_eq!(s.map(&Value::Str("c".into())), 2.0);
    assert_eq!(s.map(&Value::Str("missing".into())), 0.0);
    assert_eq!(s.breaks().len(), 3);
    assert_eq!(
        s.map_to_shape(&Value::Str("b".into())),
        Some(PointShape::ALL[1])
    );
    s.set_name("shp");
    assert_eq!(s.name(), "shp");
    let cloned = s.clone_box();
    assert_eq!(cloned.name(), "shp");
    s.reset_training();
    assert!(s.breaks().is_empty());
}

#[test]
fn scale_linetype_discrete_unit() {
    let mut s = ScaleLinetypeDiscrete::default();
    s.train(&strs(&["x", "y"]));
    assert!(s.is_discrete());
    assert_eq!(s.map(&Value::Str("y".into())), 1.0);
    assert_eq!(s.breaks().len(), 2);
    assert_eq!(
        s.map_to_linetype(&Value::Str("x".into())),
        Some(Linetype::ALL[0])
    );
    s.set_name("lt");
    assert_eq!(s.name(), "lt");
    let _ = s.clone_box();
    s.reset_training();
    assert!(s.breaks().is_empty());
}

#[test]
fn scale_size_continuous_unit() {
    let mut s = ScaleSizeContinuous::new()
        .with_range(2.0, 10.0)
        .with_name("n");
    // Untrained breaks are empty.
    assert!(s.breaks().is_empty());
    s.train(&[Value::Float(0.0), Value::Float(10.0), Value::Na]);
    assert_eq!(s.map(&Value::Float(0.0)), 0.0);
    assert_eq!(s.map(&Value::Float(10.0)), 1.0);
    assert_eq!(s.map(&Value::Str("x".into())), 0.0);
    let mapped = s.map_to_size(&Value::Float(5.0)).unwrap();
    assert!((mapped - 6.0).abs() < 1e-9);
    assert!(!s.breaks().is_empty());
    assert_eq!(s.name(), "n");
    s.set_name("m");
    assert_eq!(s.name(), "m");
    let _ = s.clone_box();
    s.reset_training();
    assert!(s.breaks().is_empty());
}

#[test]
fn scale_size_zero_range() {
    let mut s = ScaleSizeContinuous::new();
    s.train(&[Value::Float(3.0), Value::Float(3.0)]);
    // Degenerate range → midpoint mapping and a single break.
    assert_eq!(s.map(&Value::Float(3.0)), 0.5);
    assert_eq!(s.breaks().len(), 1);
}

#[test]
fn scale_alpha_continuous_unit() {
    let mut s = ScaleAlphaContinuous::new()
        .with_range(0.1, 0.9)
        .with_name("a");
    s.train(&[Value::Float(0.0), Value::Float(4.0)]);
    assert_eq!(s.map(&Value::Float(0.0)), 0.0);
    assert_eq!(s.map(&Value::Float(4.0)), 1.0);
    let a = s.map_to_alpha(&Value::Float(2.0)).unwrap();
    assert!((a - 0.5).abs() < 1e-9);
    assert!(!s.breaks().is_empty());
    s.set_name("b");
    assert_eq!(s.name(), "b");
    let _ = s.clone_box();
    s.reset_training();
    assert!(s.breaks().is_empty());
}

#[test]
fn scale_shape_manual_unit() {
    let mut s = ScaleShapeManual::new(vec![("a", PointShape::Triangle), ("b", PointShape::Square)]);
    // Training a new level extends past the provided shapes → modulo wrap.
    s.train(&strs(&["a", "b", "c"]));
    assert!(s.is_discrete());
    assert_eq!(s.map(&Value::Str("b".into())), 1.0);
    assert_eq!(
        s.map_to_shape(&Value::Str("a".into())),
        Some(PointShape::Triangle)
    );
    // "c" is index 2, wraps modulo the 2 provided shapes.
    assert!(s.map_to_shape(&Value::Str("c".into())).is_some());
    assert_eq!(s.breaks().len(), 3);
    s.set_name("shm");
    assert_eq!(s.name(), "shm");
    let _ = s.clone_box();
    s.reset_training();
    assert!(s.breaks().is_empty());
}

#[test]
fn scale_linetype_manual_unit() {
    let mut s = ScaleLinetypeManual::new(vec![("a", Linetype::Dashed), ("b", Linetype::Dotted)]);
    s.train(&strs(&["a", "b", "extra"]));
    assert_eq!(s.map(&Value::Str("b".into())), 1.0);
    assert_eq!(
        s.map_to_linetype(&Value::Str("a".into())),
        Some(Linetype::Dashed)
    );
    assert!(s.map_to_linetype(&Value::Str("extra".into())).is_some());
    assert_eq!(s.breaks().len(), 3);
    s.set_name("ltm");
    assert_eq!(s.name(), "ltm");
    let _ = s.clone_box();
    s.reset_training();
    assert!(s.breaks().is_empty());
}
