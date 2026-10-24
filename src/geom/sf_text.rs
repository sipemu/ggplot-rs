//! `geom_sf_text` / `geom_sf_label` — labels placed inside simple-features
//! polygons (feature `sf`), ggplot2's `geom_sf_text`/`geom_sf_label`.
//!
//! A label's anchor is a polygon's centroid or its pole of inaccessibility
//! (`polylabel`, Mapbox's algorithm — the inside point farthest from the
//! outline, so a C-shaped region's label stays in the spine rather than the
//! centroid, which can fall outside); `check_overlap` drops a lower-
//! priority label that would collide with one already placed, stay outside
//! the panel, or not fit its region, and a halo (shadowtext-style:
//! `bg_color`/`bg_r`) keeps a label legible over a busy fill. Ported from
//! anofox-visualization's `map/labels.rs` (DataZooDE/anofox-visualization#318).

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::spatial::{parse_wkt, Geometry, SfProjection};
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// How a label's anchor is chosen within its polygon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SfLabelAnchor {
    /// The ring's area-weighted centroid — cheap, but can land outside a
    /// concave (e.g. C-shaped) region.
    Centroid,
    /// The pole of inaccessibility (Mapbox's `polylabel`): the inside point
    /// farthest from the outline. Always inside the polygon. Default.
    #[default]
    Polylabel,
}

/// Shared rendering for `GeomSfText`/`GeomSfLabel`: anchor placement,
/// collision/legend avoidance, the optional background box, the halo.
struct SfTextStyle {
    size: f64,
    color: (u8, u8, u8),
    alpha: f64,
    anchor: SfLabelAnchor,
    check_overlap: bool,
    /// `None` disables the halo (default). `Some((color, r))` draws the
    /// text `r` px out in 8 directions in `color` first — a cheap halo any
    /// `DrawBackend` can render with plain `draw_text` calls, portable
    /// across the SVG/pixel/plotters backends alike (shadowtext's
    /// `bg.colour`/`bg.r`).
    halo: Option<((u8, u8, u8), f64)>,
    projection: SfProjection,
    /// `Some` draws a background rect behind the text (`geom_sf_label`).
    label_fill: Option<(u8, u8, u8)>,
    label_padding: f64,
}

/// Text labels placed inside `geom_sf` polygons (feature `sf`).
pub struct GeomSfText {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub anchor: SfLabelAnchor,
    pub check_overlap: bool,
    pub halo: Option<((u8, u8, u8), f64)>,
    pub projection: SfProjection,
}

impl Default for GeomSfText {
    fn default() -> Self {
        GeomSfText {
            size: 10.0,
            color: (50, 50, 50),
            alpha: 1.0,
            anchor: SfLabelAnchor::default(),
            check_overlap: false,
            halo: None,
            projection: SfProjection::default(),
        }
    }
}

impl GeomSfText {
    /// `centroid` or `polylabel` (default) anchor placement.
    pub fn fun_geometry(mut self, anchor: SfLabelAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Drop a label that would collide with one already placed (priority:
    /// the mapped `size` aesthetic if present, else the region's screen
    /// area — larger first), fall outside the panel, or not fit its
    /// region's width. Default `false` (ggplot2's `geom_text` default).
    pub fn check_overlap(mut self, check: bool) -> Self {
        self.check_overlap = check;
        self
    }

    /// A shadowtext-style halo: `color` drawn `r` px out in 8 directions
    /// behind the label (`bg.colour`/`bg.r`).
    pub fn halo(mut self, color: (u8, u8, u8), r: f64) -> Self {
        self.halo = (r > 0.0).then_some((color, r));
        self
    }

    /// The map projection applied to lon/lat coordinates (match the
    /// `geom_sf`/`coord_sf` layer this labels).
    pub fn project(mut self, projection: SfProjection) -> Self {
        self.projection = projection;
        self
    }

    fn style(&self) -> SfTextStyle {
        SfTextStyle {
            size: self.size,
            color: self.color,
            alpha: self.alpha,
            anchor: self.anchor,
            check_overlap: self.check_overlap,
            halo: self.halo,
            projection: self.projection,
            label_fill: None,
            label_padding: 0.0,
        }
    }
}

impl Geom for GeomSfText {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        draw_sf_text(&self.style(), data, coord, scales, theme, backend)
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Label]
    }

