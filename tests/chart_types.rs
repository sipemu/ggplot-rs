//! Native chart types: candlestick/OHLC, radar (`coord_radar`), gauge
//! (`CoordPolar::with_span`) and calendar heatmaps (`geom_calendar`).

use std::f64::consts::PI;

use ggplot_rs::aes::Aes;
use ggplot_rs::coord::polar::CoordPolar;
use ggplot_rs::data::Value;
use ggplot_rs::geom::polygon::GeomPolygon;
use ggplot_rs::geom::rect::GeomRect;
use ggplot_rs::geom::segment::GeomSegment;
use ggplot_rs::scale::continuous::ScaleContinuous;
use ggplot_rs::stat::calendar::{days_from_civil, MAX_CALENDAR_YEARS};
use ggplot_rs::GGPlot;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}
fn root_attr<'a>(svg: &'a str, name: &str) -> &'a str {
    let head = &svg[..svg.find('>').unwrap()];
    let key = format!(" {name}=\"");
    let i = head.find(&key).unwrap() + key.len();
    &head[i..i + head[i..].find('"').unwrap()]
}
fn nums(s: &str) -> Vec<f64> {
    s.split(' ').map(|t| t.parse().unwrap()).collect()
}
fn assert_no_nonfinite(svg: &str) {
    assert!(!svg.contains("NaN") && !svg.contains("=\"inf") && !svg.contains("-inf"));
}

fn ohlc() -> Vec<(String, Vec<Value>)> {
    let day = |d: i64| Value::DateTime(days_from_civil(2024, 3, 1 + d as u32) * 86_400);
    frame(vec![
        ("date", (0..4).map(day).collect()),
        ("o", floats(&[10.0, 12.0, 11.0, 11.0])),
        ("h", floats(&[13.0, 12.5, 14.0, 11.0])),
        ("l", floats(&[9.0, 10.0, 10.5, 11.0])),
        ("c", floats(&[12.0, 10.5, 13.5, 11.0])),
    ])
}

fn ohlc_aes() -> Aes {
    Aes::new().x("date").open("o").high("h").low("l").close("c")
}

