//! #176 coverage for the parts `tests/diagram_primitives_176.rs` doesn't
//! reach: `GeomLabel::with_wrap`/`with_radius`/`with_line_height` (that
//! integration test only exercises `GeomText`'s wrap and `GeomRect`'s
//! radius), `GeomTile::with_radius`, a custom (non-default) line height's
//! effect on wrapped-line spacing, and `check_overlap` combined with
//! wrapping on both `GeomText` and `GeomLabel`.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::theme::presets::theme_void;

fn fcol(name: &str, v: &[f64]) -> (String, Vec<Value>) {
    (
        name.to_string(),
        v.iter().map(|f| Value::Float(*f)).collect(),
    )
}
fn scol(name: &str, v: &[&str]) -> (String, Vec<Value>) {
    (
        name.to_string(),
        v.iter().map(|s| Value::Str((*s).to_string())).collect(),
    )
}
fn one_label(text: &str) -> Vec<(String, Vec<Value>)> {
    vec![fcol("x", &[0.0]), fcol("y", &[0.0]), scol("label", &[text])]
}

fn attr_f64(tag: &str, key: &str) -> f64 {
    let needle = format!(" {key}=\"");
    let at = tag.find(&needle).unwrap() + needle.len();
    let rest = &tag[at..];
    rest[..rest.find('"').unwrap()].parse().unwrap()
}

fn tags(svg: &str, open: &str) -> Vec<String> {
    svg.split(open)
        .skip(1)
        .map(|c| format!(" {}", &c[..c.find('>').unwrap()]))
        .collect()
}

/// `GeomLabel::with_wrap` wraps the label to several lines, and its
/// background box grows (width and height) to fit them plus padding,
/// while also carrying `with_radius`'s corner radius through.
#[test]
fn geom_label_wrap_grows_the_box_and_keeps_its_radius() {
    let text = "a rather long label that needs to wrap onto more than one line";
    let wrap_width = 90.0;
    let size = 11.0;
    let padding = 4.0;
    let radius = 6.0;

    let svg = GGPlot::new(one_label(text))
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label_with(
            GeomLabel {
                size,
                padding,
                ..GeomLabel::default()
            }
            .with_wrap(wrap_width)
            .with_radius(radius),
        )
        .theme(theme_void())
        .render_svg_native_with_size(400, 300)
        .unwrap();

    let lines = ggplot_rs::render::text::wrap_text(text, wrap_width, size);
    assert!(lines.len() > 1, "test needs a wrapping label: {lines:?}");

    let rects = tags(&svg, "<rect ");
    assert_eq!(rects.len(), 1, "one background rect: {svg}");
    let r = &rects[0];
    assert!(
        (attr_f64(r, "rx") - radius).abs() < 1e-6,
        "background box should carry the radius: {r}"
    );
    // Box width fits the widest wrapped line plus padding on both sides.
    let widest = ggplot_rs::render::text::wrapped_block_width(&lines, size);
    let expected_w = widest + 2.0 * padding;
    assert!(
        (attr_f64(r, "width") - expected_w).abs() < 1.0,
        "box width {} should fit the wrapped text ({expected_w}): {r}",
        attr_f64(r, "width")
    );
    // Box height fits every wrapped line (default line_height 1.2) plus padding.
    let lh = size * 1.2;
    let expected_h = lh * lines.len() as f64 + 2.0 * padding;
    assert!(
        (attr_f64(r, "height") - expected_h).abs() < 1.0,
        "box height {} should fit {} lines ({expected_h}): {r}",
        attr_f64(r, "height"),
        lines.len()
    );

    let text_count = svg.matches("<text").count();
    assert_eq!(
        text_count,
        lines.len(),
        "one <text> per wrapped line: {svg}"
    );
}

