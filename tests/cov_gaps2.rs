//! Coverage-focused integration + unit tests targeting under-covered modules:
//! renderer, guide (legend/axis), dataframe, data::Value, scale trait defaults,
//! position::dodge2, and a tail of geoms/scales.

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::position::{Position, PositionParams};
use ggplot_rs::prelude::*;
use ggplot_rs::scale::Scale;

// ─── helpers ─────────────────────────────────────────────────────────────

fn fcol(name: &str, v: &[f64]) -> (String, Vec<Value>) {
    (
        name.to_string(),
        v.iter().map(|f| Value::Float(*f)).collect(),
    )
}

fn scol(name: &str, v: &[&str]) -> (String, Vec<Value>) {
    (
        name.to_string(),
        v.iter().map(|s| Value::Str(s.to_string())).collect(),
    )
}

// ═══════════════════════════════════════════════════════════════════════
// 1. renderer.rs — special draw paths
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn renderer_continuous_colorbar_viridis_c() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0, 5.0]),
        fcol("y", &[2.0, 1.0, 4.0, 3.0, 5.0]),
        fcol("z", &[10.0, 20.0, 30.0, 40.0, 50.0]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .scale_color_viridis_c()
        .render_svg();
    assert!(svg.is_ok(), "colorbar viridis_c: {svg:?}");
}

#[test]
fn renderer_continuous_colorbar_gradientn() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0]),
        fcol("y", &[2.0, 1.0, 4.0, 3.0]),
        fcol("z", &[-5.0, 0.0, 5.0, 10.0]),
    ];
    let stops = vec![
        (0.0, RGBAColor::new(0, 0, 255)),
        (0.5, RGBAColor::new(255, 255, 255)),
        (1.0, RGBAColor::new(255, 0, 0)),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .scale_color_gradientn(stops)
        .render_svg();
    assert!(svg.is_ok(), "colorbar gradientn: {svg:?}");
}

#[test]
fn renderer_secondary_axis_both() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0]),
        fcol("y", &[10.0, 20.0, 30.0, 40.0]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .scale_y_continuous(
            ScaleContinuous::new().with_sec_axis(SecAxis::new(|v| v * 1.8 + 32.0).with_name("F")),
        )
        .scale_x_continuous(
            ScaleContinuous::new()
                .with_sec_axis(SecAxis::new(|v| v * 2.0).with_breaks(vec![2.0, 4.0, 6.0])),
        )
        .render_svg();
    assert!(svg.is_ok(), "sec_axis: {svg:?}");
}

#[test]
fn renderer_title_subtitle_caption() {
    let data = vec![fcol("x", &[1.0, 2.0, 3.0]), fcol("y", &[2.0, 4.0, 6.0])];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("Main Title")
        .subtitle("A subtitle here")
        .caption("Source: tests")
        .xlab("X axis")
        .ylab("Y axis")
        .render_svg();
    assert!(svg.is_ok(), "title/subtitle/caption: {svg:?}");
}

#[test]
fn renderer_legend_each_position() {
    for pos in [
        LegendPosition::Top,
        LegendPosition::Bottom,
        LegendPosition::Left,
        LegendPosition::Right,
        LegendPosition::None,
    ] {
        let data = vec![
            fcol("x", &[1.0, 2.0, 3.0, 4.0]),
            fcol("y", &[2.0, 4.0, 1.0, 3.0]),
            scol("g", &["a", "b", "a", "b"]),
        ];
        let svg = GGPlot::new(data)
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_point()
            .theme(Theme::default().set_legend_position(pos.clone()))
            .render_svg();
        assert!(svg.is_ok(), "legend position {pos:?}: {svg:?}");
    }
}

#[test]
fn renderer_coord_flip_polar_fixed() {
    let base = || {
        vec![
            fcol("x", &[1.0, 2.0, 3.0, 4.0]),
            fcol("y", &[2.0, 4.0, 1.0, 3.0]),
        ]
    };
    assert!(GGPlot::new(base())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_flip()
        .render_svg()
        .is_ok());
    assert!(GGPlot::new(base())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_polar()
        .render_svg()
        .is_ok());
    assert!(GGPlot::new(base())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_fixed(1.0)
        .render_svg()
        .is_ok());
}

#[test]
fn renderer_multiple_legends_at_once() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        fcol("y", &[2.0, 4.0, 1.0, 3.0, 5.0, 2.0]),
        scol("cat", &["a", "b", "a", "b", "a", "b"]),
        scol("shp", &["p", "q", "p", "q", "p", "q"]),
        fcol("sz", &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        fcol("al", &[0.1, 0.3, 0.5, 0.7, 0.9, 1.0]),
    ];
    let svg = GGPlot::new(data)
        .aes(
            Aes::new()
                .x("x")
                .y("y")
                .color("cat")
                .shape("shp")
                .size("sz")
                .alpha("al"),
        )
        .geom_point()
        .render_svg();
    assert!(svg.is_ok(), "multi-legend: {svg:?}");
}

#[test]
fn renderer_faceted_with_legend_and_subcaption() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0, 1.0, 2.0, 3.0, 4.0]),
        fcol("y", &[2.0, 4.0, 1.0, 3.0, 3.0, 1.0, 4.0, 2.0]),
        scol("g", &["a", "b", "a", "b", "a", "b", "a", "b"]),
        scol("panel", &["p1", "p1", "p1", "p1", "p2", "p2", "p2", "p2"]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .facet_wrap("panel", Some(2))
        .title("Faceted")
        .subtitle("with legend")
        .caption("cap")
        .render_svg();
    assert!(svg.is_ok(), "faceted+legend: {svg:?}");
}