    // Reuses `geom_sf`'s own `StatSf`: parses the `geometry` WKT column and
    // adds projected xmin/xmax/ymin/ymax, so the X/Y position scales train
    // over the labelled geometry's extent even when no `geom_sf` polygon
    // layer shares the plot (`geom_sf_text` alone, like ggplot2's own).
    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(super::sf::StatSf {
            projection: self.projection,
        })
    }
    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }
    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }
    fn name(&self) -> &str {
        "sf_text"
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (50, 50, 50), el.ink) {
            self.color = ink;
        }
    }
}

/// Labels placed inside `geom_sf` polygons, each on a background box
/// (feature `sf`) — `geom_sf_text` plus a card-like rect behind the text.
pub struct GeomSfLabel {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub fill: (u8, u8, u8),
    pub alpha: f64,
    pub padding: f64,
    pub anchor: SfLabelAnchor,
    pub check_overlap: bool,
    pub projection: SfProjection,
}

impl Default for GeomSfLabel {
    fn default() -> Self {
        GeomSfLabel {
            size: 10.0,
            color: (50, 50, 50),
            fill: (255, 255, 255),
            alpha: 0.85,
            padding: 3.0,
            anchor: SfLabelAnchor::default(),
            check_overlap: false,
            projection: SfProjection::default(),
        }
    }
}

impl GeomSfLabel {
    pub fn fun_geometry(mut self, anchor: SfLabelAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn check_overlap(mut self, check: bool) -> Self {
        self.check_overlap = check;
        self
    }

    pub fn project(mut self, projection: SfProjection) -> Self {
        self.projection = projection;
        self
    }

    fn style(&self) -> SfTextStyle {
        SfTextStyle {
            size: self.size,
            color: self.color,
            alpha: self.alpha,
            anchor: self.anchor,
            check_overlap: self.check_overlap,
            halo: None,
            projection: self.projection,
            label_fill: Some(self.fill),
            label_padding: self.padding,
        }
    }
}

impl Geom for GeomSfLabel {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        draw_sf_text(&self.style(), data, coord, scales, theme, backend)
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Label]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(super::sf::StatSf {
            projection: self.projection,
        })
    }
    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }
    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }
    fn name(&self) -> &str {
        "sf_label"
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (50, 50, 50), el.ink) {
            self.color = ink;
        }
        if let (true, Some(paper)) = (self.fill == (255, 255, 255), el.paper) {
            self.fill = paper;
        }
    }
}

/// One candidate label.
struct Candidate {
    at: (f64, f64),
    text: String,
    /// Higher goes first.
    priority: f64,
}

