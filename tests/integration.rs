use std::collections::HashMap;
use std::path::Path;

use ggplot_rs::data::DataFrame;
use ggplot_rs::prelude::*;

fn temp_path(name: &str) -> String {
    let dir = std::env::temp_dir();
    dir.join(format!("ggplot_rs_test_{name}"))
        .to_string_lossy()
        .to_string()
}

#[test]
fn smoke_test_scatter_svg() {
    let data = vec![
        HashMap::from([
            ("x".to_string(), Value::Float(1.0)),
            ("y".to_string(), Value::Float(2.0)),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(2.0)),
            ("y".to_string(), Value::Float(4.0)),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(3.0)),
            ("y".to_string(), Value::Float(5.0)),
        ]),
    ];

    let path = temp_path("scatter.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .save(&path)
        .expect("should render without error");

    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    assert!(content.contains("<circle"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn smoke_test_scatter_png() {
    let data = vec![
        HashMap::from([
            ("x".to_string(), Value::Float(1.0)),
            ("y".to_string(), Value::Float(2.0)),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(2.0)),
            ("y".to_string(), Value::Float(4.0)),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(3.0)),
            ("y".to_string(), Value::Float(5.0)),
        ]),
    ];

    let path = temp_path("scatter.png");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("Test Scatter")
        .xlab("X Axis")
        .ylab("Y Axis")
        .theme_minimal()
        .save(&path)
        .expect("should render PNG without error");

    assert!(Path::new(&path).exists());
    let metadata = std::fs::metadata(&path).unwrap();
    assert!(metadata.len() > 100); // Should be a real PNG
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_line_plot() {
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Float(4.0),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(4.0),
                Value::Float(2.0),
                Value::Float(5.0),
            ],
        ),
    ];

    let path = temp_path("line.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .save(&path)
        .expect("should render line plot");

    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_bar_chart() {
    let data = vec![
        HashMap::from([("category".to_string(), Value::Str("A".into()))]),
        HashMap::from([("category".to_string(), Value::Str("A".into()))]),
        HashMap::from([("category".to_string(), Value::Str("B".into()))]),
        HashMap::from([("category".to_string(), Value::Str("B".into()))]),
        HashMap::from([("category".to_string(), Value::Str("B".into()))]),
        HashMap::from([("category".to_string(), Value::Str("C".into()))]),
    ];

    let path = temp_path("bar.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("category"))
        .geom_bar()
        .title("Bar Chart")
        .theme_bw()
        .save(&path)
        .expect("should render bar chart");

    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<rect"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_histogram() {
    let values: Vec<Value> = (0..100).map(|i| Value::Float(i as f64 / 10.0)).collect();

    let data = vec![("x".to_string(), values)];

    let path = temp_path("histogram.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_histogram()
        .xlab("Value")
        .ylab("Count")
        .save(&path)
        .expect("should render histogram");

    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<rect"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_boxplot() {
    // Create data with groups
    let mut rows = Vec::new();
    for group in ["A", "B", "C"] {
        for i in 0..20 {
            let base = match group {
                "A" => 10.0,
                "B" => 20.0,
                _ => 15.0,
            };
            rows.push(HashMap::from([
                ("group".to_string(), Value::Str(group.to_string())),
                ("value".to_string(), Value::Float(base + (i as f64) * 0.5)),
            ]));
        }
    }

    let path = temp_path("boxplot.svg");
    GGPlot::new(rows)
        .aes(Aes::new().x("group").y("value"))
        .geom_boxplot()
        .title("Boxplot")
        .save(&path)
        .expect("should render boxplot");

    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_smooth() {
    let n = 50;
    let x_vals: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y_vals: Vec<Value> = (0..n)
        .map(|i| Value::Float(2.0 * i as f64 + 5.0 + (i as f64 * 0.3).sin() * 3.0))
        .collect();

    let data = vec![("x".to_string(), x_vals), ("y".to_string(), y_vals)];

    let path = temp_path("smooth.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth()
        .title("Scatter with Smooth")
        .save(&path)
        .expect("should render smooth plot");

    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<circle")); // points
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_colored_scatter() {
    let mut rows = Vec::new();
    for i in 0..30 {
        let group = if i % 3 == 0 {
            "A"
        } else if i % 3 == 1 {
            "B"
        } else {
            "C"
        };
        rows.push(HashMap::from([
            ("x".to_string(), Value::Float(i as f64)),
            ("y".to_string(), Value::Float((i as f64 * 0.5).sin() * 10.0)),
            ("species".to_string(), Value::Str(group.to_string())),
        ]));
    }

    let path = temp_path("colored_scatter.svg");
    GGPlot::new(rows)
        .aes(Aes::new().x("x").y("y").color("species"))
        .geom_point()
        .title("Colored Scatter")
        .theme_minimal()
        .save(&path)
        .expect("should render colored scatter");

    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_custom_size() {
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(3.0), Value::Float(4.0)]),
    ];

    let path = temp_path("custom_size.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .save_with_size(&path, 1200, 900)
        .expect("should render with custom size");

    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("1200"));
    assert!(content.contains("900"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_multiple_layers() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(2.0), Value::Float(4.0), Value::Float(6.0)],
        ),
    ];

    let path = temp_path("multi_layer.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_line()
        .title("Points + Line")
        .save(&path)
        .expect("should render multi-layer plot");

    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

// ─── New geom tests ──────────────────────────────────────────

fn xy_data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Float(4.0),
                Value::Float(5.0),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(2.0),
                Value::Float(4.0),
                Value::Float(3.0),
                Value::Float(5.0),
                Value::Float(1.0),
            ],
        ),
    ]
}

#[test]
fn test_geom_jitter() {
    let path = temp_path("jitter.svg");
    GGPlot::new(xy_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_jitter()
        .save(&path)
        .expect("should render jitter");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_path() {
    let path = temp_path("path.svg");
    GGPlot::new(xy_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_path()
        .save(&path)
        .expect("should render path");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_step() {
    let path = temp_path("step.svg");
    GGPlot::new(xy_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_step()
        .save(&path)
        .expect("should render step");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_freqpoly() {
    let values: Vec<Value> = (0..100).map(|i| Value::Float(i as f64 / 10.0)).collect();
    let data = vec![("x".to_string(), values)];
    let path = temp_path("freqpoly.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_freqpoly()
        .save(&path)
        .expect("should render freqpoly");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_linerange() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "ymin".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(1.5)],
        ),
        (
            "ymax".to_string(),
            vec![Value::Float(3.0), Value::Float(5.0), Value::Float(4.0)],
        ),
    ];
    let path = temp_path("linerange.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").ymin("ymin").ymax("ymax"))
        .geom_linerange()
        .save(&path)
        .expect("should render linerange");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_pointrange() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(2.0), Value::Float(3.5), Value::Float(2.5)],
        ),
        (
            "ymin".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(1.5)],
        ),
        (
            "ymax".to_string(),
            vec![Value::Float(3.0), Value::Float(5.0), Value::Float(4.0)],
        ),
    ];
    let path = temp_path("pointrange.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").ymin("ymin").ymax("ymax"))
        .geom_pointrange()
        .save(&path)
        .expect("should render pointrange");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_crossbar() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(2.0), Value::Float(3.5), Value::Float(2.5)],
        ),
        (
            "ymin".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(1.5)],
        ),
        (
            "ymax".to_string(),
            vec![Value::Float(3.0), Value::Float(5.0), Value::Float(4.0)],
        ),
    ];
    let path = temp_path("crossbar.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").ymin("ymin").ymax("ymax"))
        .geom_crossbar()
        .save(&path)
        .expect("should render crossbar");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_spoke() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "angle".to_string(),
            vec![Value::Float(0.0), Value::Float(1.5), Value::Float(3.0)],
        ),
        (
            "radius".to_string(),
            vec![Value::Float(0.5), Value::Float(0.5), Value::Float(0.5)],
        ),
    ];
    let path = temp_path("spoke.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").angle("angle").radius("radius"))
        .geom_spoke()
        .save(&path)
        .expect("should render spoke");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_rect() {
    let data = vec![
        (
            "xmin".to_string(),
            vec![Value::Float(1.0), Value::Float(3.0)],
        ),
        (
            "xmax".to_string(),
            vec![Value::Float(2.0), Value::Float(5.0)],
        ),
        (
            "ymin".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0)],
        ),
        (
            "ymax".to_string(),
            vec![Value::Float(3.0), Value::Float(4.0)],
        ),
    ];
    let path = temp_path("rect.svg");
    GGPlot::new(data)
        .aes(
            Aes::new()
                .xmin("xmin")
                .xmax("xmax")
                .ymin("ymin")
                .ymax("ymax"),
        )
        .geom_rect()
        .save(&path)
        .expect("should render rect");
    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<rect"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_tile() {
    let path = temp_path("tile.svg");
    GGPlot::new(xy_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_tile()
        .save(&path)
        .expect("should render tile");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_polygon() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(0.0), Value::Float(1.0), Value::Float(0.5)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(0.0), Value::Float(0.0), Value::Float(1.0)],
        ),
        (
            "group".to_string(),
            vec![
                Value::Str("a".to_string()),
                Value::Str("a".to_string()),
                Value::Str("a".to_string()),
            ],
        ),
    ];
    let path = temp_path("polygon.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").group("group"))
        .geom_polygon()
        .save(&path)
        .expect("should render polygon");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_curve() {
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(3.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(3.0)]),
        (
            "xend".to_string(),
            vec![Value::Float(2.0), Value::Float(5.0)],
        ),
        (
            "yend".to_string(),
            vec![Value::Float(3.0), Value::Float(1.0)],
        ),
    ];
    let path = temp_path("curve.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
        .geom_curve()
        .save(&path)
        .expect("should render curve");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_violin() {
    let mut rows = Vec::new();
    for group in ["A", "B"] {
        let base = if group == "A" { 10.0 } else { 20.0 };
        for i in 0..30 {
            rows.push(HashMap::from([
                ("group".to_string(), Value::Str(group.to_string())),
                ("value".to_string(), Value::Float(base + (i as f64) * 0.5)),
            ]));
        }
    }
    let path = temp_path("violin.svg");
    GGPlot::new(rows)
        .aes(Aes::new().x("group").y("value"))
        .geom_violin()
        .save(&path)
        .expect("should render violin");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_dotplot() {
    let values: Vec<Value> = (0..50).map(|i| Value::Float(i as f64 / 5.0)).collect();
    let data = vec![("x".to_string(), values)];
    let path = temp_path("dotplot.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_dotplot()
        .save(&path)
        .expect("should render dotplot");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_qq() {
    let y_vals: Vec<Value> = (0..100).map(|i| Value::Float(i as f64)).collect();
    let data = vec![("sample".to_string(), y_vals)];
    let path = temp_path("qq.svg");
    GGPlot::new(data)
        .aes(Aes::new().y("sample"))
        .geom_qq()
        .geom_qq_line()
        .save(&path)
        .expect("should render qq plot");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_bin2d() {
    let n = 200;
    let x_vals: Vec<Value> = (0..n)
        .map(|i| Value::Float((i as f64 * 0.1).sin()))
        .collect();
    let y_vals: Vec<Value> = (0..n)
        .map(|i| Value::Float((i as f64 * 0.1).cos()))
        .collect();
    let data = vec![("x".to_string(), x_vals), ("y".to_string(), y_vals)];
    let path = temp_path("bin2d.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_bin2d()
        .save(&path)
        .expect("should render bin2d");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_hex() {
    let n = 200;
    let x_vals: Vec<Value> = (0..n)
        .map(|i| Value::Float((i as f64 * 0.1).sin()))
        .collect();
    let y_vals: Vec<Value> = (0..n)
        .map(|i| Value::Float((i as f64 * 0.1).cos()))
        .collect();
    let data = vec![("x".to_string(), x_vals), ("y".to_string(), y_vals)];
    let path = temp_path("hex.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_hex()
        .save(&path)
        .expect("should render hex");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

// ─── Feature: Layer-level stat/position override ─────────────

#[test]
fn test_layer_stat_override() {
    // Use geom_bar (default stat=StatCount) but override to StatIdentity
    // to provide pre-computed y values
    let data = vec![
        (
            "category".to_string(),
            vec![
                Value::Str("A".into()),
                Value::Str("B".into()),
                Value::Str("C".into()),
            ],
        ),
        (
            "count".to_string(),
            vec![Value::Float(10.0), Value::Float(20.0), Value::Float(15.0)],
        ),
    ];
    let path = temp_path("stat_override.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("category").y("count"))
        .geom_col()
        .stat(StatIdentity)
        .save(&path)
        .expect("should render with stat override");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_layer_position_override() {
    // Use geom_bar (default position=PositionStack) but override to PositionDodge
    let mut rows = Vec::new();
    for x in ["a", "b", "c"] {
        for fill in ["g1", "g2"] {
            rows.push(HashMap::from([
                ("x".to_string(), Value::Str(x.into())),
                ("fill".to_string(), Value::Str(fill.into())),
            ]));
        }
    }
    let path = temp_path("position_override.svg");
    GGPlot::new(rows)
        .aes(Aes::new().x("x").fill("fill"))
        .geom_bar()
        .position(PositionDodge)
        .save(&path)
        .expect("should render with position override");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_layer_position_override_build() {
    // Verify that position override actually changes computed data
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(1.0)]),
        ("y".to_string(), vec![Value::Float(3.0), Value::Float(2.0)]),
        (
            "fill".to_string(),
            vec![Value::Str("g1".into()), Value::Str("g2".into())],
        ),
    ];
    // Default: geom_col uses PositionIdentity → y values unchanged
    let built_default = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").fill("fill"))
        .geom_col()
        .build();

    // Override: PositionStack → y should be cumulative
    let built_stacked = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("fill"))
        .geom_col()
        .position(PositionStack)
        .build();

    let default_y: Vec<f64> = built_default.layers[0]
        .data
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let stacked_y: Vec<f64> = built_stacked.layers[0]
        .data
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();

    // Stacked y should have at least one value > any default y
    let default_max = default_y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let stacked_max = stacked_y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        stacked_max > default_max,
        "stacking should produce larger y: default_max={default_max}, stacked_max={stacked_max}"
    );
}

#[test]
fn test_layer_data_override() {
    // Use different data per layer
    let base_data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(4.0)]),
    ];
    let overlay_data: DataFrame = vec![
        ("x".to_string(), vec![Value::Float(1.5), Value::Float(2.5)]),
        ("y".to_string(), vec![Value::Float(3.0), Value::Float(2.0)]),
    ]
    .into_dataframe();

    let path = temp_path("layer_data.svg");
    GGPlot::new(base_data)
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .geom_point()
        .layer_data(overlay_data)
        .save(&path)
        .expect("should render with layer data override");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

// ─── Feature: Date/time support ──────────────────────────────

#[test]
fn test_datetime_values() {
    // 2024-01-01 through 2024-01-05 (epoch seconds)
    let base = 1704067200_i64; // 2024-01-01 00:00:00 UTC
    let day = 86400_i64;
    let data = vec![
        (
            "date".to_string(),
            vec![
                Value::DateTime(base),
                Value::DateTime(base + day),
                Value::DateTime(base + 2 * day),
                Value::DateTime(base + 3 * day),
                Value::DateTime(base + 4 * day),
            ],
        ),
        (
            "price".to_string(),
            vec![
                Value::Float(100.0),
                Value::Float(102.5),
                Value::Float(101.0),
                Value::Float(105.0),
                Value::Float(103.5),
            ],
        ),
    ];
    let path = temp_path("datetime.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("date").y("price"))
        .geom_line()
        .geom_point()
        .title("Stock Price")
        .save(&path)
        .expect("should render date/time plot");
    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    // Should have date-formatted axis labels
    assert!(
        content.contains("2024"),
        "axis labels should contain date strings"
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_datetime_auto_scale() {
    // Verify that DateTime values auto-detect ScaleDateTime
    let base = 1704067200_i64;
    let day = 86400_i64;
    let data = vec![
        (
            "x".to_string(),
            vec![Value::DateTime(base), Value::DateTime(base + 30 * day)],
        ),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .build();

    // X scale breaks should be date-formatted
    let x_scale = built
        .scales
        .get(&Aesthetic::X)
        .expect("should have X scale");
    let breaks = x_scale.breaks();
    assert!(!breaks.is_empty(), "datetime scale should have breaks");
    // Check that at least one label looks like a date
    assert!(
        breaks.iter().any(|(_, label)| label.contains("2024")),
        "datetime breaks should contain year: {:?}",
        breaks
    );
}

// ─── Feature: coord_polar ────────────────────────────────────

#[test]
fn test_coord_polar_pie() {
    // Pie chart: bars in polar coordinates
    let data = vec![(
        "category".to_string(),
        vec![
            Value::Str("A".into()),
            Value::Str("A".into()),
            Value::Str("A".into()),
            Value::Str("B".into()),
            Value::Str("B".into()),
            Value::Str("C".into()),
        ],
    )];
    let path = temp_path("polar_pie.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("category"))
        .geom_bar()
        .coord_polar()
        .save(&path)
        .expect("should render polar pie chart");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_coord_polar_with_config() {
    let data = xy_data();
    let path = temp_path("polar_config.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_polar_with(
            CoordPolar::new()
                .theta("y")
                .start(std::f64::consts::FRAC_PI_2),
        )
        .save(&path)
        .expect("should render polar with config");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_coord_polar_transform() {
    use ggplot_rs::coord::Coord;
    use ggplot_rs::render::Rect;

    let coord = CoordPolar::new();
    let area = Rect {
        x: 0.0,
        y: 0.0,
        width: 200.0,
        height: 200.0,
    };

    // Center point: radius=0 → should be at center
    let (cx, cy) = coord.transform((0.0, 0.0), &area);
    assert!((cx - 100.0).abs() < 1.0, "center x: {cx}");
    assert!((cy - 100.0).abs() < 1.0, "center y: {cy}");

    // Full radius at angle 0 (12 o'clock) → should be at top center
    let (px, py) = coord.transform((0.0, 1.0), &area);
    assert!((px - 100.0).abs() < 1.0, "top x: {px}");
    assert!(py < 10.0, "top y should be near 0: {py}");
}

// ─── Feature: Continuous legend (guide_colorbar) ─────────────

#[test]
fn test_continuous_color_legend() {
    // Scatter plot with continuous color → should draw a colorbar legend
    let n = 20;
    let data = vec![
        (
            "x".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..n)
                .map(|i| Value::Float((i as f64 * 0.3).sin()))
                .collect(),
        ),
        (
            "value".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
    ];
    let path = temp_path("continuous_legend.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("value"))
        .geom_point()
        .save(&path)
        .expect("should render with continuous legend");
    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    // Should have many rect elements (the gradient bar slices)
    let rect_count = content.matches("<rect").count();
    assert!(
        rect_count > 10,
        "continuous legend should have gradient slices, found {rect_count} rects"
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_continuous_fill_legend() {
    // Bin2d with continuous fill → should draw a colorbar
    let n = 100;
    let data = vec![
        (
            "x".to_string(),
            (0..n)
                .map(|i| Value::Float((i as f64 * 0.1).sin()))
                .collect(),
        ),
        (
            "y".to_string(),
            (0..n)
                .map(|i| Value::Float((i as f64 * 0.1).cos()))
                .collect(),
        ),
    ];
    let path = temp_path("continuous_fill.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_bin2d()
        .save(&path)
        .expect("should render bin2d with continuous fill legend");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

// ─── Feature: Size/alpha continuous mapping ──────────────────

#[test]
fn test_size_mapping() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "pop".to_string(),
            vec![
                Value::Float(100.0),
                Value::Float(500.0),
                Value::Float(1000.0),
            ],
        ),
    ];
    let path = temp_path("size_mapping.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").size("pop"))
        .geom_point()
        .save(&path)
        .expect("should render with size mapping");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_size_scale_values() {
    // Verify that ScaleSizeContinuous actually maps values to different sizes
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        (
            "size".to_string(),
            vec![Value::Float(10.0), Value::Float(100.0)],
        ),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").size("size"))
        .geom_point()
        .build();

    let size_scale = built.scales.get(&Aesthetic::Size);
    assert!(size_scale.is_some(), "should have a size scale");
    let s = size_scale.unwrap();
    let small = s.map_to_size(&Value::Float(10.0)).unwrap();
    let large = s.map_to_size(&Value::Float(100.0)).unwrap();
    assert!(large > small, "larger value should map to larger size");
}

#[test]
fn test_alpha_mapping() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "density".to_string(),
            vec![Value::Float(0.1), Value::Float(0.5), Value::Float(1.0)],
        ),
    ];
    let path = temp_path("alpha_mapping.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").alpha("density"))
        .geom_point()
        .save(&path)
        .expect("should render with alpha mapping");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_alpha_scale_values() {
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        (
            "alpha".to_string(),
            vec![Value::Float(0.0), Value::Float(1.0)],
        ),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").alpha("alpha"))
        .geom_point()
        .build();

    let alpha_scale = built.scales.get(&Aesthetic::Alpha);
    assert!(alpha_scale.is_some(), "should have an alpha scale");
    let s = alpha_scale.unwrap();
    let lo = s.map_to_alpha(&Value::Float(0.0)).unwrap();
    let hi = s.map_to_alpha(&Value::Float(1.0)).unwrap();
    assert!(hi > lo, "higher value should have higher alpha");
    assert!((0.0..=1.0).contains(&lo), "alpha should be in [0,1]");
    assert!((0.0..=1.0).contains(&hi), "alpha should be in [0,1]");
}

#[test]
fn test_custom_size_range() {
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        (
            "size".to_string(),
            vec![Value::Float(1.0), Value::Float(10.0)],
        ),
    ];
    let path = temp_path("custom_size_range.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").size("size"))
        .geom_point()
        .scale_size(ScaleSizeContinuous::new().with_range(2.0, 10.0))
        .save(&path)
        .expect("should render with custom size range");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

// ─── Feature: Custom breaks/labels ──────────────────────────

#[test]
fn test_custom_breaks() {
    let data = xy_data();
    let path = temp_path("custom_breaks.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_continuous(ScaleContinuous::new().with_breaks(vec![1.0, 5.0, 10.0]))
        .save(&path)
        .expect("should render with custom breaks");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_custom_breaks_with_labels() {
    let data = xy_data();
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_continuous(
            ScaleContinuous::new()
                .with_breaks(vec![1.0, 5.0, 10.0])
                .with_labels(vec!["Low".into(), "Mid".into(), "High".into()]),
        )
        .build();

    let x_scale = built.scales.get(&Aesthetic::X).unwrap();
    let breaks = x_scale.breaks();
    let labels: Vec<&str> = breaks.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(labels, vec!["Low", "Mid", "High"]);
}

// ─── Feature: Secondary axes ────────────────────────────────

#[test]
fn test_sec_axis_render() {
    let data = vec![
        (
            "celsius".to_string(),
            vec![
                Value::Float(0.0),
                Value::Float(10.0),
                Value::Float(20.0),
                Value::Float(30.0),
            ],
        ),
        (
            "time".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Float(4.0),
            ],
        ),
    ];
    let path = temp_path("sec_axis.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("time").y("celsius"))
        .geom_line()
        .scale_y_continuous(
            ScaleContinuous::new()
                .with_sec_axis(SecAxis::new(|c| c * 9.0 / 5.0 + 32.0).with_name("Fahrenheit")),
        )
        .save(&path)
        .expect("should render with secondary axis");
    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("Fahrenheit"),
        "should contain sec axis title"
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_sec_axis_transform() {
    let sec = SecAxis::new(|x| x * 2.0);
    assert!((sec.transform_value(5.0) - 10.0).abs() < f64::EPSILON);
    assert!((sec.transform_value(0.0) - 0.0).abs() < f64::EPSILON);
}

// ─── Feature: after_stat() computed aesthetics ──────────────

#[test]
fn test_after_stat_density() {
    // Use after_stat to map density (from StatBin) to y
    let data = vec![(
        "x".to_string(),
        (0..100)
            .map(|i| Value::Float(i as f64 / 10.0))
            .collect::<Vec<_>>(),
    )];
    let path = temp_path("after_stat.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").after_stat_y("density"))
        .geom_histogram()
        .save(&path)
        .expect("should render histogram with after_stat(density)");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_after_stat_build() {
    // Verify after_stat(density) produces density values on y
    let data = vec![(
        "x".to_string(),
        (0..100)
            .map(|i| Value::Float(i as f64 / 10.0))
            .collect::<Vec<_>>(),
    )];
    let built = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").after_stat_y("density"))
        .geom_histogram()
        .build();

    let y_vals: Vec<f64> = built.layers[0]
        .data
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();

    // density values should be < 1 for uniform data over [0, 10]
    let max_y = y_vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        max_y < 1.0,
        "density values should be < 1 for uniform data: max_y={max_y}"
    );

    // Compare with default (count)
    let built_count = GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_histogram()
        .build();
    let count_max: f64 = built_count.layers[0]
        .data
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .fold(f64::NEG_INFINITY, f64::max);

    assert!(
        count_max > max_y,
        "count values should be larger than density values"
    );
}

// ─── Feature: position_dodge2 and position_nudge ─────────────

#[test]
fn test_position_dodge2() {
    let mut rows = Vec::new();
    for x in ["a", "b", "c"] {
        for fill in ["g1", "g2"] {
            rows.push(HashMap::from([
                ("x".to_string(), Value::Str(x.into())),
                ("fill".to_string(), Value::Str(fill.into())),
            ]));
        }
    }
    let path = temp_path("dodge2.svg");
    GGPlot::new(rows)
        .aes(Aes::new().x("x").fill("fill"))
        .geom_bar()
        .position(PositionDodge2::default())
        .save(&path)
        .expect("should render with position_dodge2");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_position_nudge() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "label".to_string(),
            vec![
                Value::Str("A".into()),
                Value::Str("B".into()),
                Value::Str("C".into()),
            ],
        ),
    ];
    let path = temp_path("nudge.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_point()
        .geom_text()
        .position(PositionNudge::new(0.0, 0.2))
        .save(&path)
        .expect("should render with position_nudge");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_position_nudge_build() {
    // Verify nudge actually shifts positions
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ];
    let nudge_x = 0.5;
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .position(PositionNudge::new(nudge_x, 0.0))
        .build();

    let x_vals: Vec<f64> = built.layers[0]
        .data
        .column("x")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();

    assert!(
        (x_vals[0] - 1.5).abs() < f64::EPSILON,
        "x[0] should be nudged to 1.5, got {}",
        x_vals[0]
    );
    assert!(
        (x_vals[1] - 2.5).abs() < f64::EPSILON,
        "x[1] should be nudged to 2.5, got {}",
        x_vals[1]
    );
}

// ─── Tier 2: Multi-aesthetic legends ────────────────────────

#[test]
fn test_shape_legend() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "species".to_string(),
            vec![
                Value::Str("cat".into()),
                Value::Str("dog".into()),
                Value::Str("bird".into()),
            ],
        ),
    ];
    let path = temp_path("shape_legend.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").shape("species"))
        .geom_point()
        .save(&path)
        .expect("should render with shape legend");
    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    // Legend should contain the species labels
    assert!(content.contains("cat"), "legend should show 'cat'");
    assert!(content.contains("dog"), "legend should show 'dog'");
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_linetype_legend() {
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(1.0),
                Value::Float(2.0),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(2.0),
                Value::Float(3.0),
            ],
        ),
        (
            "group".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("b".into()),
            ],
        ),
    ];
    let path = temp_path("linetype_legend.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").linetype("group"))
        .geom_line()
        .save(&path)
        .expect("should render with linetype legend");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_size_legend() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "weight".to_string(),
            vec![Value::Float(10.0), Value::Float(50.0), Value::Float(100.0)],
        ),
    ];
    let path = temp_path("size_legend.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").size("weight"))
        .geom_point()
        .save(&path)
        .expect("should render with size legend");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

// ─── Tier 2: coord_cartesian zoom ───────────────────────────

#[test]
fn test_coord_cartesian_zoom_render() {
    let data = xy_data();
    let path = temp_path("coord_zoom.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_cartesian_zoom(Some((2.0, 8.0)), Some((1.0, 5.0)))
        .save(&path)
        .expect("should render with coord_cartesian zoom");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_coord_cartesian_zoom_preserves_data() {
    // coord_cartesian zoom should NOT filter data (unlike xlim/ylim)
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(5.0), Value::Float(10.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(5.0), Value::Float(10.0)],
        ),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_cartesian_zoom(Some((2.0, 8.0)), None)
        .build();

    // All 3 data points should still be present
    let nrows = built.layers[0].data.nrows();
    assert_eq!(nrows, 3, "zoom should not filter rows, got {nrows}");
}

// ─── Tier 2: Theme composition ──────────────────────────────

#[test]
fn test_theme_update_render() {
    let data = xy_data();
    let path = temp_path("theme_update.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme_minimal()
        .theme_update(ThemeUpdate {
            title: Some(ElementText {
                color: (255, 0, 0),
                size: 20.0,
                ..Default::default()
            }),
            ..Default::default()
        })
        .title("Red Title")
        .save(&path)
        .expect("should render with theme update");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_theme_update_preserves_base() {
    // theme_update should only override specified fields
    let base = ggplot_rs::prelude::theme_minimal();
    let original_axis_text_size = base.axis_text_x.size;

    let updated = base.update(ThemeUpdate {
        title: Some(ElementText {
            color: (255, 0, 0),
            size: 24.0,
            ..Default::default()
        }),
        ..Default::default()
    });

    assert_eq!(
        updated.axis_text_x.size, original_axis_text_size,
        "non-updated fields should be preserved"
    );
    assert_eq!(updated.title.size, 24.0, "updated field should change");
    assert_eq!(
        updated.title.color,
        (255, 0, 0),
        "updated color should change"
    );
}

// ─── Tier 2: Discrete scale ordering/limits ─────────────────

#[test]
fn test_discrete_scale_limits() {
    let data = vec![(
        "x".to_string(),
        vec![
            Value::Str("c".into()),
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("a".into()),
        ],
    )];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_bar()
        .scale_x_discrete(ScaleDiscrete::new().with_limits(vec!["b", "a"]))
        .build();

    let x_scale = built.scales.get(&Aesthetic::X).unwrap();
    let breaks = x_scale.breaks();
    let labels: Vec<&str> = breaks.iter().map(|(_, l)| l.as_str()).collect();
    // Only "b" and "a" should appear, in that order
    assert_eq!(
        labels,
        vec!["b", "a"],
        "limits should control order: {labels:?}"
    );
}

#[test]
fn test_discrete_scale_custom_labels() {
    let data = vec![(
        "x".to_string(),
        vec![
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("c".into()),
        ],
    )];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_bar()
        .scale_x_discrete(ScaleDiscrete::new().with_labels(vec![
            "Alpha".into(),
            "Beta".into(),
            "Gamma".into(),
        ]))
        .build();

    let x_scale = built.scales.get(&Aesthetic::X).unwrap();
    let breaks = x_scale.breaks();
    let labels: Vec<&str> = breaks.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(labels, vec!["Alpha", "Beta", "Gamma"]);
}

// ─── Tier 2: position_jitterdodge ───────────────────────────

#[test]
fn test_position_jitterdodge_render() {
    let mut rows = Vec::new();
    for x_val in [1.0, 2.0, 3.0] {
        for fill in ["g1", "g2"] {
            for _ in 0..5 {
                rows.push(HashMap::from([
                    ("x".to_string(), Value::Float(x_val)),
                    ("y".to_string(), Value::Float(x_val + 1.0)),
                    ("fill".to_string(), Value::Str(fill.into())),
                ]));
            }
        }
    }
    let path = temp_path("jitterdodge.svg");
    GGPlot::new(rows)
        .aes(Aes::new().x("x").y("y").fill("fill"))
        .geom_point()
        .position(PositionJitterDodge::new(0.2, 0.0))
        .save(&path)
        .expect("should render with position_jitterdodge");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_position_jitterdodge_build() {
    // Verify that jitterdodge both dodges groups apart and jitters within
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(1.0),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Float(4.0),
            ],
        ),
        (
            "fill".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("b".into()),
            ],
        ),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("fill"))
        .geom_point()
        .position(PositionJitterDodge::new(0.3, 0.0).with_seed(42))
        .build();

    let x_vals: Vec<f64> = built.layers[0]
        .data
        .column("x")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();

    // All x values started at 1.0 — after dodge+jitter they should be different
    let all_same = x_vals.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-10);
    assert!(!all_same, "jitterdodge should produce different x values");

    // Group "a" (indices 0,1) should have different mean x from group "b" (indices 2,3)
    let mean_a = (x_vals[0] + x_vals[1]) / 2.0;
    let mean_b = (x_vals[2] + x_vals[3]) / 2.0;
    assert!(
        (mean_a - mean_b).abs() > 0.1,
        "dodge should separate groups: mean_a={mean_a}, mean_b={mean_b}"
    );
}