// ═══════════════════════════════════════════════════════════════════════
// 2. guide/legend.rs + guide/axis.rs
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn guide_all_discrete_aesthetics() {
    for aes_kind in ["color", "fill", "shape", "linetype"] {
        let data = vec![
            fcol("x", &[1.0, 2.0, 3.0, 4.0]),
            fcol("y", &[2.0, 4.0, 1.0, 3.0]),
            scol("g", &["a", "b", "a", "b"]),
        ];
        let mut m = Aes::new().x("x").y("y");
        m = match aes_kind {
            "color" => m.color("g"),
            "fill" => m.fill("g"),
            "shape" => m.shape("g"),
            _ => m.linetype("g"),
        };
        let svg = GGPlot::new(data).aes(m).geom_point().render_svg();
        assert!(svg.is_ok(), "discrete legend {aes_kind}: {svg:?}");
    }
}

#[test]
fn guide_continuous_size_and_alpha() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0]),
        fcol("y", &[2.0, 4.0, 1.0, 3.0]),
        fcol("v", &[10.0, 20.0, 30.0, 40.0]),
    ];
    let svg_size = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").size("v"))
        .geom_point()
        .scale_size(
            ScaleSizeContinuous::new()
                .with_range(2.0, 8.0)
                .with_name("Size"),
        )
        .render_svg();
    assert!(svg_size.is_ok(), "size legend: {svg_size:?}");

    let svg_alpha = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").alpha("v"))
        .geom_point()
        .scale_alpha(
            ScaleAlphaContinuous::new()
                .with_range(0.2, 1.0)
                .with_name("Alpha"),
        )
        .render_svg();
    assert!(svg_alpha.is_ok(), "alpha legend: {svg_alpha:?}");
}

#[test]
fn guide_config_reverse_ncol_nrow() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        fcol("y", &[2.0, 4.0, 1.0, 3.0, 5.0, 6.0]),
        scol("g", &["a", "b", "c", "d", "e", "f"]),
    ];
    let reverse = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guides(GuideLegend::new().with_title("Groups").reverse())
        .render_svg();
    assert!(reverse.is_ok(), "guides reverse: {reverse:?}");

    let ncol = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guides(GuideLegend::new().with_ncol(2))
        .render_svg();
    assert!(ncol.is_ok(), "guides ncol: {ncol:?}");

    let nrow = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guides(GuideLegend::new().with_nrow(3))
        .render_svg();
    assert!(nrow.is_ok(), "guides nrow: {nrow:?}");
}

