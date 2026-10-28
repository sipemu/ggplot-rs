//! A draw-nothing backend that records the screen geometry of what is drawn
//! — for layout decisions that need to know where the marks are, such as
//! placing an inside legend in an empty corner
//! ([`GGPlot::legend_inside_auto`](crate::GGPlot::legend_inside_auto), #332).

use crate::render::backend::{
    DrawBackend, LineStyle, PointStyle, RectStyle, TextAnchor, TextStyle,
};
use crate::render::{Rect, RenderError};

/// Cap on recorded primitives; past it, later primitives are sampled.
const MAX_PRIMITIVES: usize = 200_000;

type Pt = (f64, f64);

/// Records segments, filled polygons and boxes (rects, circles, text).
pub(crate) struct Probe {
    plot_area: Rect,
    total: Rect,
    segments: Vec<(Pt, Pt)>,
    filled: Vec<Vec<Pt>>,
    boxes: Vec<(f64, f64, f64, f64)>,
    claims: Vec<Rect>,
    seen: usize,
}

impl Probe {
    pub(crate) fn new(plot_area: Rect, total: Rect) -> Self {
        Probe {
            plot_area,
            total,
            segments: Vec::new(),
            filled: Vec::new(),
            boxes: Vec::new(),
            claims: Vec::new(),
            seen: 0,
        }
    }

    /// Keep this primitive? Every one up to the cap, then every k-th.
    fn keep(&mut self) -> bool {
        self.seen += 1;
        let n = self.segments.len() + self.filled.len() + self.boxes.len();
        n < MAX_PRIMITIVES || self.seen.is_multiple_of(1 + self.seen / MAX_PRIMITIVES)
    }

    fn add_box(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        if [x0, y0, x1, y1].iter().all(|v| v.is_finite()) && self.keep() {
            self.boxes
                .push((x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)));
        }
    }

    fn add_path(&mut self, pts: &[Pt], closed: bool) {
        let n = pts.len();
        let edges = if closed && n > 2 {
            n
        } else {
            n.saturating_sub(1)
        };
        for i in 0..edges {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            if [a.0, a.1, b.0, b.1].iter().all(|v| v.is_finite()) && self.keep() {
                self.segments.push((a, b));
            }
        }
        if n == 1 {
            self.add_box(pts[0].0, pts[0].1, pts[0].0, pts[0].1);
        }
    }

    /// The bounding box of everything drawn, if anything.
    pub(crate) fn extent(&self) -> Option<(f64, f64, f64, f64)> {
        let pts = self
            .segments
            .iter()
            .flat_map(|(a, b)| [*a, *b])
            .chain(self.filled.iter().flatten().copied())
            .chain(self.boxes.iter().flat_map(|b| [(b.0, b.1), (b.2, b.3)]));
        pts.fold(None, |acc, (x, y)| {
            Some(match acc {
                None => (x, y, x, y),
                Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
            })
        })
    }

    /// Does anything drawn touch the box `(x0, y0, x1, y1)`?
    pub(crate) fn hits(&self, (x0, y0, x1, y1): (f64, f64, f64, f64)) -> bool {
        let inside = |p: Pt| p.0 >= x0 && p.0 <= x1 && p.1 >= y0 && p.1 <= y1;
        if self
            .boxes
            .iter()
            .any(|b| b.0 <= x1 && b.2 >= x0 && b.1 <= y1 && b.3 >= y0)
        {
            return true;
        }
        if self
            .segments
            .iter()
            .any(|&(a, b)| inside(a) || inside(b) || segment_crosses_box(a, b, (x0, y0, x1, y1)))
        {
            return true;
        }
        // A filled polygon can cover the box with every vertex outside it.
        let probes = [
            (x0, y0),
            (x1, y0),
            (x0, y1),
            (x1, y1),
            ((x0 + x1) / 2.0, (y0 + y1) / 2.0),
        ];
        self.filled.iter().any(|poly| {
            poly.iter().any(|&p| inside(p)) || probes.iter().any(|&q| in_polygon(q, poly))
        })
    }
}