fn draw_sf_text(
    style: &SfTextStyle,
    data: &DataFrame,
    coord: &dyn Coord,
    scales: &ScaleSet,
    theme: &Theme,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    let geom_col = data
        .column("geometry")
        .ok_or(RenderError::MissingAesthetic("geometry".into()))?;
    let label_col = data
        .column("label")
        .ok_or(RenderError::MissingAesthetic("label".into()))?;
    let size_col = data.column("size");

    let plot_area = backend.plot_area();
    let x_scale = scales.get(&Aesthetic::X);
    let y_scale = scales.get(&Aesthetic::Y);
    let project = |c: [f64; 2]| {
        let [x, y] = style.projection.project(c);
        let nx = x_scale.map(|s| s.map(&Value::Float(x))).unwrap_or(0.0);
        let ny = y_scale.map(|s| s.map(&Value::Float(y))).unwrap_or(0.0);
        coord.transform((nx, ny), &plot_area)
    };

    let mut cands: Vec<Candidate> = Vec::new();
    for i in 0..data.nrows() {
        let g = match &geom_col[i] {
            Value::Str(s) => parse_wkt(s),
            _ => None,
        };
        let Some(g) = g else { continue };
        let text = label_col[i].to_group_key();
        if text.is_empty() {
            continue;
        }
        let Some(rings) = screen_rings(&g, &project) else {
            continue;
        };
        if rings.is_empty() || rings[0].len() < 3 {
            continue;
        }
        let at = match style.anchor {
            SfLabelAnchor::Centroid => centroid(&rings[0]),
            SfLabelAnchor::Polylabel => polylabel(&rings, 1.0)
                .map(|(p, _)| p)
                .unwrap_or_else(|| centroid(&rings[0])),
        };
        let priority = size_col
            .and_then(|c| c[i].as_f64())
            .unwrap_or_else(|| area(&rings[0]));
        cands.push(Candidate { at, text, priority });
    }

    cands.sort_by(|a, b| b.priority.total_cmp(&a.priority));

    let (pw, ph) = (plot_area.width, plot_area.height);
    let mut placed: Vec<(f64, f64, f64, f64)> = Vec::new();
    let font = super::resolve_font_family("", theme);

    for c in &cands {
        let tw = crate::render::text::text_width(&c.text, style.size);
        let th = style.size * 1.15;
        let (px, py) = c.at;
        let bbox = (px - tw / 2.0, py - th / 2.0, px + tw / 2.0, py + th / 2.0);
        // Keep the label inside the panel.
        let local = (
            bbox.0 - plot_area.x,
            bbox.1 - plot_area.y,
            bbox.2 - plot_area.x,
            bbox.3 - plot_area.y,
        );
        if local.0 < 0.0 || local.1 < 0.0 || local.2 > pw || local.3 > ph {
            continue;
        }
        if style.check_overlap && placed.iter().any(|p| overlaps(&bbox, p)) {
            continue;
        }
        if style.check_overlap {
            placed.push(bbox);
        }

        if let Some(fill) = style.label_fill {
            let half_w = tw / 2.0 + style.label_padding;
            let half_h = th / 2.0 + style.label_padding;
            backend.draw_rect(
                (px - half_w, py - half_h),
                (px + half_w, py + half_h),
                &RectStyle {
                    fill: Some(fill),
                    stroke: Some(style.color),
                    stroke_width: 0.5,
                    alpha: style.alpha,
                    clip: true,
                },
            )?;
        }

        let text_style = TextStyle {
            color: style.color,
            size: style.size,
            anchor: TextAnchor::Middle,
            angle: 0.0,
            family: font.clone(),
            face: FontFace::Plain,
        };
        if let Some((halo_color, r)) = style.halo {
            const DIRS: usize = 8;
            for k in 0..DIRS {
                let a = k as f64 * std::f64::consts::TAU / DIRS as f64;
                let op = (px + r * a.cos(), py + r * a.sin());
                backend.draw_text(
                    &c.text,
                    op,
                    &TextStyle {
                        color: halo_color,
                        ..text_style.clone()
                    },
                )?;
            }
        }
        backend.draw_text(&c.text, (px, py), &text_style)?;
    }
    Ok(())
}

fn overlaps(a: &(f64, f64, f64, f64), b: &(f64, f64, f64, f64)) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

/// The rings (exterior + holes, screen space) of a `Polygon`, or the
/// largest part of a `MultiPolygon`. `None` for any other geometry type —
/// labelling a point/line isn't this geom's job (use `geom_text`/
/// `geom_label` with the data's own x/y for that).
fn screen_rings(
    g: &Geometry,
    project: &impl Fn([f64; 2]) -> (f64, f64),
) -> Option<Vec<Vec<(f64, f64)>>> {
    let to_screen = |rings: &[Vec<[f64; 2]>]| -> Vec<Vec<(f64, f64)>> {
        rings
            .iter()
            .map(|r| r.iter().map(|c| project(*c)).collect())
            .collect()
    };
    match g {
        Geometry::Polygon(rings) => Some(to_screen(rings)),
        Geometry::MultiPolygon(polys) => polys
            .iter()
            .max_by(|a, b| ring_area(&a[0]).total_cmp(&ring_area(&b[0])))
            .map(|p| to_screen(p)),
        _ => None,
    }
}