#[test]
fn axis_log_and_custom_breaks() {
    let data = vec![
        fcol("x", &[1.0, 10.0, 100.0, 1000.0]),
        fcol("y", &[1.0, 2.0, 3.0, 4.0]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_log10()
        .scale_y_continuous(
            ScaleContinuous::new()
                .with_breaks(vec![1.0, 2.0, 3.0, 4.0])
                .with_labels(vec![
                    "one".into(),
                    "two".into(),
                    "three".into(),
                    "four".into(),
                ]),
        )
        .render_svg();
    assert!(svg.is_ok(), "axis log/custom: {svg:?}");
}

// ═══════════════════════════════════════════════════════════════════════
// 3. data/dataframe.rs — direct unit tests
// ═══════════════════════════════════════════════════════════════════════

fn small_df() -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column(
        "cat".into(),
        vec![
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("a".into()),
        ],
    );
    df.add_column(
        "val".into(),
        vec![Value::Float(3.0), Value::Float(1.0), Value::Float(2.0)],
    );
    df
}

#[test]
fn df_accessors_and_names() {
    let df = small_df();
    assert_eq!(df.nrows(), 3);
    assert_eq!(df.ncols(), 2);
    assert_eq!(df.column_names(), vec!["cat", "val"]);
    assert!(df.has_column("cat"));
    assert!(!df.has_column("nope"));
    assert!(df.column("val").is_some());
    assert!(df.column("nope").is_none());
}

#[test]
fn df_column_mut_and_default() {
    let mut df = small_df();
    if let Some(col) = df.column_mut("val") {
        col[0] = Value::Float(99.0);
    }
    assert_eq!(df.column("val").unwrap()[0].as_f64(), Some(99.0));
    assert!(df.column_mut("nope").is_none());
    let d = DataFrame::default();
    assert_eq!(d.nrows(), 0);
    assert_eq!(d.ncols(), 0);
}

#[test]
fn df_group_by_multi_and_empty() {
    let df = small_df();
    let groups = df.group_by(&["cat"]);
    assert_eq!(groups.len(), 2);
    // multi-key group_by with a missing key column falls back to "NA"
    let g2 = df.group_by(&["cat", "missing"]);
    assert_eq!(g2.len(), 2);
    // empty frame
    let empty = DataFrame::new();
    assert!(empty.group_by(&["cat"]).is_empty());
}

#[test]
fn df_vstack_with_missing_columns() {
    let mut a = DataFrame::new();
    a.add_column("x".into(), vec![Value::Float(1.0)]);
    a.add_column("y".into(), vec![Value::Float(2.0)]);

    let mut b = DataFrame::new();
    b.add_column("x".into(), vec![Value::Float(3.0)]);
    b.add_column("z".into(), vec![Value::Float(4.0)]);

    a.vstack(&b);
    assert_eq!(a.nrows(), 2);
    // y filled with NA for b's row; z added and back-filled with NA for a's row
    assert!(a.has_column("z"));
    assert!(a.column("y").unwrap()[1].is_na());
    assert!(a.column("z").unwrap()[0].is_na());

    // vstack of an empty frame is a no-op; vstack onto empty clones.
    let mut c = DataFrame::new();
    c.vstack(&a);
    assert_eq!(c.nrows(), 2);
    let n_before = c.nrows();
    c.vstack(&DataFrame::new());
    assert_eq!(c.nrows(), n_before);
}

#[test]
fn df_select_row_sort_unique() {
    let df = small_df();
    let sel = df.select(&["val", "missing"]);
    assert_eq!(sel.ncols(), 1);
    assert!(sel.has_column("val"));

    let r = df.row(1);
    assert_eq!(r.get("cat").unwrap().as_str(), Some("b"));

    let sorted = df.sort_by("val");
    let v = sorted.column("val").unwrap();
    assert_eq!(v[0].as_f64(), Some(1.0));
    assert_eq!(v[2].as_f64(), Some(3.0));
    // sort by missing column returns a clone unchanged
    let same = df.sort_by("missing");
    assert_eq!(same.nrows(), 3);

    let uniq = df.unique_values("cat");
    assert_eq!(uniq.len(), 2);
    assert!(df.unique_values("missing").is_empty());
}

#[test]
fn df_from_rows_and_csv() {
    use indexmap::IndexMap;
    let mut r1 = IndexMap::new();
    r1.insert("a".to_string(), Value::Float(1.0));
    let mut r2 = IndexMap::new();
    r2.insert("a".to_string(), Value::Float(2.0));
    r2.insert("b".to_string(), Value::Str("x".into()));
    let df = DataFrame::from_rows(vec![r1, r2]);
    assert_eq!(df.nrows(), 2);
    assert!(df.has_column("b"));
    assert!(df.column("b").unwrap()[0].is_na());
    assert!(DataFrame::from_rows(vec![]).nrows() == 0);

    // from_csv round-trip
    let dir = std::env::temp_dir();
    let path = dir.join("cov_gaps2_df.csv");
    std::fs::write(&path, "x,y\n1,2\nNA,foo\n\n").unwrap();
    let loaded = DataFrame::from_csv(path.to_str().unwrap()).unwrap();
    assert_eq!(loaded.nrows(), 2);
    assert!(loaded.column("x").unwrap()[1].is_na());
    assert_eq!(loaded.column("y").unwrap()[1].as_str(), Some("foo"));
    std::fs::remove_file(&path).ok();
    // Missing file errors.
    assert!(DataFrame::from_csv("/no/such/cov_gaps2.csv").is_err());
}

// ═══════════════════════════════════════════════════════════════════════
// 4. data/mod.rs — Value methods
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn value_as_f64_all_variants() {
    assert_eq!(Value::Float(1.5).as_f64(), Some(1.5));
    assert_eq!(Value::Integer(3).as_f64(), Some(3.0));
    assert_eq!(Value::DateTime(100).as_f64(), Some(100.0));
    assert_eq!(Value::Str("x".into()).as_f64(), None);
    assert_eq!(Value::Bool(true).as_f64(), None);
    assert_eq!(Value::Na.as_f64(), None);
}

#[test]
fn value_predicates_and_constructors() {
    assert!(Value::Na.is_na());
    assert!(!Value::Float(0.0).is_na());
    assert!(Value::DateTime(0).is_datetime());
    assert!(!Value::Integer(0).is_datetime());
    assert_eq!(Value::from_timestamp(42), Value::DateTime(42));
    assert_eq!(Value::Str("hi".into()).as_str(), Some("hi"));
    assert_eq!(Value::Float(1.0).as_str(), None);
}

#[test]
fn value_to_group_key_all_variants() {
    assert_eq!(Value::Float(1.5).to_group_key(), "1.5");
    assert_eq!(Value::Integer(7).to_group_key(), "7");
    assert_eq!(Value::Str("z".into()).to_group_key(), "z");
    assert_eq!(Value::Bool(true).to_group_key(), "true");
    assert_eq!(Value::Na.to_group_key(), "NA");
    // DateTime uses format_epoch_secs
    assert_eq!(Value::DateTime(0).to_group_key(), "1970-01-01");
}

#[test]
fn value_format_epoch_secs_variants() {
    use ggplot_rs::data::format_epoch_secs;
    assert_eq!(format_epoch_secs(0), "1970-01-01");
    // with time component
    assert_eq!(format_epoch_secs(3661), "1970-01-01 01:01:01");
    // negative (before epoch)
    let neg = format_epoch_secs(-86400);
    assert_eq!(neg, "1969-12-31");
    // a known modern date
    assert_eq!(format_epoch_secs(1_600_000_000), "2020-09-13 12:26:40");
}

#[test]
fn value_partial_eq_variants() {
    assert_eq!(Value::Float(1.0), Value::Float(1.0));
    assert_ne!(Value::Float(1.0), Value::Float(2.0));
    assert_eq!(Value::Integer(5), Value::Integer(5));
    assert_eq!(Value::Bool(false), Value::Bool(false));
    assert_eq!(Value::DateTime(9), Value::DateTime(9));
    assert_eq!(Value::Na, Value::Na);
    assert_ne!(Value::Float(1.0), Value::Integer(1));
    assert_ne!(Value::Str("a".into()), Value::Str("b".into()));
}

// ═══════════════════════════════════════════════════════════════════════
// 5. scale/mod.rs — Scale trait default methods
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn scale_trait_default_methods() {
    let v = Value::Float(1.0);

    // ScaleColorDiscrete overrides map_to_color/is_discrete/reset_training but
    // NOT the other aesthetic mappers / sec_axis / filter_limits / domain /
    // set_limits / transform — so those hit the trait defaults.
    let mut disc = ScaleColorDiscrete::new(ggplot_rs::aes::Aesthetic::Color);
    disc.train(&[Value::Str("a".into())]);
    assert!(disc.map_to_shape(&v).is_none());
    assert!(disc.map_to_linetype(&v).is_none());
    assert!(disc.map_to_size(&v).is_none());
    assert!(disc.map_to_alpha(&v).is_none());
    assert!(disc.sec_axis().is_none());
    assert!(disc.filter_limits().is_none());
    assert!(disc.domain().is_none());
    disc.set_limits(0.0, 1.0); // default no-op
    assert_eq!(disc.transform(&v), v); // default clone

    // ScaleContinuous does not override map_to_color or is_discrete.
    let cont = ScaleContinuous::new();
    assert!(cont.map_to_color(&v).is_none());
    assert!(!cont.is_discrete());
}

// ═══════════════════════════════════════════════════════════════════════
// 6. position/dodge2.rs — xmin/xmax branch + early returns
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dodge2_shifts_x_and_xmin_xmax() {
    let mut df = DataFrame::new();
    df.add_column(
        "x".into(),
        vec![
            Value::Float(1.0),
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(2.0),
        ],
    );
    df.add_column(
        "fill".into(),
        vec![
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("a".into()),
            Value::Str("b".into()),
        ],
    );
    df.add_column("xmin".into(), vec![Value::Float(0.6); 4]);
    df.add_column("xmax".into(), vec![Value::Float(1.4); 4]);

    let pos = PositionDodge2::new(0.1);
    pos.compute(&mut df, &PositionParams::default());

    // Group "a" shifts left, "b" shifts right relative to their x.
    let x = df.column("x").unwrap();
    assert!(x[0].as_f64().unwrap() < 1.0, "group a shifts left of 1.0");
    assert!(x[1].as_f64().unwrap() > 1.0, "group b shifts right of 1.0");
    // xmin/xmax recomputed around the shifted centers.
    let xmin = df.column("xmin").unwrap();
    let xmax = df.column("xmax").unwrap();
    assert!(xmin[0].as_f64().unwrap() < xmax[0].as_f64().unwrap());
    assert_eq!(pos.name(), "dodge2");
}

#[test]
fn dodge2_early_returns() {
    // No x column → early return.
    let mut no_x = DataFrame::new();
    no_x.add_column("fill".into(), vec![Value::Str("a".into())]);
    PositionDodge2::default().compute(&mut no_x, &PositionParams::default());

    // No grouping column → early return.
    let mut no_grp = DataFrame::new();
    no_grp.add_column("x".into(), vec![Value::Float(1.0), Value::Float(2.0)]);
    PositionDodge2::default().compute(&mut no_grp, &PositionParams::default());

    // Single group → early return.
    let mut one_grp = DataFrame::new();
    one_grp.add_column("x".into(), vec![Value::Float(1.0), Value::Float(2.0)]);
    one_grp.add_column(
        "group".into(),
        vec![Value::Str("a".into()), Value::Str("a".into())],
    );
    PositionDodge2::default().compute(&mut one_grp, &PositionParams::default());
    // x unchanged for single group
    assert_eq!(one_grp.column("x").unwrap()[0].as_f64(), Some(1.0));
}

#[test]
fn dodge2_via_render_histogram() {
    // Histogram (StatBin emits xmin/xmax) with a fill group + dodge2 renders.
    let data = vec![
        fcol("v", &[1.0, 1.2, 2.0, 2.1, 3.0, 3.3, 1.1, 2.2, 3.1, 1.3]),
        scol("g", &["a", "a", "a", "b", "b", "b", "a", "b", "a", "b"]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("v").fill("g"))
        .geom_histogram()
        .position(PositionDodge2::new(0.2))
        .render_svg();
    assert!(svg.is_ok(), "histogram dodge2 render: {svg:?}");
}

// ═══════════════════════════════════════════════════════════════════════
// 7. Geom / scale tail
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn geom_density2d_renders_contours() {
    // A clustered point cloud gives StatDensity2d something to contour.
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for i in 0..60 {
        let a = (i as f64) * 0.31;
        xs.push(2.0 + a.sin());
        ys.push(2.0 + a.cos() * 0.9 + (i as f64) * 0.005);
    }
    let data = vec![fcol("x", &xs), fcol("y", &ys)];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_density2d_with(GeomDensity2d::new().with_color((10, 20, 30)).with_levels(6))
        .render_svg();
    assert!(svg.is_ok(), "density2d: {svg:?}");
}

#[test]
fn geom_qq_and_qqline_with_color() {
    let vals: Vec<f64> = (0..40).map(|i| ((i as f64) - 20.0) * 0.5).collect();
    let groups: Vec<&str> = (0..40)
        .map(|i| if i % 2 == 0 { "a" } else { "b" })
        .collect();
    let data = vec![fcol("y", &vals), scol("grp", &groups)];
    let svg = GGPlot::new(data)
        .aes(Aes::new().y("y").color("grp"))
        .geom_qq()
        .geom_qq_line()
        .render_svg();
    assert!(svg.is_ok(), "qq+qqline: {svg:?}");
}

#[test]
fn scale_gradient2_low_mid_high_and_nonnumeric() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0, 5.0]),
        fcol("y", &[1.0, 2.0, 3.0, 4.0, 5.0]),
        fcol("z", &[-4.0, -1.0, 0.0, 2.0, 6.0]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .scale_color_gradient2(
            RGBAColor::new(0, 0, 255),
            RGBAColor::new(255, 255, 255),
            RGBAColor::new(255, 0, 0),
        )
        .render_svg();
    assert!(svg.is_ok(), "gradient2 render: {svg:?}");

    // Direct map_to_color: below midpoint, above midpoint, and non-numeric → None.
    let mut g = ScaleColorGradient2::new(ggplot_rs::aes::Aesthetic::Color).with_midpoint(0.0);
    g.train(&[Value::Float(-4.0), Value::Float(6.0)]);
    assert!(g.map_to_color(&Value::Float(-2.0)).is_some());
    assert!(g.map_to_color(&Value::Float(4.0)).is_some());
    assert!(g.map_to_color(&Value::Str("nope".into())).is_none());
    assert!(g.domain().is_some());
}

