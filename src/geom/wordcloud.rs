//! Word cloud: `geom_wordcloud`. Words sized by `weight` are placed — the
//! largest first — along an Archimedean spiral from the panel centre at the
//! first spot where their bounding box overlaps no placed word
//! (ggwordcloud / wordcloud2). Layout happens at draw time in pixels, since
//! collisions depend on the rendered text size; it is deterministic (no
//! randomness: rotations come from a hash of the word and `seed`). Words
//! that don't fit are left out.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Word cloud. Map `label` (word) and `weight` (frequency); optionally
/// `color`.
pub struct GeomWordcloud {
    /// Font size range in pixels (by √ of the frequency share).
    pub size_range: (f64, f64),
    /// Share of words drawn vertically (0–1).
    pub rotate_share: f64,
    /// Seed for the rotation choice.
    pub seed: u64,
    /// Colours cycled by rank when `color` is unmapped.
    pub palette: Vec<(u8, u8, u8)>,
    /// Padding around each word's box, px.
    pub padding: f64,
    /// Bold words.
    pub bold: bool,
}

impl Default for GeomWordcloud {
    fn default() -> Self {
        GeomWordcloud {
            size_range: (9.0, 54.0),
            rotate_share: 0.2,
            seed: 1,
            palette: vec![
                (31, 119, 180),
                (255, 127, 14),
                (44, 160, 44),
                (214, 39, 40),
                (148, 103, 189),
                (140, 86, 75),
                (227, 119, 194),
                (23, 190, 207),
            ],
            padding: 1.5,
            bold: true,
        }
    }
}

/// FNV-1a hash of `s` mixed with `seed` → [0, 1).
fn unit_hash(s: &str, seed: u64) -> f64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// Axis-aligned box `(x0, y0, x1, y1)`.
type Box2 = (f64, f64, f64, f64);

fn overlaps(a: &Box2, b: &Box2) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

impl Geom for GeomWordcloud {
    fn draw(
        &self,
        data: &DataFrame,
        _coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let label = data
            .column("label")
            .ok_or(RenderError::MissingAesthetic("label".into()))?;
        let weight = data.column("weight");
        let color_col = data.column("color");
        let area = backend.plot_area();
        let mut words: Vec<(usize, f64)> = (0..data.nrows())
            .filter(|&i| !label[i].is_na() && !super::tip_value(&label[i]).is_empty())
            .map(|i| (i, weight.and_then(|w| w[i].as_f64()).unwrap_or(1.0)))
            .filter(|w| w.1.is_finite() && w.1 > 0.0)
            .collect();
        words.sort_by(|a, b| b.1.total_cmp(&a.1));
        let max_w = words.first().map(|w| w.1).unwrap_or(1.0);
        let (cx, cy) = (area.x + area.width / 2.0, area.y + area.height / 2.0);
        let aspect = (area.width / area.height.max(1.0)).max(0.1);
        let family = super::resolve_font_family("", theme);
        let face = if self.bold {
            FontFace::Bold
        } else {
            FontFace::Plain
        };
        let mut placed: Vec<Box2> = Vec::new();
        for (rank, &(i, w)) in words.iter().enumerate() {
            let word = super::tip_value(&label[i]);
            let (lo, hi) = self.size_range;
            let size = lo + (hi - lo) * (w / max_w).sqrt();
            let vertical = unit_hash(&word, self.seed) < self.rotate_share;
            let (tw, th) = (
                crate::render::text::text_width(&word, size) * if self.bold { 1.08 } else { 1.0 },
                size * 1.05,
            );
            let (bw, bh) = if vertical { (th, tw) } else { (tw, th) };
            let (bw, bh) = (bw + 2.0 * self.padding, bh + 2.0 * self.padding);
            if bw > area.width || bh > area.height {
                continue;
            }
            // Archimedean spiral r = k·θ (k px per radian), walked in ~3 px
            // steps along the arc and stretched to the panel's aspect.
            let mut spot = None;
            let (k, mut theta) = (1.0f64, 0.0f64);
            let r_max = area.width.max(area.height);
            loop {
                let r = k * theta;
                if r > r_max {
                    break;
                }
                let (x, y) = (cx + r * theta.cos() * aspect, cy + r * theta.sin());
                let b = (x - bw / 2.0, y - bh / 2.0, x + bw / 2.0, y + bh / 2.0);
                let inside = b.0 >= area.x
                    && b.1 >= area.y
                    && b.2 <= area.x + area.width
                    && b.3 <= area.y + area.height;
                if inside && !placed.iter().any(|p| overlaps(p, &b)) {
                    spot = Some((x, y, b));
                    break;
                }
                theta += 3.0 / r.max(3.0);
            }
            let Some((x, y, b)) = spot else { continue };
            placed.push(b);
            let color = color_col
                .and_then(|c| scales.map_color(&Aesthetic::Color, &c[i]))
                .unwrap_or_else(|| self.palette[rank % self.palette.len().max(1)]);
            super::set_mark(
                backend,
                Some(format!("{word}: {}", super::tip_value(&Value::Float(w)))),
                Some(word.clone()),
                super::series_key(data, i),
                super::raw_value(&Value::Float(w)),
            );
            super::set_row_extras(backend, data, i);
            backend.draw_text(
                &word,
                (x, y),
                &TextStyle {
                    color,
                    size,
                    anchor: TextAnchor::Middle,
                    angle: if vertical { 90.0 } else { 0.0 },
                    family: family.clone(),
                    face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Label]
    }
    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatIdentity)
    }
    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }
    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }
    fn name(&self) -> &str {
        "wordcloud"
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_seeded() {
        assert_eq!(unit_hash("rust", 1), unit_hash("rust", 1));
        assert_ne!(unit_hash("rust", 1), unit_hash("rust", 2));
        assert!((0.0..1.0).contains(&unit_hash("x", 7)));
        assert!(overlaps(&(0.0, 0.0, 2.0, 2.0), &(1.0, 1.0, 3.0, 3.0)));
        assert!(!overlaps(&(0.0, 0.0, 1.0, 1.0), &(1.0, 0.0, 2.0, 1.0)));
    }
}