// ─── Tier 2: Legend position variants ───────────────────────

#[test]
fn test_legend_position_bottom() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "color".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        ),
    ];
    let path = temp_path("legend_bottom.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("color"))
        .geom_point()
        .theme(Theme::default().set_legend_position(LegendPosition::Bottom))
        .save(&path)
        .expect("should render with bottom legend");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_legend_position_top() {
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        (
            "color".to_string(),
            vec![Value::Str("x".into()), Value::Str("y".into())],
        ),
    ];
    let path = temp_path("legend_top.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("color"))
        .geom_point()
        .theme(Theme::default().set_legend_position(LegendPosition::Top))
        .save(&path)
        .expect("should render with top legend");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_legend_position_left() {
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        (
            "color".to_string(),
            vec![Value::Str("x".into()), Value::Str("y".into())],
        ),
    ];
    let path = temp_path("legend_left.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("color"))
        .geom_point()
        .theme(Theme::default().set_legend_position(LegendPosition::Left))
        .save(&path)
        .expect("should render with left legend");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_legend_position_none() {
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        (
            "color".to_string(),
            vec![Value::Str("x".into()), Value::Str("y".into())],
        ),
    ];
    let path = temp_path("legend_none.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("color"))
        .geom_point()
        .theme(Theme::default().set_legend_position(LegendPosition::None))
        .save(&path)
        .expect("should render with no legend");
    assert!(Path::new(&path).exists());
    // Just verify it renders without error
    std::fs::remove_file(&path).ok();
}

// ─── Tier 3: Scale fill/reverse convenience methods ──────────

#[test]
fn test_scale_fill_gradient() {
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(3.0), Value::Float(4.0)]),
        (
            "fill".to_string(),
            vec![Value::Float(0.0), Value::Float(1.0)],
        ),
    ];
    let path = temp_path("fill_gradient.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("fill"))
        .geom_point()
        .scale_fill_gradient(
            RGBAColor {
                r: 255,
                g: 255,
                b: 255,
                a: 1.0,
            },
            RGBAColor {
                r: 0,
                g: 0,
                b: 255,
                a: 1.0,
            },
        )
        .save(&path)
        .expect("should render with fill gradient");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_scale_fill_gradient2() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(3.0), Value::Float(4.0), Value::Float(5.0)],
        ),
        (
            "fill".to_string(),
            vec![Value::Float(-1.0), Value::Float(0.0), Value::Float(1.0)],
        ),
    ];
    let path = temp_path("fill_gradient2.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("fill"))
        .geom_point()
        .scale_fill_gradient2(
            RGBAColor {
                r: 255,
                g: 0,
                b: 0,
                a: 1.0,
            },
            RGBAColor {
                r: 255,
                g: 255,
                b: 255,
                a: 1.0,
            },
            RGBAColor {
                r: 0,
                g: 0,
                b: 255,
                a: 1.0,
            },
        )
        .save(&path)
        .expect("should render with fill gradient2");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_scale_fill_viridis() {
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        ),
        (
            "fill".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        ),
    ];
    let path = temp_path("fill_viridis.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").fill("fill"))
        .geom_bar()
        .scale_fill_viridis()
        .save(&path)
        .expect("should render with fill viridis");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_scale_fill_brewer() {
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        ),
        (
            "fill".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        ),
    ];
    let path = temp_path("fill_brewer.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").fill("fill"))
        .geom_bar()
        .scale_fill_brewer(PaletteName::Set3)
        .save(&path)
        .expect("should render with fill brewer");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_scale_x_reverse() {
    let data = xy_data();
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_reverse()
        .build();

    // With reverse transform, data values get negated in the pipeline.
    // So original x=1 becomes -1, x=10 becomes -10.
    // When mapped, -1 (originally small x) should be at a higher position than -10.
    let x_scale = built.scales.get(&Aesthetic::X).unwrap();
    let pos_neg1 = x_scale.map(&Value::Float(-1.0));
    let pos_neg10 = x_scale.map(&Value::Float(-10.0));
    assert!(
        pos_neg1 > pos_neg10,
        "reverse: -1 (orig 1) should map higher than -10 (orig 10), got {pos_neg1} vs {pos_neg10}"
    );
}