#[test]
fn scale_grey_range_edges() {
    // Multiple levels + full black-to-white range.
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0]),
        fcol("y", &[1.0, 2.0, 3.0, 4.0]),
        scol("g", &["a", "b", "c", "d"]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("g"))
        .geom_col()
        .scale_fill_grey_with(
            ScaleColorGrey::new(ggplot_rs::aes::Aesthetic::Fill).with_range(0.0, 1.0),
        )
        .render_svg();
    assert!(svg.is_ok(), "grey multi: {svg:?}");

    // Single-level branch (n == 1 midpoint path) via direct map.
    let mut grey = ScaleColorGrey::new(ggplot_rs::aes::Aesthetic::Fill).with_range(0.1, 0.9);
    grey.train(&[Value::Str("only".into())]);
    let c = grey.map_to_color(&Value::Str("only".into())).unwrap();
    assert_eq!(c.0, c.1);
    assert_eq!(c.1, c.2);
    // Unknown value falls back to index 0.
    assert!(grey.map_to_color(&Value::Str("other".into())).is_some());
}

#[test]
fn geom_boxplot_horizontal_with_outliers() {
    // Include an extreme value so the outlier-drawing branch executes.
    let ys = [
        10.0, 11.0, 12.0, 13.0, 12.5, 11.5, 10.5, 13.5, 12.2, 100.0, 11.8, 12.8,
    ];
    let cats: Vec<&str> = ys.iter().map(|_| "A").collect();
    let data = vec![scol("cat", &cats), fcol("y", &ys)];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("cat").y("y"))
        .geom_boxplot()
        .coord_flip()
        .render_svg();
    assert!(svg.is_ok(), "boxplot horizontal: {svg:?}");
}

