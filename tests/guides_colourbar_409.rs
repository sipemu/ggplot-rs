//! #409: `guide_colourbar` options (bar size, nbin, display, frame, ticks),
//! `guide_coloursteps`, and per-aesthetic `reverse`.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let n = 20;
    vec![
        ("x".into(), (0..n).map(|i| Value::Float(i as f64)).collect()),
        ("y".into(), (0..n).map(|i| Value::Float(i as f64)).collect()),
        (
            "z".into(),
            (0..n).map(|i| Value::Float(i as f64 * 5.0)).collect(),
        ),
        (
            "g".into(),
            (0..n)
                .map(|i| Value::Str(["a", "b", "c"][i % 3].into()))
                .collect(),
        ),
    ]
}

fn plot() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
}

fn svg(p: GGPlot) -> String {
    p.render_svg_native_with_size(500, 400).unwrap()
}

/// Every `<rect>` with no stroke whose x lies right of the panel: the
/// colourbar's blocks, as `(x, y, w, h)`.
fn bar_rects(svg: &str) -> Vec<(f64, f64, f64, f64)> {
    let attr = |tag: &str, name: &str| -> Option<f64> {
        let a = tag.find(&format!(" {name}=\""))? + name.len() + 3;
        tag[a..].split('"').next()?.parse().ok()
    };
    svg.split("<rect")
        .skip(1)
        .filter_map(|t| {
            let t = &t[..t.find('>').unwrap_or(t.len())];
            Some((
                attr(t, "x")?,
                attr(t, "y")?,
                attr(t, "width")?,
                attr(t, "height")?,
            ))
        })
        .filter(|r| r.0 > 300.0 && r.2 <= 60.0 && r.3 < 250.0)
        .collect()
}

fn extent(r: &[(f64, f64, f64, f64)]) -> (f64, f64) {
    let x0 = r.iter().map(|r| r.0).fold(f64::MAX, f64::min);
    let x1 = r.iter().map(|r| r.0 + r.2).fold(f64::MIN, f64::max);
    let y0 = r.iter().map(|r| r.1).fold(f64::MAX, f64::min);
    let y1 = r.iter().map(|r| r.1 + r.3).fold(f64::MIN, f64::max);
    (x1 - x0, y1 - y0)
}

#[test]
fn nbin_and_rectangles_set_the_block_count() {
    let default = bar_rects(&svg(plot()));
    assert_eq!(default.len(), 50, "{default:?}");
    let seven = bar_rects(&svg(plot().guide_colourbar(
        Aesthetic::Color,
        GuideColourbar::new()
            .with_nbin(7)
            .with_display(ColourbarDisplay::Rectangles),
    )));
    assert_eq!(seven.len(), 7);
    // Rectangles meet edge to edge: they tile the bar exactly.
    let total: f64 = seven.iter().map(|r| r.3).sum();
    assert!((total - extent(&seven).1).abs() < 0.05, "{seven:?}");
}

#[test]
fn bar_width_and_height_resize_the_bar_and_its_reserved_band() {
    let base = svg(plot());
    let big = svg(plot().guide_colourbar(
        Aesthetic::Color,
        GuideColourbar::new()
            .with_bar_width(40.0)
            .with_bar_height(200.0),
    ));
    let (w, h) = extent(&bar_rects(&big));
    assert!(
        (w - 40.0).abs() < 1e-6 && (h - 200.0).abs() < 1.0,
        "{w} x {h}"
    );
    // A wider bar widens the right legend band, so the panel shrinks.
    let panel_w = |s: &str| -> f64 {
        let t = s.split("<rect").nth(2).unwrap();
        let a = t.find(" width=\"").unwrap() + 8;
        t[a..].split('"').next().unwrap().parse().unwrap()
    };
    assert!(
        panel_w(&big) < panel_w(&base),
        "{} vs {}",
        panel_w(&big),
        panel_w(&base)
    );
}

#[test]
fn ticks_and_frame_can_be_switched_off() {
    let lines = |s: &str| s.matches("<line").count() + s.matches("<polyline").count();
    let base = svg(plot());
    let no_ticks =
        svg(plot().guide_colourbar(Aesthetic::Color, GuideColourbar::new().with_ticks(false)));
    let no_frame =
        svg(plot().guide_colourbar(Aesthetic::Color, GuideColourbar::new().with_frame(false)));
    assert!(lines(&no_ticks) < lines(&base), "ticks dropped");
    assert!(lines(&no_frame) <= lines(&base));
    assert_ne!(no_frame, base);
}

#[test]
fn coloursteps_draws_one_block_per_interval() {
    let s = svg(plot().guide_coloursteps(Aesthetic::Color));
    let blocks = bar_rects(&s);
    // z spans 0..95; breaks 0, 20, …, 80 → inner 20 / 40 / 60 / 80 → 5 blocks.
    assert_eq!(blocks.len(), 5, "{blocks:?}");
    // Even steps: all the same height.
    let h0 = blocks[0].3;
    assert!(blocks.iter().all(|b| (b.3 - h0).abs() < 1e-6), "{blocks:?}");
    assert!(s.contains(">20<") && s.contains(">80<"));
    // Uneven steps follow the values: the last block (80–95) is shorter.
    let uneven = bar_rects(&svg(plot().guide_colourbar(
        Aesthetic::Color,
        GuideColourbar::steps().with_even_steps(false),
    )));
    assert_eq!(uneven.len(), 5);
    let top = uneven.iter().min_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
    assert!(top.3 < h0, "{uneven:?}");
}

#[test]
fn reverse_one_aesthetic_only() {
    let p = || {
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y").color("g").shape("g"))
            .geom_point()
    };
    let pos = |s: &str, l: &str| s.find(&format!(">{l}<")).unwrap();
    let base = svg(p().guide_title(Aesthetic::Shape, "S"));
    let rev = svg(p()
        .guide_title(Aesthetic::Shape, "S")
        .guide_reverse(Aesthetic::Color, true));
    assert!(pos(&base, "a") < pos(&base, "c"));
    assert!(pos(&rev, "a") > pos(&rev, "c"), "colour keys reversed");
    // `guides(..)` afterwards keeps the per-aesthetic options.
    let kept = svg(plot()
        .guide_coloursteps(Aesthetic::Color)
        .guides(GuideLegend::new().with_title("Z")));
    assert_eq!(bar_rects(&kept).len(), 5);
}