// ─── Tier 3: Label formatters ───────────────────────────────

#[test]
fn test_label_formatter_comma() {
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Float(1000.0),
                Value::Float(5000.0),
                Value::Float(10000.0),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(1000.0),
                Value::Float(50000.0),
                Value::Float(100000.0),
            ],
        ),
    ];
    let path = temp_path("formatter_comma.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_continuous(ScaleContinuous::new().with_label_formatter(label_comma))
        .save(&path)
        .expect("should render with comma formatter");
    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    // Labels should have commas
    assert!(
        content.contains(','),
        "comma formatter should produce commas in labels"
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_label_formatter_percent() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(0.1), Value::Float(0.5), Value::Float(0.9)],
        ),
    ];
    let path = temp_path("formatter_percent.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_continuous(ScaleContinuous::new().with_label_formatter(label_percent))
        .save(&path)
        .expect("should render with percent formatter");
    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains('%'),
        "percent formatter should produce % in labels"
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_label_formatter_dollar() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(1000.0),
                Value::Float(2000.0),
                Value::Float(3000.0),
            ],
        ),
    ];
    let path = temp_path("formatter_dollar.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_continuous(ScaleContinuous::new().with_label_formatter(label_dollar))
        .save(&path)
        .expect("should render with dollar formatter");
    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains('$'),
        "dollar formatter should produce $ in labels"
    );
    std::fs::remove_file(&path).ok();
}