/// A custom (non-default) `with_line_height` changes the vertical spacing
/// between wrapped lines proportionally — not just the default `1.2`.
#[test]
fn custom_line_height_scales_wrapped_line_spacing() {
    let text = "alpha beta gamma delta epsilon zeta eta theta";
    let wrap_width = 60.0;
    let size = 10.0;

    let render_with = |line_height: f64| {
        GGPlot::new(one_label(text))
            .aes(Aes::new().x("x").y("y").label("label"))
            .geom_text_with(
                GeomText::default()
                    .with_wrap(wrap_width)
                    .with_line_height(line_height),
            )
            .theme(theme_void())
            .render_svg_native_with_size(400, 300)
            .unwrap()
    };
    let y_positions = |svg: &str| -> Vec<f64> {
        tags(svg, "<text ")
            .iter()
            .map(|t| attr_f64(t, "y"))
            .collect()
    };

    let tight = render_with(1.0);
    let loose = render_with(3.0);
    let ty = y_positions(&tight);
    let ly = y_positions(&loose);
    assert!(ty.len() > 1, "test needs a wrapping label: {ty:?}");
    assert_eq!(ty.len(), ly.len());

    let tight_gap = (ty[1] - ty[0]).abs();
    let loose_gap = (ly[1] - ly[0]).abs();
    assert!(
        loose_gap > tight_gap * 2.0,
        "line_height=3.0 gap ({loose_gap}) should be much bigger than \
         line_height=1.0's ({tight_gap})"
    );
    // The gap is exactly `size * line_height` between consecutive lines.
    assert!((tight_gap - size * 1.0).abs() < 0.5, "got {tight_gap}");
    assert!((loose_gap - size * 3.0).abs() < 0.5, "got {loose_gap}");
}

/// `check_overlap` on a wrapped `GeomLabel` still drops the second of two
/// labels placed on top of each other — the overlap bbox uses the wrapped
/// block's real height/width, not a single line's.
#[test]
fn check_overlap_drops_an_overlapping_wrapped_label() {
    let text = "a fairly long repeated label that wraps";
    let data = vec![
        fcol("x", &[0.0, 0.0]), // exactly on top of each other
        fcol("y", &[0.0, 0.0]),
        scol("label", &[text, text]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label_with(
            GeomLabel::default()
                .with_wrap(80.0)
                .with_check_overlap(true),
        )
        .theme(theme_void())
        .render_svg_native_with_size(400, 300)
        .unwrap();

    let lines = ggplot_rs::render::text::wrap_text(text, 80.0, 10.0);
    assert!(lines.len() > 1, "test needs a wrapping label: {lines:?}");
    // Both rows carry the identical label at nearly the same spot: only the
    // first should survive check_overlap, so exactly one background rect
    // and that label's wrapped lines' worth of <text>, not twice as many.
    assert_eq!(tags(&svg, "<rect ").len(), 1, "{svg}");
    assert_eq!(svg.matches("<text").count(), lines.len(), "{svg}");
}

/// `GeomTile::with_radius` reaches the SVG the same way `GeomRect`'s does.
#[test]
fn geom_tile_with_radius_rounds_its_corners() {
    let data = vec![fcol("x", &[0.0, 1.0]), fcol("y", &[0.0, 0.0])];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_tile_with(GeomTile::default().with_radius(4.0))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let count = svg.matches("rx=\"4.00\"").count();
    assert_eq!(count, 2, "both tiles should be rounded: {svg}");
}

/// Under polar coords `GeomRect` draws an annulus sector (a polygon, via
/// `draw_polygon`), not an axis-aligned `draw_rect` — `radius` has no
/// meaning there, and `GeomRect::draw` is written to pass `corner_radius:
/// 0.0` in that case rather than the (meaningless) configured radius. This
/// does not change what's drawn (polar never calls `draw_rect`), but
/// exercises that `is_polar()` branch so it's not silently wrong — and
/// confirms setting `.with_radius(_)` under polar neither panics nor leaks
/// an `rx` attribute (there is no `<rect>` to put one on).
#[test]
fn geom_rect_radius_is_a_no_op_under_polar_coords() {
    let data = vec![
        fcol("xmin", &[0.0]),
        fcol("xmax", &[1.0]),
        fcol("ymin", &[0.0]),
        fcol("ymax", &[5.0]),
    ];
    let svg = GGPlot::new(data)
        .aes(
            Aes::new()
                .xmin("xmin")
                .xmax("xmax")
                .ymin("ymin")
                .ymax("ymax"),
        )
        .geom_rect_with(GeomRect::default().with_radius(8.0))
        .coord_polar()
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert!(
        svg.contains("<polygon"),
        "polar rect should be a sector polygon: {svg}"
    );
    assert!(
        !svg.contains(" rx="),
        "a polygon has no rx attribute: {svg}"
    );
}
