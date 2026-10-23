//! #176 "done when": a node-link diagram — boxes with wrapped labels, rounded
//! corners, arrows between box edges — drawn entirely through the public
//! `GGPlot` geom builder API, no hand-rolled SVG. This mirrors the pattern
//! `anofox-visualization`'s `diagram.rs` hand-rolled before this issue
//! (DataZooDE/anofox-visualization#318): `geom_rect().with_radius(_)` for the
//! node boxes, `geom_text().with_wrap(_)` for their labels, and
//! `geom_segment().arrow(_)` with endpoints from [`rect_edge_point`] for the
//! connecting edges.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::arrow::{arrow, rect_edge_point};
use ggplot_rs::geom::rect::GeomRect;
use ggplot_rs::geom::segment::GeomSegment;
use ggplot_rs::geom::text::GeomText;
use ggplot_rs::render::text::wrap_text;
use ggplot_rs::theme::presets::theme_void;
use ggplot_rs::GGPlot;

fn fcol(name: &str, v: &[f64]) -> (String, Vec<Value>) {
    (
        name.to_string(),
        v.iter().map(|f| Value::Float(*f)).collect(),
    )
}
fn scol(name: &str, v: &[String]) -> (String, Vec<Value>) {
    (
        name.to_string(),
        v.iter().map(|s| Value::Str(s.clone())).collect(),
    )
}

struct Node {
    cx: f64,
    cy: f64,
    hw: f64,
    hh: f64,
    label: &'static str,
}

/// Four boxes: a straight run (Start → Validate → Done) plus a branch
/// (Validate → Error handling) — enough to exercise a horizontal edge, a
/// vertical edge, and a long label that must wrap.
fn nodes() -> Vec<Node> {
    vec![
        Node {
            cx: 0.0,
            cy: 0.0,
            hw: 55.0,
            hh: 25.0,
            label: "Start",
        },
        Node {
            cx: 260.0,
            cy: 0.0,
            hw: 95.0,
            hh: 34.0,
            label: "Validate and transform the incoming request",
        },
        Node {
            cx: 520.0,
            cy: 0.0,
            hw: 55.0,
            hh: 25.0,
            label: "Done",
        },
        Node {
            cx: 260.0,
            cy: 170.0,
            hw: 70.0,
            hh: 25.0,
            label: "Error handling",
        },
    ]
}

/// `(from, to)` node indices for each connecting edge.
fn edges() -> Vec<(usize, usize)> {
    vec![(0, 1), (1, 2), (1, 3)]
}

const RADIUS: f64 = 10.0;
const WRAP_WIDTH: f64 = 150.0;
const TEXT_SIZE: f64 = 12.0;

/// A segment's `(start, end)` box-edge points, each `(x, y)`.
type EdgePoints = ((f64, f64), (f64, f64));

/// Builds the diagram's SVG, plus the independently-computed edge endpoints
/// and wrapped-line counts the assertions check the render against.
fn render() -> (String, Vec<EdgePoints>, Vec<usize>) {
    let ns = nodes();

    let boxes = vec![
        fcol("xmin", &ns.iter().map(|n| n.cx - n.hw).collect::<Vec<_>>()),
        fcol("xmax", &ns.iter().map(|n| n.cx + n.hw).collect::<Vec<_>>()),
        fcol("ymin", &ns.iter().map(|n| n.cy - n.hh).collect::<Vec<_>>()),
        fcol("ymax", &ns.iter().map(|n| n.cy + n.hh).collect::<Vec<_>>()),
    ];
    let labels = vec![
        fcol("x", &ns.iter().map(|n| n.cx).collect::<Vec<_>>()),
        fcol("y", &ns.iter().map(|n| n.cy).collect::<Vec<_>>()),
        scol(
            "label",
            &ns.iter().map(|n| n.label.to_string()).collect::<Vec<_>>(),
        ),
    ];

    // The connecting edges' endpoints: each box's edge facing the other
    // box, computed by #176's new helper — not the raw centres, and not
    // hand-rolled "which axis clips first" math in the test itself.
    let seg_pts: Vec<EdgePoints> = edges()
        .iter()
        .map(|&(a, b)| {
            let (na, nb) = (&ns[a], &ns[b]);
            let start = rect_edge_point((na.cx, na.cy), na.hw, na.hh, (nb.cx, nb.cy));
            let end = rect_edge_point((nb.cx, nb.cy), nb.hw, nb.hh, (na.cx, na.cy));
            (start, end)
        })
        .collect();
    let segments = vec![
        fcol("x", &seg_pts.iter().map(|(s, _)| s.0).collect::<Vec<_>>()),
        fcol("y", &seg_pts.iter().map(|(s, _)| s.1).collect::<Vec<_>>()),
        fcol(
            "xend",
            &seg_pts.iter().map(|(_, e)| e.0).collect::<Vec<_>>(),
        ),
        fcol(
            "yend",
            &seg_pts.iter().map(|(_, e)| e.1).collect::<Vec<_>>(),
        ),
    ];

    let svg = GGPlot::new(boxes)
        .aes(
            Aes::new()
                .xmin("xmin")
                .xmax("xmax")
                .ymin("ymin")
                .ymax("ymax"),
        )
        .geom_rect_with(GeomRect::default().with_radius(RADIUS))
        .geom_text_with(GeomText::default().with_wrap(WRAP_WIDTH).with_vjust(0.5))
        .layer_data(labels)
        .layer_aes(Aes::new().x("x").y("y").label("label"))
        .geom_segment_with(GeomSegment::default().arrow(arrow()))
        .layer_data(segments)
        .layer_aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
        .theme(theme_void())
        .render_svg_native_with_size(640, 340)
        .expect("diagram renders");

    let line_counts: Vec<usize> = ns
        .iter()
        .map(|n| wrap_text(n.label, WRAP_WIDTH, TEXT_SIZE).len())
        .collect();

    (svg, seg_pts, line_counts)
}