fn segment_crosses_box(a: Pt, b: Pt, (x0, y0, x1, y1): (f64, f64, f64, f64)) -> bool {
    let edges = [
        ((x0, y0), (x1, y0)),
        ((x1, y0), (x1, y1)),
        ((x1, y1), (x0, y1)),
        ((x0, y1), (x0, y0)),
    ];
    edges.iter().any(|&(c, d)| segments_intersect(a, b, c, d))
}

fn segments_intersect(p1: Pt, p2: Pt, p3: Pt, p4: Pt) -> bool {
    let cross = |o: Pt, a: Pt, b: Pt| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let (d1, d2) = (cross(p3, p4, p1), cross(p3, p4, p2));
    let (d3, d4) = (cross(p1, p2, p3), cross(p1, p2, p4));
    (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
}

/// Even-odd point-in-polygon.
fn in_polygon((x, y): Pt, poly: &[Pt]) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.1 > y) != (b.1 > y) && x < (b.0 - a.0) * (y - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
    }
    inside
}

impl DrawBackend for Probe {
    fn draw_circle(&mut self, c: Pt, r: f64, _style: &PointStyle) -> Result<(), RenderError> {
        self.add_box(c.0 - r, c.1 - r, c.0 + r, c.1 + r);
        Ok(())
    }
    fn draw_line(&mut self, pts: &[Pt], _style: &LineStyle) -> Result<(), RenderError> {
        self.add_path(pts, false);
        Ok(())
    }
    fn draw_rect(&mut self, a: Pt, b: Pt, style: &RectStyle) -> Result<(), RenderError> {
        if style.fill.is_some() {
            self.add_box(a.0, a.1, b.0, b.1);
        } else {
            self.add_path(&[a, (b.0, a.1), b, (a.0, b.1)], true);
        }
        Ok(())
    }
    fn draw_text(&mut self, text: &str, pos: Pt, style: &TextStyle) -> Result<(), RenderError> {
        let w = text.chars().count() as f64 * style.size * 0.6;
        let (x0, x1) = match style.anchor {
            TextAnchor::Start => (pos.0, pos.0 + w),
            TextAnchor::Middle => (pos.0 - w / 2.0, pos.0 + w / 2.0),
            TextAnchor::End => (pos.0 - w, pos.0),
        };
        self.add_box(x0, pos.1 - style.size / 2.0, x1, pos.1 + style.size / 2.0);
        Ok(())
    }
    fn draw_polygon(&mut self, pts: &[Pt], style: &RectStyle) -> Result<(), RenderError> {
        if style.fill.is_some() && pts.len() > 2 && self.keep() {
            self.filled.push(pts.to_vec());
        } else {
            self.add_path(pts, true);
        }
        Ok(())
    }
    fn draw_quadratic(
        &mut self,
        p0: Pt,
        ctrl: Pt,
        p1: Pt,
        _style: &LineStyle,
    ) -> Result<(), RenderError> {
        let pts: Vec<Pt> = (0..=8)
            .map(|i| {
                let t = i as f64 / 8.0;
                let u = 1.0 - t;
                (
                    u * u * p0.0 + 2.0 * u * t * ctrl.0 + t * t * p1.0,
                    u * u * p0.1 + 2.0 * u * t * ctrl.1 + t * t * p1.1,
                )
            })
            .collect();
        self.add_path(&pts, false);
        Ok(())
    }
    fn draw_shape(&mut self, c: Pt, r: f64, _style: &PointStyle) -> Result<(), RenderError> {
        self.add_box(c.0 - r, c.1 - r, c.0 + r, c.1 + r);
        Ok(())
    }
    fn plot_area(&self) -> Rect {
        self.plot_area.clone()
    }
    fn total_area(&self) -> Rect {
        self.total.clone()
    }
    fn corner_claims(&self) -> Vec<Rect> {
        self.claims.clone()
    }
    fn claim_corner_box(&mut self, rect: Rect) {
        self.claims.push(rect);
    }
    fn clear_corner_claims(&mut self) {
        self.claims.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe() -> Probe {
        let r = Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        Probe::new(r.clone(), r)
    }

    #[test]
    fn filled_polygon_covering_a_box_hits_it() {
        let mut p = probe();
        let style = RectStyle {
            fill: Some((1, 2, 3)),
            stroke: None,
            stroke_width: 0.0,
            alpha: 1.0,
            clip: false,
            corner_radius: 0.0,
        };
        p.draw_polygon(
            &[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            &style,
        )
        .unwrap();
        assert!(
            p.hits((40.0, 40.0, 60.0, 60.0)),
            "covered, no vertex inside"
        );
    }

    #[test]
    fn a_line_crossing_a_box_hits_it_and_a_distant_one_does_not() {
        let mut p = probe();
        p.draw_line(&[(0.0, 50.0), (100.0, 50.0)], &LineStyle::default())
            .unwrap();
        assert!(p.hits((40.0, 40.0, 60.0, 60.0)));
        assert!(!p.hits((40.0, 70.0, 60.0, 90.0)));
    }

    #[test]
    fn every_primitive_is_recorded_with_its_extent() {
        let mut p = probe();
        assert_eq!(p.extent(), None);
        let point = PointStyle::default();
        p.draw_circle((10.0, 10.0), 2.0, &point).unwrap();
        p.draw_shape((90.0, 90.0), 3.0, &point).unwrap();
        let unfilled = RectStyle {
            fill: None,
            stroke: Some((0, 0, 0)),
            stroke_width: 1.0,
            alpha: 1.0,
            clip: false,
            corner_radius: 0.0,
        };
        p.draw_rect((20.0, 20.0), (30.0, 30.0), &unfilled).unwrap();
        p.draw_polygon(&[(50.0, 0.0), (60.0, 5.0)], &unfilled)
            .unwrap();
        p.draw_quadratic(
            (0.0, 100.0),
            (50.0, 50.0),
            (100.0, 100.0),
            &LineStyle::default(),
        )
        .unwrap();
        p.draw_line(&[(70.0, 70.0)], &LineStyle::default()).unwrap();
        for anchor in [TextAnchor::Start, TextAnchor::Middle, TextAnchor::End] {
            let style = TextStyle {
                anchor,
                size: 10.0,
                ..Default::default()
            };
            p.draw_text("abc", (40.0, 60.0), &style).unwrap();
        }
        assert_eq!(p.extent(), Some((0.0, 0.0, 100.0, 100.0)));
        // An unfilled rect only hits along its outline, not inside.
        assert!(p.hits((19.0, 24.0, 21.0, 26.0)));
        assert!(!p.hits((24.0, 24.0, 26.0, 26.0)));
        assert!(p.hits((69.0, 69.0, 71.0, 71.0)), "a single-point line");
        assert_eq!(p.plot_area().width, 100.0);
        assert_eq!(p.total_area().height, 100.0);
    }

    #[test]
    fn corner_claims_are_kept_until_cleared() {
        let mut p = probe();
        let r = p.plot_area();
        p.claim_corner_box(r.clone());
        assert_eq!(p.corner_claims().len(), 1);
        p.clear_corner_claims();
        assert!(p.corner_claims().is_empty());
    }

    #[test]
    fn primitives_past_the_cap_are_sampled() {
        let mut p = probe();
        for i in 0..(MAX_PRIMITIVES + 1000) {
            let x = (i % 100) as f64;
            p.draw_circle((x, x), 0.5, &PointStyle::default()).unwrap();
        }
        assert!(p.boxes.len() >= MAX_PRIMITIVES);
        assert!(p.boxes.len() < MAX_PRIMITIVES + 1000);
    }
}