#[test]
fn candlestick_draws_bodies_wicks_and_trains_on_high_low() {
    let (svg, warnings) = GGPlot::new(ohlc())
        .aes(ohlc_aes())
        .geom_candlestick()
        .title("Prices")
        .render_svg_native_with_warnings(500, 300)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("Prices"));
    // y domain covers [low, high] = [9, 14] (+5%).
    let yd = nums(root_attr(&svg, "data-ydomain"));
    assert!(
        (yd[0] - 8.75).abs() < 1e-9 && (yd[1] - 14.25).abs() < 1e-9,
        "{yd:?}"
    );
    // Up (green) and down (red) candles, with hover metadata.
    assert!(svg.contains("#0CA678") && svg.contains("#E03131"));
    assert!(svg.contains(r#"data-x="2024-03-02" data-series="down" data-value="10.5""#));
    assert!(svg.contains("<title>2024-03-01 — O 10 H 13 L 9 C 12</title>"));
    // A doji (open == close) still gets a visible body.
    assert!(svg.contains(r#"data-series="up" data-value="11""#));
    assert!(
        svg.matches("data-series=\"up\"").count() + svg.matches("data-series=\"down\"").count()
            >= 4
    );
    assert_no_nonfinite(&svg);
}

#[test]
fn ohlc_bars_render_and_missing_close_errors() {
    let svg = GGPlot::new(ohlc())
        .aes(ohlc_aes())
        .geom_ohlc()
        .render_svg_native_with_size(500, 300)
        .unwrap();
    assert!(svg.matches("<polyline").count() >= 12);

    let r = GGPlot::new(ohlc())
        .aes(Aes::new().x("date").open("o").high("h").low("l"))
        .geom_candlestick()
        .render_svg_native_with_size(500, 300);
    assert!(r.is_err(), "close is required");
}

fn radar_data() -> Vec<(String, Vec<Value>)> {
    let axes = ["speed", "power", "range", "cost", "comfort"];
    let mut ax = vec![];
    let mut series = vec![];
    let mut val = vec![];
    for (s, vals) in [
        ("A", [4.0, 3.0, 5.0, 2.0, 4.0]),
        ("B", [2.0, 5.0, 3.0, 4.0, 3.0]),
    ] {
        for (a, v) in axes.iter().zip(vals) {
            ax.push(*a);
            series.push(s);
            val.push(v);
        }
    }
    frame(vec![
        ("axis", strs(&ax)),
        ("series", strs(&series)),
        ("v", floats(&val)),
    ])
}

#[test]
fn radar_draws_closed_polygons_spokes_and_labels() {
    let svg = GGPlot::new(radar_data())
        .aes(Aes::new().x("axis").y("v").color("series").fill("series"))
        .geom_polygon_with(GeomPolygon {
            alpha: 0.2,
            line_width: 1.5,
            ..Default::default()
        })
        .geom_point()
        .coord_radar()
        .title("Radar")
        .render_svg_native_with_size(500, 500)
        .unwrap();
    assert!(svg.contains("Radar"));
    // One polygon per series with 5 vertices, tagged with its series.
    let polys: Vec<&str> = svg
        .split("<polygon ")
        .skip(1)
        .filter(|p| p.contains("data-series"))
        .collect();
    assert_eq!(polys.len(), 2);
    for p in &polys {
        let pts = &p[p.find("points=\"").unwrap() + 8..];
        let pts = &pts[..pts.find('"').unwrap()];
        assert_eq!(pts.split(' ').count(), 5);
    }
    // Spoke labels around the outside, no cartesian axis.
    for a in ["speed", "power", "range", "cost", "comfort"] {
        assert!(svg.contains(&format!(">{a}</text>")));
    }
    // The first spoke points straight up from the centre: "speed" sits above
    // the centre, horizontally centred.
    let p = nums(root_attr(&svg, "data-plot"));
    let (cx, cy) = (p[0] + p[2] / 2.0, p[1] + p[3] / 2.0);
    let at = svg.find(">speed</text>").unwrap();
    let tag = &svg[svg[..at].rfind("<text").unwrap()..at];
    let get = |k: &str| -> f64 {
        let key = format!(" {k}=\"");
        let i = tag.find(&key).unwrap() + key.len();
        tag[i..i + tag[i..].find('"').unwrap()].parse().unwrap()
    };
    assert!((get("x") - cx).abs() < 0.5 && get("y") < cy);
    // Points carry hover metadata for hosts.
    assert!(svg.contains(r#"data-x="cost" data-series="B" data-value="4""#));
    assert_no_nonfinite(&svg);
}

#[test]
fn radar_line_is_closed() {
    let svg = GGPlot::new(radar_data())
        .aes(Aes::new().x("axis").y("v").color("series"))
        .geom_line()
        .coord_radar()
        .render_svg_native_with_size(400, 400)
        .unwrap();
    let lines: Vec<&str> = svg
        .split("<polyline ")
        .skip(1)
        .filter(|p| p.contains("data-series"))
        .collect();
    assert_eq!(lines.len(), 2);
    for l in lines {
        let pts = &l[8..];
        let pts: Vec<&str> = pts[..pts.find('"').unwrap()].split(' ').collect();
        assert_eq!(pts.len(), 6);
        assert_eq!(pts.first(), pts.last());
    }
}

#[test]
fn half_donut_gauge_fills_the_panel() {
    let bands = frame(vec![
        ("x0", floats(&[0.0, 60.0, 85.0])),
        ("x1", floats(&[60.0, 85.0, 100.0])),
        ("y0", floats(&[0.7, 0.7, 0.7])),
        ("y1", floats(&[1.0, 1.0, 1.0])),
        ("zone", strs(&["ok", "warn", "bad"])),
    ]);
    let needle = frame(vec![
        ("v", floats(&[72.0])),
        ("r0", floats(&[0.0])),
        ("r1", floats(&[0.9])),
    ]);
    let svg = GGPlot::new(bands)
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
            color: (30, 30, 30),
            width: 3.0,
            alpha: 1.0,
        })
        .layer_data(needle)
        .layer_aes(Aes::new().x("v").xend("v").y("r0").yend("r1"))
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
        .theme_void()
        .title("Utilisation")
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(svg.contains("Utilisation"));
    let p = nums(root_attr(&svg, "data-plot"));
    // Collect every vertex of the band sectors.
    let mut xs = vec![];
    let mut ys = vec![];
    for poly in svg.split("<polygon ").skip(1) {
        let pts = &poly[poly.find("points=\"").unwrap() + 8..];
        for pt in pts[..pts.find('"').unwrap()].split(' ') {
            let (x, y) = pt.split_once(',').unwrap();
            xs.push(x.parse::<f64>().unwrap());
            ys.push(y.parse::<f64>().unwrap());
        }
    }
    assert_eq!(svg.matches("<polygon ").count(), 3, "three bands");
    let (x0, x1) = xs
        .iter()
        .fold((f64::MAX, f64::MIN), |a, &v| (a.0.min(v), a.1.max(v)));
    let (y0, y1) = ys
        .iter()
        .fold((f64::MAX, f64::MIN), |a, &v| (a.0.min(v), a.1.max(v)));
    // The half circle spans the panel: full width (it's the limiting side of a
    // 2:1 box) or full height, and sits inside it.
    let fits_w = (x1 - x0 - p[2]).abs() < 1.0;
    let fits_h = (y1 - y0 - p[3]).abs() < 1.0;
    assert!(
        fits_w || fits_h,
        "arc {x0}..{x1} × {y0}..{y1} vs panel {p:?}"
    );
    assert!(x0 >= p[0] - 0.5 && x1 <= p[0] + p[2] + 0.5);
    assert!(y0 >= p[1] - 0.5 && y1 <= p[1] + p[3] + 0.5);
    // The needle points up-right (72% of a left→right half circle).
    let seg = svg
        .split("<polyline ")
        .find(|l| l.contains("#1E1E1E"))
        .unwrap();
    let pts = &seg[8..];
    let pts: Vec<(f64, f64)> = pts[..pts.find('"').unwrap()]
        .split(' ')
        .map(|pt| {
            let (x, y) = pt.split_once(',').unwrap();
            (x.parse().unwrap(), y.parse().unwrap())
        })
        .collect();
    assert!(pts[1].0 > pts[0].0 && pts[1].1 < pts[0].1);
    assert_no_nonfinite(&svg);
}

#[test]
fn full_circle_polar_is_unchanged_by_span_fitting() {
    // A full span keeps the classic layout: centred, radius = half the min side.
    let c = CoordPolar::new();
    let area = ggplot_rs::render::Rect {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 100.0,
    };
    use ggplot_rs::coord::Coord;
    let (x, y) = c.transform((0.0, 1.0), &area);
    assert!((x - 110.0).abs() < 1e-9 && (y - 20.0).abs() < 1e-9);
    let (x, y) = c.transform((0.25, 1.0), &area);
    assert!((x - 160.0).abs() < 1e-9 && (y - 70.0).abs() < 1e-9);
}

fn calendar_data(days: &[i64]) -> Vec<(String, Vec<Value>)> {
    frame(vec![
        (
            "date",
            days.iter().map(|d| Value::DateTime(d * 86_400)).collect(),
        ),
        (
            "n",
            floats(&days.iter().map(|d| (d % 7) as f64).collect::<Vec<_>>()),
        ),
    ])
}

#[test]
fn calendar_lays_out_weeks_by_weekdays() {
    let start = days_from_civil(2024, 1, 1); // a Monday
    let days: Vec<i64> = (start..start + 90).collect();
    let (svg, warnings) = GGPlot::new(calendar_data(&days))
        .aes(Aes::new().x("date").fill("n"))
        .geom_calendar()
        .title("Activity")
        .render_svg_native_with_warnings(700, 300)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("Activity"));
    // One tile per day, keyed by its date.
    assert_eq!(svg.matches("data-x=\"2024-").count(), 90);
    assert!(svg.contains(&format!("<title>2024-01-01: {}</title>", start % 7)));
    // Month labels and weekday labels.
    for l in [">Jan<", ">Feb<", ">Mar<", ">Mon<", ">Wed<", ">Fri<"] {
        assert!(svg.contains(l), "missing label {l}");
    }
    // Square cells: equal width and height.
    let tag = svg
        .split("<rect ")
        .find(|t| t.contains("data-x=\"2024-01-02\""))
        .unwrap();
    let get = |k: &str| -> f64 {
        let key = format!("{k}=\"");
        let i = tag.find(&key).unwrap() + key.len();
        tag[i..i + tag[i..].find('"').unwrap()].parse().unwrap()
    };
    assert!((get("width") - get("height")).abs() < 0.05);
    // Month boundaries (Jan|Feb, Feb|Mar) drawn as segments.
    assert!(svg.matches("#78808C").count() >= 4);
    assert_no_nonfinite(&svg);

    // Built data: x = week column, y = weekday row (Sunday on top = 6).
    let built = GGPlot::new(calendar_data(&days))
        .aes(Aes::new().x("date").fill("n"))
        .geom_calendar()
        .try_build()
        .unwrap();
    let d = &built.layers[0].data;
    assert_eq!(d.column("x").unwrap()[0].as_f64(), Some(0.0));
    assert_eq!(d.column("y").unwrap()[0].as_f64(), Some(5.0)); // Monday
}

#[test]
fn calendar_clips_huge_spans_and_accepts_iso_strings() {
    let newest = days_from_civil(2024, 6, 1);
    let mut days = vec![days_from_civil(1, 1, 1), days_from_civil(1800, 1, 1)];
    days.extend(newest - 10..=newest);
    let (svg, warnings) = GGPlot::new(calendar_data(&days))
        .aes(Aes::new().x("date").fill("n"))
        .geom_calendar()
        .render_svg_native_with_warnings(600, 200)
        .unwrap();
    assert!(warnings
        .iter()
        .any(|w| w.contains(&format!("{MAX_CALENDAR_YEARS} years"))));
    assert_eq!(svg.matches("data-x=\"2024-").count(), 11);
    assert_no_nonfinite(&svg);

    let iso = frame(vec![
        (
            "d",
            strs(&["2024-02-28", "2024-02-29", "2024-03-01", "not a date"]),
        ),
        ("n", floats(&[1.0, 2.0, 3.0, 4.0])),
    ]);
    let svg = GGPlot::new(iso)
        .aes(Aes::new().x("d").fill("n"))
        .geom_calendar()
        .render_svg_native_with_size(400, 200)
        .unwrap();
    assert!(svg.contains("data-x=\"2024-02-29\""));
    assert_eq!(svg.matches("data-x=\"2024-").count(), 3);
}

#[test]
fn calendar_without_dates_renders_empty() {
    let data = frame(vec![("d", strs(&["x"])), ("n", floats(&[1.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("d").fill("n"))
        .geom_calendar()
        .title("Empty")
        .render_svg_native_with_size(400, 200)
        .unwrap();
    assert!(svg.contains("Empty"));
}
