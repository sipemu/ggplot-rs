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