#[test]
fn geom_path_grouped_and_plain() {
    // Grouped by color + linetype.
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 1.0, 2.0, 3.0]),
        fcol("y", &[1.0, 3.0, 2.0, 2.0, 1.0, 3.0]),
        scol("g", &["a", "a", "a", "b", "b", "b"]),
    ];
    let grouped = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g").linetype("g"))
        .geom_path()
        .render_svg();
    assert!(grouped.is_ok(), "grouped path: {grouped:?}");

    // Plain single path (no color grouping).
    let data2 = vec![fcol("x", &[3.0, 1.0, 2.0]), fcol("y", &[1.0, 2.0, 3.0])];
    let plain = GGPlot::new(data2)
        .aes(Aes::new().x("x").y("y"))
        .geom_path()
        .render_svg();
    assert!(plain.is_ok(), "plain path: {plain:?}");
}

#[test]
fn geom_refline_custom_styles() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0]),
        fcol("y", &[1.0, 2.0, 3.0, 4.0]),
    ];
    let mut hline = GeomHline::new(2.5);
    hline.color = (255, 0, 0);
    hline.linetype = Linetype::Solid;
    hline.width = 2.0;
    hline.alpha = 0.5;

    let mut vline = GeomVline::new(2.5);
    vline.color = (0, 128, 0);
    vline.linetype = Linetype::Dotted;

    let mut abline = GeomAbline::new(1.0, 0.0);
    abline.color = (0, 0, 255);
    abline.linetype = Linetype::Dashed;

    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_hline_with(hline)
        .geom_vline_with(vline)
        .geom_abline_with(abline)
        .render_svg();
    assert!(svg.is_ok(), "refline custom: {svg:?}");
}

#[test]
fn geom_text_and_label_check_overlap() {
    let data = vec![
        fcol("x", &[1.0, 1.01, 3.0]),
        fcol("y", &[1.0, 1.01, 3.0]),
        scol("label", &["aaa", "bbb", "ccc"]),
    ];
    let text = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(
            GeomText::default()
                .with_hjust(0.0)
                .with_vjust(1.0)
                .with_fontfamily("serif")
                .with_check_overlap(true),
        )
        .render_svg();
    assert!(text.is_ok(), "text overlap: {text:?}");

    let label = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label_with(
            GeomLabel::default()
                .with_hjust(1.0)
                .with_vjust(0.0)
                .with_check_overlap(true),
        )
        .render_svg();
    assert!(label.is_ok(), "label overlap: {label:?}");
}

#[test]
fn scale_color_discrete_and_continuous_helpers() {
    // Exercise ScaleColorDiscrete helper accessors + ScaleColorContinuous.
    let mut disc = ScaleColorDiscrete::new(ggplot_rs::aes::Aesthetic::Color)
        .with_palette(vec![RGBAColor::new(1, 2, 3), RGBAColor::new(4, 5, 6)]);
    disc.train(&[Value::Str("a".into()), Value::Str("b".into())]);
    assert_eq!(disc.levels().len(), 2);
    let c = disc.color_for_value(&Value::Str("b".into()));
    assert_eq!((c.r, c.g, c.b), (4, 5, 6));
    // wrap-around index
    let c2 = disc.color_for_index(2);
    assert_eq!((c2.r, c2.g, c2.b), (1, 2, 3));
    assert!(disc.breaks().len() == 2);

    let mut cont = ScaleColorContinuous::new(ggplot_rs::aes::Aesthetic::Color);
    cont.train(&[Value::Float(0.0), Value::Float(10.0)]);
    let mid = cont.color_at(0.5);
    let _ = (mid.r, mid.g, mid.b);
    assert!(cont.map_to_color(&Value::Float(5.0)).is_some());
    assert!(!cont.breaks().is_empty());
    assert!(cont.domain().is_some());

    // RGBAColor helpers
    let base = RGBAColor::new(0, 0, 0).with_alpha(0.5);
    let mixed = base.lerp(&RGBAColor::new(100, 100, 100), 0.5);
    assert_eq!(mixed.r, 50);
}