// ─── Tier 3: StatBin binwidth parameter ─────────────────────

#[test]
fn test_histogram_binwidth() {
    let data = vec![(
        "x".to_string(),
        (0..100).map(|i| Value::Float(i as f64 / 10.0)).collect(),
    )];
    let path = temp_path("hist_binwidth.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_histogram_with(GeomHistogram::default().with_binwidth(1.0))
        .save(&path)
        .expect("should render with binwidth");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_histogram_binwidth_build() {
    let data = vec![(
        "x".to_string(),
        (0..100).map(|i| Value::Float(i as f64 / 10.0)).collect(),
    )];
    // With binwidth=2.0, range 0-9.9 should give ~5 bins
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_histogram_with(GeomHistogram::default().with_binwidth(2.0))
        .build();

    let nrows = built.layers[0].data.nrows();
    assert!(
        nrows == 5,
        "binwidth=2.0 over range 0-9.9 should give 5 bins, got {nrows}"
    );
}

#[test]
fn test_histogram_custom_bins() {
    let data = vec![(
        "x".to_string(),
        (0..100).map(|i| Value::Float(i as f64)).collect(),
    )];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_histogram_with(GeomHistogram::default().with_bins(10))
        .build();

    let nrows = built.layers[0].data.nrows();
    assert_eq!(nrows, 10, "should have exactly 10 bins, got {nrows}");
}

// ─── Tier 3: Expanded palettes ──────────────────────────────

#[test]
fn test_palette_set3() {
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        ),
        (
            "color".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        ),
    ];
    let path = temp_path("palette_set3.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").color("color"))
        .geom_bar()
        .scale_color_brewer(PaletteName::Set3)
        .save(&path)
        .expect("should render with Set3 palette");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_palette_magma() {
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        ),
        (
            "color".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        ),
    ];
    let path = temp_path("palette_magma.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").color("color"))
        .geom_bar()
        .scale_color_brewer(PaletteName::Magma)
        .save(&path)
        .expect("should render with Magma palette");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

