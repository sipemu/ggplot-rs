//! #409: `guide_axis_logticks` — log10 minor ticks on a log axis.

use ggplot_rs::prelude::*;

fn plot() -> GGPlot {
    GGPlot::new(vec![
        (
            "x".to_string(),
            [1.0, 10.0, 100.0, 1000.0]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        ),
        (
            "y".to_string(),
            (0..4).map(|i| Value::Float(i as f64)).collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
}

fn tick_lengths(svg: &str) -> Vec<f64> {
    // Vertical tick segments below the panel: `points="x,y0 x,y1"`.
    svg.split("<polyline points=\"")
        .skip(1)
        .filter_map(|t| {
            let p = t.split('"').next()?;
            let pts: Vec<(f64, f64)> = p
                .split(' ')
                .filter_map(|xy| {
                    let (x, y) = xy.split_once(',')?;
                    Some((x.parse().ok()?, y.parse().ok()?))
                })
                .collect();
            (pts.len() == 2 && (pts[0].0 - pts[1].0).abs() < 1e-6)
                .then(|| (pts[1].1 - pts[0].1).abs())
        })
        .filter(|l| *l < 20.0)
        .collect()
}

#[test]
fn log_ticks_long_mid_short() {
    let base = plot()
        .scale_x_log10()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let svg = plot()
        .scale_x_log10()
        .guide_axis_logticks(Aesthetic::X)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let t = 2.75;
    let l = tick_lengths(&svg);
    let count = |k: f64| l.iter().filter(|x| (**x - t * k).abs() < 0.02).count();
    // 1 → 1000 (+ 5 % padding, reaching 0.8): 4 decades (long), 3 fives
    // (mid), 3 × 7 others + 0.8, 0.9 (short).
    assert_eq!(count(2.25), 4, "{l:?}");
    assert_eq!(count(1.5), 3);
    assert_eq!(count(0.75), 23);
    assert!(tick_lengths(&base).iter().all(|x| (x - t).abs() < 0.02));
    // A linear axis ignores it.
    let lin = plot()
        .scale_x_continuous(ScaleContinuous::new().with_log_ticks())
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(tick_lengths(&lin).iter().all(|x| (x - t).abs() < 0.02));
}