// ═══════════════════════════════════════════════════════════════════════
// 8. Extra scattered-line unit coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn aes_all_builder_methods() {
    let a = Aes::new()
        .x("x")
        .y("y")
        .color("c")
        .fill("f")
        .size("s")
        .shape("sh")
        .alpha("al")
        .group("g")
        .ymin("ymn")
        .ymax("ymx")
        .xmin("xmn")
        .xmax("xmx")
        .label("lb")
        .weight("w")
        .xend("xe")
        .yend("ye")
        .angle("an")
        .radius("r")
        .linetype("lt")
        .after_stat_x("ax")
        .after_stat_y("ay")
        .after_stat_fill("af")
        .after_stat_color("ac")
        .after_stat_size("asz")
        .after_stat_alpha("aal");
    assert_eq!(a.get_mapping(&ggplot_rs::aes::Aesthetic::X), Some("x"));
    assert_eq!(a.get_mapping(&ggplot_rs::aes::Aesthetic::Angle), Some("an"));
    assert!(a.get_mapping(&ggplot_rs::aes::Aesthetic::Radius).is_some());

    // col_name for every aesthetic variant.
    use ggplot_rs::aes::Aesthetic::*;
    for aes in [
        X, Y, Color, Fill, Size, Shape, Alpha, Linetype, Group, Ymin, Ymax, Xmin, Xmax, Label,
        Weight, Xend, Yend, Angle, Radius,
    ] {
        assert!(!aes.col_name().is_empty());
    }

    // merge override
    let b = Aes::new().x("newx");
    let merged = a.merge(&b);
    assert_eq!(
        merged.get_mapping(&ggplot_rs::aes::Aesthetic::X),
        Some("newx")
    );
}

#[test]
fn sec_axis_methods_direct() {
    let sec = SecAxis::new(|c| c * 9.0 / 5.0 + 32.0)
        .with_name("Fahrenheit")
        .with_breaks(vec![0.0, 50.0, 100.0]);
    assert_eq!(sec.transform_value(100.0), 212.0);
    assert_eq!(sec.name, "Fahrenheit");
    assert!(sec.breaks.is_some());
    // Debug impl
    let dbg = format!("{sec:?}");
    assert!(dbg.contains("SecAxis"));
    let clone = sec.clone();
    assert_eq!(clone.transform_value(0.0), 32.0);
}

#[test]
fn scales_map_setname_clonebox_breaks() {
    use ggplot_rs::aes::Aesthetic;

    // Discrete color
    let mut disc = ScaleColorDiscrete::new(Aesthetic::Color);
    disc.train(&[Value::Str("a".into()), Value::Str("b".into())]);
    assert_eq!(disc.map(&Value::Str("b".into())), 1.0);
    assert_eq!(disc.map(&Value::Str("missing".into())), 0.0);
    disc.set_name("Cat");
    assert_eq!(disc.name(), "Cat");
    assert!(disc.is_discrete());
    assert_eq!(disc.aesthetic(), Aesthetic::Color);
    let _ = disc.clone_box();
    disc.reset_training();
    assert!(disc.breaks().is_empty());

    // Continuous color
    let mut cont = ScaleColorContinuous::new(Aesthetic::Color);
    cont.train(&[Value::Float(0.0), Value::Float(10.0)]);
    assert!(cont.map(&Value::Float(5.0)) > 0.0);
    assert_eq!(cont.map(&Value::Str("x".into())), 0.0);
    cont.set_name("N");
    assert_eq!(cont.name(), "N");
    let _ = cont.clone_box();
    cont.reset_training();

    // Gradient2
    let mut g2 = ScaleColorGradient2::new(Aesthetic::Fill).with_midpoint(0.0);
    g2.train(&[Value::Float(-5.0), Value::Float(5.0)]);
    assert!(g2.map(&Value::Float(0.0)) >= 0.0);
    assert_eq!(g2.map(&Value::Str("x".into())), 0.0);
    assert!(!g2.breaks().is_empty());
    g2.set_name("G2");
    assert_eq!(g2.name(), "G2");
    assert_eq!(g2.aesthetic(), Aesthetic::Fill);
    let _ = g2.clone_box();
    g2.reset_training();
    assert!(g2.breaks().is_empty());

    // Grey
    let mut grey = ScaleColorGrey::new(Aesthetic::Color);
    grey.train(&[Value::Str("a".into()), Value::Str("b".into())]);
    assert_eq!(grey.map(&Value::Str("b".into())), 1.0);
    grey.set_name("Grey");
    assert_eq!(grey.name(), "Grey");
    assert!(grey.is_discrete());
    assert_eq!(grey.breaks().len(), 2);
    let _ = grey.clone_box();
    grey.reset_training();

    // Size
    let mut size = ScaleSizeContinuous::new().with_range(1.0, 6.0);
    size.train(&[Value::Float(1.0), Value::Float(9.0)]);
    assert!(size.map_to_size(&Value::Float(5.0)).is_some());
    assert_eq!(size.map(&Value::Str("x".into())), 0.0);
    assert!(!size.breaks().is_empty());
    size.set_name("Size");
    assert_eq!(size.name(), "Size");
    assert_eq!(size.aesthetic(), Aesthetic::Size);
    let _ = size.clone_box();
    size.reset_training();
    assert!(size.breaks().is_empty());

    // Alpha
    let mut alpha = ScaleAlphaContinuous::new().with_range(0.1, 1.0);
    alpha.train(&[Value::Float(1.0), Value::Float(9.0)]);
    assert!(alpha.map_to_alpha(&Value::Float(5.0)).is_some());
    assert!(!alpha.breaks().is_empty());
    alpha.set_name("Alpha");
    assert_eq!(alpha.name(), "Alpha");
    let _ = alpha.clone_box();
    alpha.reset_training();
}