// ─── Tier 3: Guide configuration ───────────────────────────

#[test]
fn test_guide_title_override() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "color".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        ),
    ];
    let path = temp_path("guide_title.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("color"))
        .geom_point()
        .guides(GuideLegend::new().with_title("My Legend"))
        .save(&path)
        .expect("should render with guide title override");
    assert!(Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("My Legend"),
        "guide title should appear in SVG"
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_guide_reverse() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "color".to_string(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        ),
    ];
    let path = temp_path("guide_reverse.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("color"))
        .geom_point()
        .guides(GuideLegend::new().reverse())
        .save(&path)
        .expect("should render with reversed legend");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

// ─── Tier 3: Text geom hjust/vjust ─────────────────────────

#[test]
fn test_geom_text_hjust() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "label".to_string(),
            vec![
                Value::Str("left".into()),
                Value::Str("center".into()),
                Value::Str("right".into()),
            ],
        ),
    ];
    let path = temp_path("text_hjust.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText::default().with_hjust(0.0))
        .save(&path)
        .expect("should render with left-aligned text");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_label_hjust_fontfamily() {
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        (
            "label".to_string(),
            vec![Value::Str("A".into()), Value::Str("B".into())],
        ),
    ];
    let path = temp_path("label_hjust.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label_with(
            GeomLabel::default()
                .with_hjust(1.0)
                .with_fontfamily("monospace"),
        )
        .save(&path)
        .expect("should render with right-aligned labels");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

// ─── Tier 4: Auto-group by discrete X ──────────────────────

#[test]
fn test_auto_group_discrete_x() {
    // Boxplot with discrete x should auto-group without explicit group column
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Str("A".into()),
                Value::Str("A".into()),
                Value::Str("A".into()),
                Value::Str("A".into()),
                Value::Str("A".into()),
                Value::Str("B".into()),
                Value::Str("B".into()),
                Value::Str("B".into()),
                Value::Str("B".into()),
                Value::Str("B".into()),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Float(4.0),
                Value::Float(5.0),
                Value::Float(10.0),
                Value::Float(20.0),
                Value::Float(30.0),
                Value::Float(40.0),
                Value::Float(50.0),
            ],
        ),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_boxplot()
        .build();

    // Should have at least 2 rows (one per group) from boxplot stat
    assert!(
        built.layers[0].data.nrows() >= 2,
        "auto-grouping by discrete x should produce multiple groups, got {} rows",
        built.layers[0].data.nrows()
    );
}

