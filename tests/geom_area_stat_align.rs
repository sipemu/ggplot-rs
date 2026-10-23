//! #172: `geom_area()` aligns its series (`stat_align`) before stacking, so a
//! series above one that has no row at some x doesn't dip there. Values from
//! ggplot2 4.0.3: `layer_data(ggplot(d, aes(x, y, fill = g)) + geom_area())`
//! with a = 1 at x 1..4, b = 2 at x 1, 3, 4.
use ggplot_rs::data::DataFrame;
use ggplot_rs::prelude::*;

fn built() -> DataFrame {
    let xs = [1.0, 2.0, 3.0, 4.0, 1.0, 3.0, 4.0];
    let ys = [1.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0];
    let gs = ["a", "a", "a", "a", "b", "b", "b"];
    let p = GGPlot::new(vec![
        (
            "x".to_string(),
            xs.iter().map(|v| Value::Float(*v)).collect(),
        ),
        (
            "y".to_string(),
            ys.iter().map(|v| Value::Float(*v)).collect(),
        ),
        (
            "fill".to_string(),
            gs.iter().map(|s| Value::Str(s.to_string())).collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y").fill("fill"))
    .geom_area();
    p.build().layers[0].data.clone()
}

#[test]
fn stacked_areas_do_not_dip_on_a_gap() {
    let d = built();
    let (x, ymin, y) = (
        d.column("x").unwrap(),
        d.column("ymin").unwrap(),
        d.column("y").unwrap(),
    );
    let fill = d.column("fill").unwrap();
    let mut a_inner = 0;
    for i in 0..d.nrows() {
        let xv = x[i].as_f64().unwrap();
        let inner = (1.0..=4.0).contains(&xv);
        if fill[i].as_str() == Some("a") && inner {
            // ggplot2: a is 2..3 at every inner position, also at x = 2.
            assert!(
                (ymin[i].as_f64().unwrap() - 2.0).abs() < 1e-9
                    && (y[i].as_f64().unwrap() - 3.0).abs() < 1e-9,
                "a at x={xv}: [{:?}, {:?}], want [2, 3]",
                ymin[i],
                y[i]
            );
            a_inner += 1;
        }
    }
    // 1.000, 1.003, 1.997, 2.000, 2.003, 2.997, 3.000, 3.003, 3.997, 4.000
    assert_eq!(a_inner, 10, "{d:?}");
    assert_eq!(d.nrows(), 24, "two groups, 12 aligned rows each (ggplot2)");
}

#[test]
fn a_single_series_area_is_unchanged() {
    let p = GGPlot::new(vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(4.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(3.0), Value::Float(1.0), Value::Float(2.0)],
        ),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_area();
    let d = p.build().layers[0].data.clone();
    assert_eq!(d.nrows(), 3);
}
