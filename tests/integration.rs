use std::collections::HashMap;
use std::path::Path;

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