#[test]
fn node_link_diagram_draws_with_geoms_only() {
    let (svg, _seg_pts, _line_counts) = render();
    assert!(svg.contains("<svg"), "{svg}");
    assert!(svg.contains("</svg>"));
}

/// Part 1: `geom_rect().with_radius(_)` reaches the SVG as `rx`/`ry` on every
/// box's `<rect>` — no plain, square-cornered boxes slipped through.
#[test]
fn boxes_have_rounded_corners() {
    let (svg, _, _) = render();
    let rx_count = svg.matches(&format!("rx=\"{RADIUS:.2}\"")).count();
    assert_eq!(
        rx_count,
        nodes().len(),
        "expected one rx={RADIUS:.2} rect per node box in {svg}"
    );
}

/// Part 2: `geom_text().with_wrap(_)` draws one `<text>` per wrapped line —
/// the long "Validate and transform the incoming request" label wraps to
/// more than one line, and the total `<text>` count matches exactly (a
/// `theme_void()` plot has no axis/legend text to confound the count).
#[test]
fn long_labels_wrap_to_multiple_text_elements() {
    let (svg, _, line_counts) = render();
    assert!(
        line_counts.iter().any(|&n| n > 1),
        "expected at least one label to wrap: {line_counts:?}"
    );
    let expected_total: usize = line_counts.iter().sum();
    let text_count = svg.matches("<text").count();
    assert_eq!(
        text_count, expected_total,
        "expected exactly the wrapped line count's worth of <text> elements"
    );
}

/// Extracts `key="value"` as `f64` from a tag's attribute text.
fn attr_f64(tag: &str, key: &str) -> f64 {
    let needle = format!(" {key}=\"");
    let at = tag
        .find(&needle)
        .unwrap_or_else(|| panic!("no {key} in {tag}"))
        + needle.len();
    let rest = &tag[at..];
    let end = rest.find('"').unwrap();
    rest[..end].parse().unwrap()
}

/// Every `<rect `/`<polyline ` tag's attribute text (up to its closing `>`),
/// with a leading space so `attr_f64`/`attr_str` can match a key at the
/// very start the same way they match one later in the attribute list.
fn tags(svg: &str, open: &str) -> Vec<String> {
    svg.split(open)
        .skip(1)
        .map(|chunk| format!(" {}", &chunk[..chunk.find('>').unwrap()]))
        .collect()
}

/// A linear `px = a*data + b` map, recovered from two known (data, px) pairs.
struct Affine {
    a: f64,
    b: f64,
}
impl Affine {
    fn from_two_points(d0: f64, p0: f64, d1: f64, p1: f64) -> Self {
        let a = (p1 - p0) / (d1 - d0);
        Affine { a, b: p0 - a * d0 }
    }
    fn apply(&self, d: f64) -> f64 {
        self.a * d + self.b
    }
}