/// The (data-space, unsigned) shoelace area of a ring — used only to pick
/// a `MultiPolygon`'s largest part; final priority/placement area is
/// computed on the projected screen ring.
fn ring_area(r: &[[f64; 2]]) -> f64 {
    let n = r.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    let mut j = n - 1;
    for i in 0..n {
        s += (r[j][0] + r[i][0]) * (r[j][1] - r[i][1]);
        j = i;
    }
    (s / 2.0).abs()
}

/// The screen area of a ring (shoelace, unsigned).
fn area(r: &[(f64, f64)]) -> f64 {
    let n = r.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    let mut j = n - 1;
    for i in 0..n {
        s += (r[j].0 + r[i].0) * (r[j].1 - r[i].1);
        j = i;
    }
    (s / 2.0).abs()
}

fn centroid(r: &[(f64, f64)]) -> (f64, f64) {
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    let n = r.len();
    let mut j = n - 1;
    for i in 0..n {
        let (p, q) = (r[i], r[j]);
        let f = p.0 * q.1 - q.0 * p.1;
        cx += (p.0 + q.0) * f;
        cy += (p.1 + q.1) * f;
        a += f * 3.0;
        j = i;
    }
    if a.abs() < 1e-9 {
        return r[0];
    }
    (cx / a, cy / a)
}

/// Distance from `p` to the polygon outline (`rings[0]` exterior, the rest
/// holes), negative outside.
fn signed_dist(p: (f64, f64), rings: &[Vec<(f64, f64)>]) -> f64 {
    let mut inside = false;
    let mut best = f64::INFINITY;
    for r in rings {
        let n = r.len();
        if n < 2 {
            continue;
        }
        let mut j = n - 1;
        for i in 0..n {
            let (a, b) = (r[i], r[j]);
            if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
                inside = !inside;
            }
            best = best.min(seg_dist2(p, a, b));
            j = i;
        }
    }
    let d = best.sqrt();
    if inside {
        d
    } else {
        -d
    }
}

fn seg_dist2(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (ex, ey) = (a.0 + t * dx - p.0, a.1 + t * dy - p.1);
    ex * ex + ey * ey
}

struct Cell {
    x: f64,
    y: f64,
    h: f64,
    /// Signed distance to the outline (negative outside).
    d: f64,
    /// The best distance any point in the cell could have.
    max: f64,
}

impl Cell {
    fn new(x: f64, y: f64, h: f64, rings: &[Vec<(f64, f64)>]) -> Cell {
        let d = signed_dist((x, y), rings);
        Cell {
            x,
            y,
            h,
            d,
            max: d + h * std::f64::consts::SQRT_2,
        }
    }
}