// ─── Tier 4: Log2 and Ln transforms ────────────────────────

#[test]
fn test_scale_x_log2() {
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(4.0),
                Value::Float(8.0),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Float(4.0),
            ],
        ),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_log2()
        .build();

    let x_scale = built.scales.get(&Aesthetic::X).unwrap();
    // log2(8) = 3.0, log2(1) = 0.0
    let v8 = x_scale.map(&Value::Float(8.0_f64.log2()));
    let v1 = x_scale.map(&Value::Float(1.0_f64.log2()));
    assert!(v8 > v1, "log2(8) should map higher than log2(1)");
}

#[test]
fn test_scale_y_ln() {
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(std::f64::consts::E)],
        ),
    ];
    let path = temp_path("scale_y_ln.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_ln()
        .save(&path)
        .expect("should render with ln-transformed y axis");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

// ─── Tier 4: geom_count / stat_sum ─────────────────────────

#[test]
fn test_geom_count_render() {
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(2.0),
                Value::Float(3.0),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(2.0),
                Value::Float(3.0),
            ],
        ),
    ];
    let path = temp_path("geom_count.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_count()
        .save(&path)
        .expect("should render geom_count");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_stat_sum_build() {
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(2.0),
                Value::Float(3.0),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(2.0),
                Value::Float(3.0),
            ],
        ),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_count()
        .build();

    let layer = &built.layers[0].data;
    // 3 unique (x,y) pairs: (1,1)x3, (2,2)x2, (3,3)x1
    assert_eq!(layer.nrows(), 3, "stat_sum should produce 3 unique groups");
    let n_col = layer.column("n").expect("should have n column");
    let counts: Vec<f64> = n_col.iter().filter_map(|v| v.as_f64()).collect();
    assert!(
        counts.contains(&3.0),
        "should have count 3 for (1,1), got {:?}",
        counts
    );
    assert!(
        counts.contains(&2.0),
        "should have count 2 for (2,2), got {:?}",
        counts
    );
    assert!(
        counts.contains(&1.0),
        "should have count 1 for (3,3), got {:?}",
        counts
    );
}

// ─── Tier 4: geom_contour / stat_contour ───────────────────

