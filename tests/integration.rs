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