#[test]
fn stat_qq_ecdf_edge_cases() {
    use ggplot_rs::scale::ScaleSet;
    use ggplot_rs::stat::qq::{StatQQ, StatQQLine};
    use ggplot_rs::stat::Stat;

    let scales = ScaleSet::new();

    // Empty / missing-column inputs → empty DataFrame, no panic.
    let empty = DataFrame::new();
    assert_eq!(StatQQ.compute_group(&empty, &scales).nrows(), 0);
    assert_eq!(StatQQLine.compute_group(&empty, &scales).nrows(), 0);
    assert_eq!(StatEcdf.compute_group(&empty, &scales).nrows(), 0);

    // No-y column for QQ.
    let mut nox = DataFrame::new();
    nox.add_column("z".into(), vec![Value::Float(1.0)]);
    assert_eq!(StatQQ.compute_group(&nox, &scales).nrows(), 0);

    // QQLine with < 4 values returns empty.
    let mut few = DataFrame::new();
    few.add_column(
        "y".into(),
        vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
    );
    assert_eq!(StatQQLine.compute_group(&few, &scales).nrows(), 0);

    // Normal QQ / QQLine with grouping carryover.
    let mut df = DataFrame::new();
    let ys: Vec<Value> = (0..20).map(|i| Value::Float(i as f64)).collect();
    let xs: Vec<Value> = (0..20).map(|i| Value::Float((20 - i) as f64)).collect();
    df.add_column("x".into(), xs);
    df.add_column("y".into(), ys);
    df.add_column("color".into(), vec![Value::Str("g".into()); 20]);
    let qq = StatQQ.compute_group(&df, &scales);
    assert_eq!(qq.nrows(), 20);
    assert!(qq.has_column("color"));
    let ql = StatQQLine.compute_group(&df, &scales);
    assert_eq!(ql.nrows(), 2);
    assert!(ql.has_column("color"));
    assert_eq!(StatQQ.required_aes().len(), 1);
    assert_eq!(StatQQ.name(), "qq");
    assert_eq!(StatQQLine.name(), "qq_line");

    // StatEcdf normal path + carryover + accessors.
    let ecdf = StatEcdf.compute_group(&df, &scales);
    assert_eq!(ecdf.nrows(), 20);
    assert!(ecdf.has_column("color"));
    let y = ecdf.column("y").unwrap();
    assert_eq!(y[19].as_f64(), Some(1.0));
    assert_eq!(StatEcdf.required_aes().len(), 1);
    assert_eq!(StatEcdf.name(), "ecdf");
    let mut nox2 = DataFrame::new();
    nox2.add_column("q".into(), vec![Value::Float(1.0)]);
    assert_eq!(StatEcdf.compute_group(&nox2, &scales).nrows(), 0);
}

#[test]
fn ecdf_via_geom_step_render() {
    let data = vec![fcol("x", &[3.0, 1.0, 2.0, 5.0, 4.0])];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_step()
        .stat(StatEcdf)
        .layer_aes(Aes::new().x("x").after_stat_y("y"))
        .render_svg();
    assert!(svg.is_ok(), "ecdf step: {svg:?}");
}

// ═══════════════════════════════════════════════════════════════════════
// 9. Faceted rendering with varied geoms (PanelBackendAdapter methods) +
//    themed backgrounds/borders + datetime axis + grouped step.
// ═══════════════════════════════════════════════════════════════════════

fn panel_xy() -> Vec<(String, Vec<Value>)> {
    vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0, 1.0, 2.0, 3.0, 4.0]),
        fcol("y", &[2.0, 4.0, 1.0, 3.0, 3.0, 1.0, 4.0, 2.0]),
        scol("p", &["p1", "p1", "p1", "p1", "p2", "p2", "p2", "p2"]),
    ]
}

#[test]
fn faceted_line_draws_via_adapter() {
    let svg = GGPlot::new(panel_xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .facet_wrap("p", Some(2))
        .render_svg();
    assert!(svg.is_ok(), "faceted line: {svg:?}");
}

#[test]
fn faceted_area_draws_polygon_via_adapter() {
    let svg = GGPlot::new(panel_xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_area()
        .facet_wrap("p", Some(1))
        .render_svg();
    assert!(svg.is_ok(), "faceted area: {svg:?}");
}

#[test]
fn faceted_text_draws_text_via_adapter() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 1.0, 2.0]),
        fcol("y", &[1.0, 2.0, 2.0, 1.0]),
        scol("lab", &["a", "b", "c", "d"]),
        scol("p", &["p1", "p1", "p2", "p2"]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("lab"))
        .geom_text()
        .facet_grid(Some("p"), None)
        .render_svg();
    assert!(svg.is_ok(), "faceted text: {svg:?}");
}

#[test]
fn faceted_boxplot_draws_circles_via_adapter() {
    // Outliers exercise draw_circle inside the panel adapter.
    let ys = [
        10.0, 11.0, 12.0, 13.0, 12.5, 100.0, 10.0, 11.0, 12.0, 13.0, 12.5, -80.0,
    ];
    let panels: Vec<&str> = (0..12).map(|i| if i < 6 { "p1" } else { "p2" }).collect();
    let cats: Vec<&str> = (0..12).map(|_| "A").collect();
    let data = vec![scol("cat", &cats), fcol("y", &ys), scol("p", &panels)];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("cat").y("y"))
        .geom_boxplot()
        .facet_wrap("p", Some(2))
        .render_svg();
    assert!(svg.is_ok(), "faceted boxplot: {svg:?}");
}