#[test]
fn test_geom_contour_render() {
    // Generate gridded z = x^2 + y^2 data
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
    let data = vec![
        ("x".to_string(), xs),
        ("y".to_string(), ys),
        ("z".to_string(), zs),
    ];
    let path = temp_path("geom_contour.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_contour()
        .save(&path)
        .expect("should render contour lines");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_stat_contour_build() {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut zs = Vec::new();
    for ix in 0..20 {
        for iy in 0..20 {
            let x = ix as f64;
            let y = iy as f64;
            xs.push(Value::Float(x));
            ys.push(Value::Float(y));
            zs.push(Value::Float(x * x + y * y));
        }
    }
    let data = vec![
        ("x".to_string(), xs),
        ("y".to_string(), ys),
        ("z".to_string(), zs),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_contour()
        .build();

    let layer = &built.layers[0].data;
    // Should produce contour line segments
    assert!(
        layer.nrows() > 0,
        "stat_contour should produce contour line data"
    );
    assert!(
        layer.column("level").is_some(),
        "contour data should have level column"
    );
    assert!(
        layer.column("group").is_some(),
        "contour data should have group column"
    );
}

// ─── Tier 4: Plot panel clipping ────────────────────────────

#[test]
fn test_clipping_out_of_bounds() {
    // Data extending well beyond coord limits should still render without panic
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Float(-100.0),
                Value::Float(0.0),
                Value::Float(5.0),
                Value::Float(200.0),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(-50.0),
                Value::Float(0.0),
                Value::Float(5.0),
                Value::Float(100.0),
            ],
        ),
    ];
    let path = temp_path("clipping.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_line()
        .coord_cartesian_zoom(Some((0.0, 10.0)), Some((0.0, 10.0)))
        .save(&path)
        .expect("should render with clipping for out-of-bounds data");
    assert!(Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}

// ─── Tier 4: stat_summary_bin ───────────────────────────────

#[test]
fn test_stat_summary_bin_build() {
    let data = vec![
        (
            "x".to_string(),
            (0..100)
                .map(|i| Value::Float(i as f64 / 10.0))
                .collect::<Vec<_>>(),
        ),
        (
            "y".to_string(),
            (0..100)
                .map(|i| Value::Float((i as f64 / 10.0).sin()))
                .collect::<Vec<_>>(),
        ),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .stat(StatSummaryBin::default().with_bins(5))
        .build();

    let layer = &built.layers[0].data;
    // 5 bins, some may be empty but we should have at least a few rows
    assert!(
        layer.nrows() >= 3,
        "stat_summary_bin should produce binned summary rows, got {}",
        layer.nrows()
    );
    assert!(layer.column("ymin").is_some(), "should have ymin column");
    assert!(layer.column("ymax").is_some(), "should have ymax column");
}

#[test]
fn test_stat_summary_bin_mean() {
    // 2 bins, each with known y values
    let data = vec![
        (
            "x".to_string(),
            vec![
                Value::Float(0.0),
                Value::Float(0.1),
                Value::Float(0.2),
                Value::Float(0.5),
                Value::Float(0.6),
                Value::Float(0.7),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(10.0),
                Value::Float(20.0),
                Value::Float(30.0),
                Value::Float(40.0),
                Value::Float(50.0),
                Value::Float(60.0),
            ],
        ),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .stat(StatSummaryBin::default().with_bins(2))
        .build();

    let layer = &built.layers[0].data;
    let y_col = layer.column("y").expect("should have y column");
    let y_vals: Vec<f64> = y_col.iter().filter_map(|v| v.as_f64()).collect();
    // Bin 1: x in [0, 0.35) -> y = [10, 20, 30] -> mean = 20
    // Bin 2: x in [0.35, 0.7] -> y = [40, 50, 60] -> mean = 50
    assert_eq!(y_vals.len(), 2, "should have 2 bins");
    assert!(
        (y_vals[0] - 20.0).abs() < 1e-6,
        "first bin mean should be 20, got {}",
        y_vals[0]
    );
    assert!(
        (y_vals[1] - 50.0).abs() < 1e-6,
        "second bin mean should be 50, got {}",
        y_vals[1]
    );
}

// ─── Tier 5 tests ────────────────────────────────────────────────

#[test]
fn test_rect_clip_false_backgrounds_render() {
    // Verify that plot background and strip rects with clip=false render outside plot area
    let data = vec![
        HashMap::from([
            ("x".to_string(), Value::Str("A".to_string())),
            ("y".to_string(), Value::Float(1.0)),
            ("grp".to_string(), Value::Str("g1".to_string())),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Str("B".to_string())),
            ("y".to_string(), Value::Float(2.0)),
            ("grp".to_string(), Value::Str("g2".to_string())),
        ]),
    ];

    let path = temp_path("clip_false_bg.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_col()
        .facet_wrap("grp", Some(2))
        .theme_bw()
        .title("Background Test")
        .save(&path)
        .expect("should render without error");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    // With clip=false, backgrounds should render as full rectangles
    assert!(content.contains("<rect"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_scale_expand_zero() {
    // Verify expand(0, 0) produces tighter mapping
    use ggplot_rs::data::Value;
    use ggplot_rs::scale::Scale;

    let mut s = ScaleContinuous::new()
        .for_aesthetic(Aesthetic::X)
        .with_expand(0.0, 0.0);
    s.train(&[Value::Float(0.0), Value::Float(10.0)]);

    // With expand(0,0), map(0) should be exactly 0.0 and map(10) exactly 1.0
    let v0 = s.map(&Value::Float(0.0));
    let v1 = s.map(&Value::Float(10.0));
    assert!(
        (v0 - 0.0).abs() < 1e-10,
        "expand(0,0): map(min) should be 0.0, got {v0}"
    );
    assert!(
        (v1 - 1.0).abs() < 1e-10,
        "expand(0,0): map(max) should be 1.0, got {v1}"
    );
}

#[test]
fn test_scale_expand_default() {
    // Default expand is (0.05, 0) — so map(min) > 0 and map(max) < 1
    use ggplot_rs::data::Value;
    use ggplot_rs::scale::Scale;

    let mut s = ScaleContinuous::new().for_aesthetic(Aesthetic::X);
    s.train(&[Value::Float(0.0), Value::Float(10.0)]);

    let v0 = s.map(&Value::Float(0.0));
    let v1 = s.map(&Value::Float(10.0));
    // With 5% expand, min maps to ~0.045 and max to ~0.955
    assert!(
        v0 > 0.0,
        "default expand: map(min) should be > 0.0, got {v0}"
    );
    assert!(
        v1 < 1.0,
        "default expand: map(max) should be < 1.0, got {v1}"
    );
}

#[test]
fn test_gradient_n_viridis_continuous_color_mapping() {
    use ggplot_rs::data::Value;
    use ggplot_rs::scale::Scale;

    let mut g = ScaleColorGradientN::viridis(Aesthetic::Color);
    g.train(&[Value::Float(0.0), Value::Float(100.0)]);

    // Map min value -> should be first viridis color (dark purple)
    let c_min = g.map_to_color(&Value::Float(0.0)).unwrap();
    assert_eq!(c_min, (68, 1, 84), "min should map to viridis start");

    // Map max value -> should be last viridis color (yellow)
    let c_max = g.map_to_color(&Value::Float(100.0)).unwrap();
    assert_eq!(c_max, (253, 231, 37), "max should map to viridis end");

    // Mid value should be a middle green-ish color
    let c_mid = g.map_to_color(&Value::Float(50.0)).unwrap();
    assert!(
        c_mid.1 > 100,
        "mid value should have green component > 100, got {:?}",
        c_mid
    );
}

#[test]
fn test_gradient_n_custom_stops() {
    use ggplot_rs::data::Value;
    use ggplot_rs::scale::Scale;

    let mut g = ScaleColorGradientN::new(
        Aesthetic::Fill,
        vec![
            (0.0, RGBAColor::new(0, 0, 0)),     // black
            (0.5, RGBAColor::new(255, 0, 0)),   // red
            (1.0, RGBAColor::new(255, 255, 0)), // yellow
        ],
    );
    g.train(&[Value::Float(0.0), Value::Float(1.0)]);

    let c0 = g.map_to_color(&Value::Float(0.0)).unwrap();
    assert_eq!(c0, (0, 0, 0));

    let c_mid = g.map_to_color(&Value::Float(0.5)).unwrap();
    assert_eq!(c_mid, (255, 0, 0));

    let c1 = g.map_to_color(&Value::Float(1.0)).unwrap();
    assert_eq!(c1, (255, 255, 0));
}

#[test]
fn test_scale_fill_viridis_c_renders() {
    let data = vec![
        HashMap::from([
            ("x".to_string(), Value::Float(1.0)),
            ("y".to_string(), Value::Float(2.0)),
            ("z".to_string(), Value::Float(10.0)),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(2.0)),
            ("y".to_string(), Value::Float(3.0)),
            ("z".to_string(), Value::Float(50.0)),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(3.0)),
            ("y".to_string(), Value::Float(1.0)),
            ("z".to_string(), Value::Float(90.0)),
        ]),
    ];

    let path = temp_path("viridis_c.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .scale_color_viridis_c()
        .save(&path)
        .expect("viridis_c should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    assert!(content.contains("<circle"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_subtitle_caption_in_faceted_plot() {
    let data = vec![
        HashMap::from([
            ("x".to_string(), Value::Float(1.0)),
            ("y".to_string(), Value::Float(2.0)),
            ("grp".to_string(), Value::Str("A".to_string())),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(2.0)),
            ("y".to_string(), Value::Float(3.0)),
            ("grp".to_string(), Value::Str("B".to_string())),
        ]),
    ];

    let path = temp_path("subtitle_caption_faceted.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .facet_wrap("grp", Some(2))
        .title("Main Title")
        .subtitle("A subtitle here")
        .caption("Source: test data")
        .save(&path)
        .expect("faceted with subtitle/caption should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("Main Title"));
    assert!(content.contains("A subtitle here"));
    assert!(content.contains("Source: test data"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_subtitle_caption_layout_reserves_space() {
    let data = vec![HashMap::from([
        ("x".to_string(), Value::Float(1.0)),
        ("y".to_string(), Value::Float(2.0)),
    ])];

    // Without subtitle/caption
    let path1 = temp_path("no_subtitle.svg");
    GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("Title")
        .save(&path1)
        .expect("should render");

    // With subtitle and caption
    let path2 = temp_path("with_subtitle.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("Title")
        .subtitle("Subtitle")
        .caption("Caption")
        .save(&path2)
        .expect("should render");

    // Both should produce valid SVGs
    let c1 = std::fs::read_to_string(&path1).unwrap();
    let c2 = std::fs::read_to_string(&path2).unwrap();
    assert!(c1.contains("<svg"));
    assert!(c2.contains("<svg"));
    assert!(c2.contains("Subtitle"));
    assert!(c2.contains("Caption"));
    std::fs::remove_file(&path1).ok();
    std::fs::remove_file(&path2).ok();
}

#[test]
fn test_font_family_passthrough() {
    let data = vec![HashMap::from([
        ("x".to_string(), Value::Float(1.0)),
        ("y".to_string(), Value::Float(2.0)),
    ])];

    let mut custom_theme = theme_bw();
    custom_theme.title.family = "serif".to_string();
    custom_theme.axis_text_x.family = "monospace".to_string();

    let path = temp_path("font_family.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme(custom_theme)
        .title("Serif Title")
        .save(&path)
        .expect("font family should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    // SVG should contain the font family references
    assert!(
        content.contains("serif") || content.contains("Serif"),
        "SVG should reference serif font"
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_text_rotation_270_y_axis() {
    let data = vec![HashMap::from([
        ("x".to_string(), Value::Float(1.0)),
        ("y".to_string(), Value::Float(2.0)),
    ])];

    let path = temp_path("rotation_270.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .ylab("Y Axis (rotated)")
        .save(&path)
        .expect("270° rotation should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    // The Y axis title uses 270° rotation
    assert!(content.contains("Y Axis"), "should contain Y axis label");
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_text_rotation_45_x_axis() {
    let data = vec![
        HashMap::from([
            ("x".to_string(), Value::Str("Long Label A".to_string())),
            ("y".to_string(), Value::Float(1.0)),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Str("Long Label B".to_string())),
            ("y".to_string(), Value::Float(2.0)),
        ]),
    ];

    let mut custom_theme = theme_bw();
    custom_theme.axis_text_x.angle = 45.0;

    let path = temp_path("rotation_45.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_col()
        .theme(custom_theme)
        .save(&path)
        .expect("45° x-axis rotation should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    assert!(content.contains("Long Label A"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_text_rotation_90() {
    let data = vec![HashMap::from([
        ("x".to_string(), Value::Str("Cat".to_string())),
        ("y".to_string(), Value::Float(5.0)),
    ])];

    let mut custom_theme = theme_bw();
    custom_theme.axis_text_x.angle = 90.0;

    let path = temp_path("rotation_90.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_col()
        .theme(custom_theme)
        .save(&path)
        .expect("90° rotation should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_scale_expand_zero_heatmap() {
    // Edge-to-edge rendering with expand(0, 0)
    let mut data = vec![];
    for x in 0..3 {
        for y in 0..3 {
            data.push(HashMap::from([
                ("x".to_string(), Value::Float(x as f64)),
                ("y".to_string(), Value::Float(y as f64)),
                ("z".to_string(), Value::Float((x + y) as f64)),
            ]));
        }
    }

    let path = temp_path("heatmap_expand0.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("z"))
        .geom_tile()
        .scale_x_continuous(ScaleContinuous::new().with_expand(0.0, 0.0))
        .scale_y_continuous(ScaleContinuous::new().with_expand(0.0, 0.0))
        .scale_fill_viridis_c()
        .save(&path)
        .expect("heatmap with expand(0,0) should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    assert!(content.contains("<rect"));
    std::fs::remove_file(&path).ok();
}

// ─── Tier 6 tests ──────────────────────────────────────────────────────

#[test]
fn test_geom_blank_extends_axis() {
    // geom_blank() with data that extends x beyond the main layer should train scales
    let main_data = vec![
        HashMap::from([
            ("x".to_string(), Value::Float(1.0)),
            ("y".to_string(), Value::Float(2.0)),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(3.0)),
            ("y".to_string(), Value::Float(4.0)),
        ]),
    ];

    let blank_data = vec![HashMap::from([
        ("x".to_string(), Value::Float(0.0)),
        ("y".to_string(), Value::Float(10.0)),
    ])];

    let path = temp_path("geom_blank.svg");
    GGPlot::new(main_data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_blank()
        .layer_data(blank_data)
        .save(&path)
        .expect("geom_blank should render without error");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    // Only the geom_point circles should appear (no drawing from geom_blank)
    assert!(content.contains("<circle"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_blank_build_trains_scales() {
    let main_data = vec![HashMap::from([
        ("x".to_string(), Value::Float(1.0)),
        ("y".to_string(), Value::Float(2.0)),
    ])];
    let blank_data = vec![HashMap::from([
        ("x".to_string(), Value::Float(0.0)),
        ("y".to_string(), Value::Float(10.0)),
    ])];

    let built = GGPlot::new(main_data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_blank()
        .layer_data(blank_data)
        .build();

    // Y scale should include 10.0 from blank layer
    let y_breaks = built.scales.get(&Aesthetic::Y).unwrap().breaks();
    let max_break_val: f64 = y_breaks
        .iter()
        .filter_map(|(_, label)| label.parse::<f64>().ok())
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(max_break_val >= 10.0, "Y axis should extend to at least 10");
}

#[test]
fn test_scale_color_grey() {
    let data = vec![
        HashMap::from([
            ("x".to_string(), Value::Float(1.0)),
            ("y".to_string(), Value::Float(2.0)),
            ("grp".to_string(), Value::Str("A".to_string())),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(2.0)),
            ("y".to_string(), Value::Float(3.0)),
            ("grp".to_string(), Value::Str("B".to_string())),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(3.0)),
            ("y".to_string(), Value::Float(1.0)),
            ("grp".to_string(), Value::Str("C".to_string())),
        ]),
    ];

    let path = temp_path("scale_grey.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("grp"))
        .geom_point()
        .scale_color_grey()
        .save(&path)
        .expect("scale_color_grey should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    assert!(content.contains("<circle"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_scale_fill_grey_custom_range() {
    let data = vec![
        HashMap::from([
            ("x".to_string(), Value::Str("A".to_string())),
            ("y".to_string(), Value::Float(5.0)),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Str("B".to_string())),
            ("y".to_string(), Value::Float(3.0)),
        ]),
    ];

    let path = temp_path("scale_fill_grey_custom.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("x"))
        .geom_col()
        .scale_fill_grey_with(ScaleColorGrey::new(Aesthetic::Fill).with_range(0.0, 1.0))
        .save(&path)
        .expect("scale_fill_grey custom range should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    assert!(content.contains("<rect"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_xlim_filters_data_before_stat() {
    // With xlim(2, 4), data outside [2,4] should be removed before stat_smooth
    let data: Vec<HashMap<String, Value>> = (0..10)
        .map(|i| {
            let x = i as f64;
            HashMap::from([
                ("x".to_string(), Value::Float(x)),
                ("y".to_string(), Value::Float(x * 2.0)),
            ])
        })
        .collect();

    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .xlim(2.0, 7.0)
        .build();

    // The point layer data should only contain rows where x is in [2, 7]
    let layer_data = &built.layers[0].data;
    if let Some(x_col) = layer_data.column("x") {
        for v in x_col {
            let f = v.as_f64().unwrap();
            assert!(
                (2.0..=7.0).contains(&f),
                "x={f} should be filtered out by xlim(2,7)"
            );
        }
    }
}

#[test]
fn test_coord_cartesian_does_not_filter() {
    // coord_cartesian zoom should NOT filter data
    let data: Vec<HashMap<String, Value>> = (0..10)
        .map(|i| {
            let x = i as f64;
            HashMap::from([
                ("x".to_string(), Value::Float(x)),
                ("y".to_string(), Value::Float(x * 2.0)),
            ])
        })
        .collect();

    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_cartesian_zoom(Some((2.0, 7.0)), None)
        .build();

    // All 10 rows should still be in the data
    assert_eq!(built.layers[0].data.nrows(), 10);
}

#[test]
fn test_facet_labeller_both() {
    let data = vec![
        HashMap::from([
            ("x".to_string(), Value::Float(1.0)),
            ("y".to_string(), Value::Float(2.0)),
            ("grp".to_string(), Value::Str("A".to_string())),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(2.0)),
            ("y".to_string(), Value::Float(3.0)),
            ("grp".to_string(), Value::Str("B".to_string())),
        ]),
    ];

    let path = temp_path("facet_labeller_both.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .facet_wrap_labeller("grp", Some(2), FacetLabeller::Both)
        .save(&path)
        .expect("facet labeller Both should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    // Strip labels should contain "grp: A" and "grp: B"
    assert!(
        content.contains("grp: A"),
        "Should have 'grp: A' strip label"
    );
    assert!(
        content.contains("grp: B"),
        "Should have 'grp: B' strip label"
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_facet_labeller_custom() {
    let data = vec![
        HashMap::from([
            ("x".to_string(), Value::Float(1.0)),
            ("y".to_string(), Value::Float(2.0)),
            ("grp".to_string(), Value::Str("A".to_string())),
        ]),
        HashMap::from([
            ("x".to_string(), Value::Float(2.0)),
            ("y".to_string(), Value::Float(3.0)),
            ("grp".to_string(), Value::Str("B".to_string())),
        ]),
    ];

    fn my_labeller(_var: &str, val: &str) -> String {
        format!("Group {val}")
    }

    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .facet_wrap_labeller("grp", Some(2), FacetLabeller::Custom(my_labeller))
        .build();

    assert_eq!(built.panels[0].label, "Group A");
    assert_eq!(built.panels[1].label, "Group B");
}

#[test]
fn test_check_overlap_text() {
    // Many overlapping labels — check_overlap should produce valid SVG
    let data: Vec<HashMap<String, Value>> = (0..20)
        .map(|i| {
            HashMap::from([
                ("x".to_string(), Value::Float(1.0)), // all same position
                ("y".to_string(), Value::Float(1.0)),
                ("label".to_string(), Value::Str(format!("Label{i}"))),
            ])
        })
        .collect();

    let path = temp_path("check_overlap.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText::default().with_check_overlap(true))
        .save(&path)
        .expect("check_overlap text should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    // With check_overlap, only the first label should appear (all at same position)
    assert!(content.contains("Label0"));
    // Count how many <text> elements appear — should be fewer than 20
    let text_count = content.matches("<text").count();
    // At least title/axis labels exist, but data labels should be just 1
    // We'll just check it's much less than 20 + overhead
    assert!(
        text_count < 25,
        "check_overlap should reduce text count, got {text_count}"
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_geom_smooth_with_color_groups() {
    let mut data = Vec::new();
    for grp in ["A", "B"] {
        for i in 0..10 {
            let x = i as f64;
            let y = if grp == "A" { x * 2.0 } else { x * 0.5 + 5.0 };
            data.push(HashMap::from([
                ("x".to_string(), Value::Float(x)),
                ("y".to_string(), Value::Float(y)),
                ("grp".to_string(), Value::Str(grp.to_string())),
            ]));
        }
    }

    let path = temp_path("smooth_color_groups.svg");
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("grp"))
        .geom_point()
        .geom_smooth()
        .save(&path)
        .expect("geom_smooth with color groups should render");

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    // Should have polylines/paths for the smooth lines
    assert!(
        content.contains("<polyline") || content.contains("<path"),
        "Should have smooth lines rendered"
    );
    std::fs::remove_file(&path).ok();
}
