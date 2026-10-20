/// Text element styling.
#[derive(Clone, Debug)]
pub struct ElementText {
    pub family: String,
    /// Font face (R's `element_text(face = ...)`).
    pub face: crate::render::backend::FontFace,
    pub size: f64,
    pub color: (u8, u8, u8),
    pub angle: f64,
    pub hjust: f64,
    pub vjust: f64,
    pub visible: bool,
}

impl ElementText {
    /// Create an invisible (blank) text element.
    pub fn blank() -> Self {
        ElementText {
            visible: false,
            ..Default::default()
        }
    }
}

impl Default for ElementText {
    fn default() -> Self {
        ElementText {
            family: "sans-serif".to_string(),
            face: crate::render::backend::FontFace::Plain,
            size: 12.0,
            color: (0, 0, 0),
            angle: 0.0,
            hjust: 0.5,
            vjust: 0.5,
            visible: true,
        }
    }
}

/// Line element styling.
#[derive(Clone, Debug)]
pub struct ElementLine {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub visible: bool,
    /// Line style (R's `element_line(linetype = ...)`).
    pub linetype: crate::render::backend::Linetype,
}

impl ElementLine {
    /// Create an invisible (blank) line element.
    pub fn blank() -> Self {
        ElementLine {
            visible: false,
            ..Default::default()
        }
    }
}

impl Default for ElementLine {
    fn default() -> Self {
        ElementLine {
            color: (0, 0, 0),
            width: 1.0,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        }
    }
}

/// Rectangle element styling.
#[derive(Clone, Debug)]
pub struct ElementRect {
    pub fill: Option<(u8, u8, u8)>,
    pub color: Option<(u8, u8, u8)>,
    pub width: f64,
    pub visible: bool,
}

impl ElementRect {
    /// Create an invisible (blank) rect element.
    pub fn blank() -> Self {
        ElementRect {
            visible: false,
            ..Default::default()
        }
    }
}

impl Default for ElementRect {
    fn default() -> Self {
        ElementRect {
            fill: Some((255, 255, 255)),
            color: None,
            width: 0.0,
            visible: true,
        }
    }
}

/// Geom "ink"/"paper"/"accent" palette (ggplot2 4.0's `element_geom`,
/// B2-7): recolours a geom's *built-in default* colour without touching
/// every geom's own color/fill field or needing a mapped/explicit colour
/// to be set first. `None` everywhere (the default) changes nothing —
/// byte-identical output. Marked `#[non_exhaustive]` so a later field can
/// be added without an `..Default::default()` break.
///
/// A default [`Geom::apply_element_geom`](crate::geom::Geom::apply_element_geom)
/// is a no-op; a geom that honours this overrides it, replacing only
/// fields still at their own built-in default — called once per layer,
/// before the brand [`Theme::primary`](super::Theme::primary) colour is
/// applied, and (like the brand colour) only for a layer that was *not*
/// configured explicitly via `geom_*_with(...)`: that is the actual guard
/// against touching an explicit colour (an `explicit_style` gate in the
/// build pipeline), not a comparison of the field's *value* against the
/// hard-coded default, which can't otherwise tell an explicitly-set black
/// apart from a black nobody touched.
///
/// **Geoms covered (0.21.0):** `GeomPoint`, `GeomLine`, `GeomPath`,
/// `GeomText`, `GeomLabel` and `GeomBoxplot` (`ink` for a black/
/// `(50, 50, 50)` line, text or outline; `paper` for a white fill) and
/// `GeomSmooth` (`accent` for its default blue line and ribbon fill) — plus
/// the legend key swatch colour, which reads `ink` directly rather than
/// going through this trait method. Every other geom (segment, curve,
/// reference lines, errorbar/linerange/pointrange, step, rect/tile
/// outlines, density/violin/area outlines, crossbar, …) does not yet
/// honour `ElementGeom` and keeps its hard-coded colour regardless —
/// tracked at sipemu/ggplot-rs#142.
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct ElementGeom {
    /// Replaces a geom's built-in black / `(50, 50, 50)` line, text or
    /// outline colour (including the legend key colour).
    pub ink: Option<(u8, u8, u8)>,
    /// Replaces a geom's built-in white fill.
    pub paper: Option<(u8, u8, u8)>,
    /// Replaces the smooth line's built-in colour.
    pub accent: Option<(u8, u8, u8)>,
    /// Font family — the middle tier of B2-6's resolution order
    /// (a geom's own `fontfamily`, then this, then `theme.text.family`).
    pub family: Option<String>,
}

impl ElementGeom {
    /// Builder for [`ink`](Self::ink) — `#[non_exhaustive]` blocks a
    /// cross-crate struct literal even with `..Default::default()`, so
    /// each field gets one of these.
    pub fn with_ink(mut self, color: (u8, u8, u8)) -> Self {
        self.ink = Some(color);
        self
    }

    /// Builder for [`paper`](Self::paper).
    pub fn with_paper(mut self, color: (u8, u8, u8)) -> Self {
        self.paper = Some(color);
        self
    }

    /// Builder for [`accent`](Self::accent).
    pub fn with_accent(mut self, color: (u8, u8, u8)) -> Self {
        self.accent = Some(color);
        self
    }

    /// Builder for [`family`](Self::family).
    pub fn with_family(mut self, family: &str) -> Self {
        self.family = Some(family.to_string());
        self
    }
}

#[cfg(test)]
mod element_geom_tests {
    use super::ElementGeom;

    #[test]
    fn builders_set_every_field() {
        let el = ElementGeom::default()
            .with_ink((1, 2, 3))
            .with_paper((4, 5, 6))
            .with_accent((7, 8, 9))
            .with_family("Georgia");
        assert_eq!(el.ink, Some((1, 2, 3)));
        assert_eq!(el.paper, Some((4, 5, 6)));
        assert_eq!(el.accent, Some((7, 8, 9)));
        assert_eq!(el.family, Some("Georgia".to_string()));
    }
}