#[test]
fn themed_backgrounds_and_borders() {
    for theme_kind in ["bw", "dark", "linedraw", "light", "classic"] {
        let base = GGPlot::new(panel_xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_point();
        let base = match theme_kind {
            "bw" => base.theme_bw(),
            "dark" => base.theme_dark(),
            "linedraw" => base.theme_linedraw(),
            "light" => base.theme_light(),
            _ => base.theme_classic(),
        };
        assert!(base.render_svg().is_ok(), "single {theme_kind}");

        // Same theme, faceted, to hit panel border/background in the facet path.
        let f = GGPlot::new(panel_xy())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .facet_wrap("p", Some(2));
        let f = match theme_kind {
            "bw" => f.theme_bw(),
            "dark" => f.theme_dark(),
            "linedraw" => f.theme_linedraw(),
            "light" => f.theme_light(),
            _ => f.theme_classic(),
        };
        assert!(f.render_svg().is_ok(), "faceted {theme_kind}");
    }
}

#[test]
fn datetime_axis_renders() {
    // Days across ~4 months as epoch seconds.
    let day = 86400.0;
    let xs: Vec<f64> = (0..6).map(|i| i as f64 * 20.0 * day).collect();
    let data = vec![
        (
            "t".to_string(),
            xs.iter().map(|s| Value::DateTime(*s as i64)).collect(),
        ),
        fcol("y", &[1.0, 3.0, 2.0, 5.0, 4.0, 6.0]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("t").y("y"))
        .geom_line()
        .scale_x_datetime(ScaleDateTime::new())
        .render_svg();
    assert!(svg.is_ok(), "datetime axis: {svg:?}");
}

#[test]
fn geom_step_grouped_and_directions() {
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 1.0, 2.0, 3.0]),
        fcol("y", &[1.0, 3.0, 2.0, 2.0, 1.0, 3.0]),
        scol("g", &["a", "a", "a", "b", "b", "b"]),
    ];
    let hv = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_step_with(GeomStep::default())
        .render_svg();
    assert!(hv.is_ok(), "grouped step hv: {hv:?}");

    let vh = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_step_with(GeomStep {
            direction: StepDirection::Vh,
            ..GeomStep::default()
        })
        .render_svg();
    assert!(vh.is_ok(), "grouped step vh: {vh:?}");
}

#[test]
fn sec_axis_with_visible_axis_lines() {
    // theme_classic makes axis lines visible, exercising the sec-y-axis line block.
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0]),
        fcol("y", &[10.0, 20.0, 30.0, 40.0]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .scale_y_continuous(
            ScaleContinuous::new()
                .with_sec_axis(SecAxis::new(|v| v * 1.8 + 32.0).with_name("Fahrenheit")),
        )
        .theme_classic()
        .render_svg();
    assert!(svg.is_ok(), "sec axis + axis lines: {svg:?}");
}

#[test]
fn horizontal_continuous_legends() {
    // Colorbar + size + alpha legends laid out horizontally (legend at Top).
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0]),
        fcol("y", &[2.0, 4.0, 1.0, 3.0]),
        fcol("z", &[10.0, 20.0, 30.0, 40.0]),
    ];
    let colorbar = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .scale_color_viridis_c()
        .theme(Theme::default().set_legend_position(LegendPosition::Top))
        .render_svg();
    assert!(colorbar.is_ok(), "horizontal colorbar: {colorbar:?}");

    let size = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").size("z"))
        .geom_point()
        .theme(Theme::default().set_legend_position(LegendPosition::Bottom))
        .render_svg();
    assert!(size.is_ok(), "horizontal size: {size:?}");

    let alpha = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").alpha("z"))
        .geom_point()
        .theme(Theme::default().set_legend_position(LegendPosition::Top))
        .render_svg();
    assert!(alpha.is_ok(), "horizontal alpha: {alpha:?}");
}

#[test]
fn color_and_fill_both_mapped_dedup() {
    // Mapping the same variable to both color and fill exercises the
    // Fill-dedup branch in the legend collector.
    let data = vec![
        fcol("x", &[1.0, 2.0, 3.0, 4.0]),
        fcol("y", &[2.0, 4.0, 1.0, 3.0]),
        scol("g", &["a", "b", "a", "b"]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g").fill("g"))
        .geom_point()
        .render_svg();
    assert!(svg.is_ok(), "color+fill dedup: {svg:?}");
}

#[test]
fn rotated_x_axis_labels() {
    use ggplot_rs::theme::ThemeUpdate;
    let data = vec![
        scol("cat", &["alpha", "beta", "gamma", "delta"]),
        fcol("y", &[1.0, 2.0, 3.0, 4.0]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("cat").y("y"))
        .geom_col()
        .theme_update(ThemeUpdate {
            axis_text_x: Some(ElementText {
                angle: 45.0,
                ..Default::default()
            }),
            ..Default::default()
        })
        .render_svg();
    assert!(svg.is_ok(), "rotated x labels: {svg:?}");
}

#[test]
fn position_jitter_compute_direct() {
    let pos = PositionJitter {
        width: 0.5,
        height: 0.5,
    };
    let mut df = DataFrame::new();
    df.add_column(
        "x".into(),
        vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
    );
    df.add_column(
        "y".into(),
        vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
    );
    pos.compute(&mut df, &PositionParams::default());
    // All x values should be within the jitter band of their originals.
    let x = df.column("x").unwrap();
    assert!((x[0].as_f64().unwrap() - 1.0).abs() <= 0.5);
    assert_eq!(pos.name(), "jitter");

    // height == 0 skips the y branch.
    let no_h = PositionJitter {
        width: 0.3,
        height: 0.0,
    };
    let mut df2 = DataFrame::new();
    df2.add_column("x".into(), vec![Value::Float(5.0)]);
    df2.add_column("y".into(), vec![Value::Float(9.0)]);
    no_h.compute(&mut df2, &PositionParams::default());
    assert_eq!(df2.column("y").unwrap()[0].as_f64(), Some(9.0));

    let _ = PositionJitter::default();
}

#[test]
fn grouped_freqpoly_and_stacked_histogram() {
    let data = vec![
        fcol(
            "v",
            &[1.0, 1.2, 2.0, 2.1, 3.0, 3.3, 1.1, 2.2, 3.1, 1.3, 2.5, 3.5],
        ),
        scol(
            "g",
            &["a", "a", "a", "b", "b", "b", "a", "b", "a", "b", "a", "b"],
        ),
    ];
    let fp = GGPlot::new(data.clone())
        .aes(Aes::new().x("v").color("g"))
        .geom_freqpoly()
        .render_svg();
    assert!(fp.is_ok(), "grouped freqpoly: {fp:?}");

    let hist = GGPlot::new(data)
        .aes(Aes::new().x("v").fill("g"))
        .geom_histogram()
        .position(PositionStack)
        .render_svg();
    assert!(hist.is_ok(), "stacked histogram: {hist:?}");
}