/// Part 3: each `geom_segment`'s drawn `<polyline>` starts and ends exactly
/// at the box-edge points [`rect_edge_point`] computed — not at the raw
/// node centres, and not short of or past the edge. The node boxes' own
/// `<rect>`s (same data-space x/y scales as the segments, since every mark
/// in one panel shares them) give the pixel-space affine map to check
/// against, independently of the segment geom's own transform code.
#[test]
fn segments_start_and_end_exactly_on_the_box_edges() {
    let (svg, seg_pts, _) = render();
    let ns = nodes();

    let rects = tags(&svg, "<rect ");
    assert_eq!(rects.len(), ns.len(), "expected one <rect> per node box");
    // Recover the shared x/y affine maps from the first box: its `<rect>`'s
    // (x, width) against its known data (xmin, xmax), and (y, height)
    // against (ymax, ymin) — a rect's SVG `y` is its *top*, i.e. the larger
    // data y (standard Cartesian: y grows up in data, down in pixels).
    let r0 = &rects[0];
    let n0 = &ns[0];
    let (rx, ry, rw, rh) = (
        attr_f64(r0, "x"),
        attr_f64(r0, "y"),
        attr_f64(r0, "width"),
        attr_f64(r0, "height"),
    );
    let x_map = Affine::from_two_points(n0.cx - n0.hw, rx, n0.cx + n0.hw, rx + rw);
    let y_map = Affine::from_two_points(n0.cy + n0.hh, ry, n0.cy - n0.hh, ry + rh);

    // Every other box's rect must agree with the SAME map (confirms it's
    // really one shared linear scale, not a coincidence of box 0's numbers).
    for (i, r) in rects.iter().enumerate().skip(1) {
        let n = &ns[i];
        let tol = 0.5;
        assert!(
            (attr_f64(r, "x") - x_map.apply(n.cx - n.hw)).abs() < tol,
            "box {i} x"
        );
        assert!(
            (attr_f64(r, "y") - y_map.apply(n.cy + n.hh)).abs() < tol,
            "box {i} y"
        );
    }

    // Each edge draws two `<polyline>`s: the segment itself, and its (open)
    // arrowhead's two wings — the latter decorative, `aria-hidden` (see
    // `SvgBackend::push_mark`'s `mark_part` handling). Keep only the
    // segment lines themselves.
    let polylines: Vec<String> = tags(&svg, "<polyline ")
        .into_iter()
        .filter(|t| !t.contains("aria-hidden"))
        .collect();
    assert_eq!(
        polylines.len(),
        seg_pts.len(),
        "expected one polyline per edge"
    );
    for (i, (p, (start, end))) in polylines.iter().zip(seg_pts.iter()).enumerate() {
        let pts = attr_str(p, "points");
        let mut it = pts.split(' ');
        let (ax0, ay0) = it.next().unwrap().split_once(',').unwrap();
        let (ax1, ay1) = it.next().unwrap().split_once(',').unwrap();
        let (ax0, ay0): (f64, f64) = (ax0.parse().unwrap(), ay0.parse().unwrap());
        let (ax1, ay1): (f64, f64) = (ax1.parse().unwrap(), ay1.parse().unwrap());

        let tol = 0.5; // px, allows for the SVG's 2-decimal rounding
        assert!(
            (ax0 - x_map.apply(start.0)).abs() < tol && (ay0 - y_map.apply(start.1)).abs() < tol,
            "edge {i} start {:?} doesn't land on the box edge (expected ({}, {}))",
            (ax0, ay0),
            x_map.apply(start.0),
            y_map.apply(start.1)
        );
        assert!(
            (ax1 - x_map.apply(end.0)).abs() < tol && (ay1 - y_map.apply(end.1)).abs() < tol,
            "edge {i} end {:?} doesn't land on the box edge (expected ({}, {}))",
            (ax1, ay1),
            x_map.apply(end.0),
            y_map.apply(end.1)
        );
    }
}

/// Extracts `key="value"` as `&str` from a tag's attribute text.
fn attr_str<'a>(tag: &'a str, key: &str) -> &'a str {
    let needle = format!(" {key}=\"");
    let at = tag
        .find(&needle)
        .unwrap_or_else(|| panic!("no {key} in {tag}"))
        + needle.len();
    let rest = &tag[at..];
    let end = rest.find('"').unwrap();
    &rest[..end]
}