impl PartialEq for Cell {
    fn eq(&self, o: &Self) -> bool {
        self.max == o.max
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Cell {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.max.total_cmp(&o.max)
    }
}

/// The pole of inaccessibility of a polygon (`rings[0]` the exterior, the
/// rest holes): the inside point farthest from the outline, to `precision`
/// pixels (Mapbox's polylabel). Returns the point and its distance.
fn polylabel(rings: &[Vec<(f64, f64)>], precision: f64) -> Option<((f64, f64), f64)> {
    let outer = rings.first().filter(|r| r.len() >= 3)?;
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in outer {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (w, h) = (x1 - x0, y1 - y0);
    let size = w.min(h);
    if size.is_nan() || size <= 0.0 {
        return Some(((x0, y0), 0.0));
    }
    let mut heap = std::collections::BinaryHeap::new();
    let mut half = size / 2.0;
    let mut x = x0;
    while x < x1 {
        let mut y = y0;
        while y < y1 {
            heap.push(Cell::new(x + half, y + half, half, rings));
            y += size;
        }
        x += size;
    }
    let cen = centroid(outer);
    let mut best = Cell::new(cen.0, cen.1, 0.0, rings);
    let bbox = Cell::new(x0 + w / 2.0, y0 + h / 2.0, 0.0, rings);
    if bbox.d > best.d {
        best = bbox;
    }
    let mut steps = 0;
    while let Some(c) = heap.pop() {
        let (cx, cy, ch, cmax) = (c.x, c.y, c.h, c.max);
        if c.d > best.d {
            best = c;
        }
        if cmax - best.d <= precision {
            continue;
        }
        steps += 1;
        if steps > 10_000 {
            break;
        }
        half = ch / 2.0;
        for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            heap.push(Cell::new(cx + dx * half, cy + dy * half, half, rings));
        }
    }
    Some(((best.x, best.y), best.d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pole_of_a_c_shape_is_inside_it() {
        let c: Vec<(f64, f64)> = vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 2.0),
            (2.0, 2.0),
            (2.0, 8.0),
            (10.0, 8.0),
            (10.0, 10.0),
            (0.0, 10.0),
        ];
        let ((x, y), d) = polylabel(std::slice::from_ref(&c), 0.1).unwrap();
        assert!(
            signed_dist((x, y), std::slice::from_ref(&c)) > 0.0,
            "({x},{y}) inside"
        );
        assert!(x < 2.0 && d > 0.8, "({x},{y}) in the spine, d = {d}");
        let cen = centroid(&c);
        assert!(signed_dist(cen, &[c]) < 0.0, "the centroid is outside");

        let sq = vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        let ((x, y), _) = polylabel(&[sq], 0.01).unwrap();
        assert!((x - 2.0).abs() < 0.05 && (y - 2.0).abs() < 0.05);
    }

    #[test]
    fn polylabel_respects_a_hole() {
        // An 8x8 square with a 4x4 hole dead centre: the pole must avoid it.
        let outer = vec![(0.0, 0.0), (8.0, 0.0), (8.0, 8.0), (0.0, 8.0)];
        let hole = vec![(2.0, 2.0), (6.0, 2.0), (6.0, 6.0), (2.0, 6.0)];
        let ((x, y), _) = polylabel(&[outer, hole], 0.1).unwrap();
        assert!(
            !(2.0..=6.0).contains(&x) || !(2.0..=6.0).contains(&y),
            "({x},{y}) outside the hole"
        );
    }

    #[test]
    fn overlap_detects_shared_area_only() {
        assert!(overlaps(&(0.0, 0.0, 10.0, 10.0), &(5.0, 5.0, 15.0, 15.0)));
        assert!(!overlaps(
            &(0.0, 0.0, 10.0, 10.0),
            &(10.1, 10.1, 20.0, 20.0)
        ));
    }

    #[test]
    fn multipolygon_picks_the_largest_part() {
        let g = parse_wkt(
            "MULTIPOLYGON (((0 0, 1 0, 1 1, 0 1, 0 0)), ((10 10, 20 10, 20 20, 10 20, 10 10)))",
        )
        .unwrap();
        let identity = |c: [f64; 2]| (c[0], c[1]);
        let rings = screen_rings(&g, &identity).unwrap();
        // The second part (100 sq units) beats the first (1 sq unit).
        assert!(rings[0].contains(&(10.0, 10.0)));
    }

    #[test]
    fn non_polygon_geometry_has_no_rings() {
        let identity = |c: [f64; 2]| (c[0], c[1]);
        assert!(screen_rings(&Geometry::Point([1.0, 2.0]), &identity).is_none());
        assert!(screen_rings(
            &Geometry::LineString(vec![[0.0, 0.0], [1.0, 1.0]]),
            &identity
        )
        .is_none());
    }

    #[test]
    fn default_anchor_is_polylabel() {
        assert_eq!(SfLabelAnchor::default(), SfLabelAnchor::Polylabel);
    }
}
