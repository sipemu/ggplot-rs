//! Legend key glyphs (#410): ggplot2's `draw_key_*` — which picture a
//! layer contributes to a legend key — chosen per geom (`Geom::key_glyph`)
//! or per layer (`GGPlot::key_glyph`), and several layers drawn into one key.

use crate::aes::Aesthetic;
use crate::render::backend::{
    DrawBackend, LineStyle, Linetype, PointShape, PointStyle, RectStyle, TextAnchor, TextStyle,
};
use crate::render::RenderError;

/// The picture a layer draws in a legend key (ggplot2's `key_glyph =`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyGlyph {
    /// A point (`draw_key_point`).
    Point,
    /// A horizontal line (`draw_key_path`).
    Path,
    /// A filled square (`draw_key_rect`).
    Rect,
    /// A filled square with an outline (`draw_key_polygon`).
    Polygon,
    /// A small boxplot (`draw_key_boxplot`).
    Boxplot,
    /// A crossbar (`draw_key_crossbar`).
    Crossbar,
    /// A vertical line with a point (`draw_key_pointrange`).
    Pointrange,
    /// A vertical line (`draw_key_linerange`).
    Linerange,
    /// A line over a light ribbon (`draw_key_smooth`).
    Smooth,
    /// The letter "a" (`draw_key_text`).
    Text,
    /// "a" on a box (`draw_key_label`).
    Label,
    /// A vertical line (`draw_key_vline`).
    Vline,
    /// A vertical path (`draw_key_vpath`).
    Vpath,
    /// A diagonal line (`draw_key_abline`).
    Abline,
    /// A zig-zag line (`draw_key_timeseries`).
    Timeseries,
    /// A filled dot (`draw_key_dotplot`).
    Dotplot,
    /// Nothing (`draw_key_blank`).
    Blank,
}

impl KeyGlyph {
    /// Parse ggplot2's glyph names (`"point"`, `"path"`, `"boxplot"`, …).
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "point" => KeyGlyph::Point,
            "path" => KeyGlyph::Path,
            "rect" => KeyGlyph::Rect,
            "polygon" => KeyGlyph::Polygon,
            "boxplot" => KeyGlyph::Boxplot,
            "crossbar" => KeyGlyph::Crossbar,
            "pointrange" => KeyGlyph::Pointrange,
            "linerange" => KeyGlyph::Linerange,
            "smooth" => KeyGlyph::Smooth,
            "text" => KeyGlyph::Text,
            "label" => KeyGlyph::Label,
            "vline" => KeyGlyph::Vline,
            "vpath" => KeyGlyph::Vpath,
            "abline" => KeyGlyph::Abline,
            "timeseries" => KeyGlyph::Timeseries,
            "dotplot" => KeyGlyph::Dotplot,
            "blank" => KeyGlyph::Blank,
            _ => return None,
        })
    }
}

/// One layer's contribution to the legends: its glyph (`None` = the
/// pre-#410 per-aesthetic swatch) and the legend aesthetics it maps and shows.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerKey {
    pub glyph: Option<KeyGlyph>,
    pub aesthetics: Vec<Aesthetic>,
    /// The layer's constant fill and alpha ([`Geom::key_fill`](crate::geom::Geom::key_fill), #473).
    pub fill: Option<((u8, u8, u8), f64)>,
}

/// The mapped values a key is drawn with (each `None` = not mapped in this
/// legend → the glyph's default).
#[derive(Clone, Debug, Default)]
pub(crate) struct KeyProps {
    pub color: Option<(u8, u8, u8)>,
    pub fill: Option<(u8, u8, u8)>,
    pub shape: Option<PointShape>,
    pub linetype: Option<Linetype>,
    pub size: Option<f64>,
    pub alpha: Option<f64>,
    pub linewidth: Option<f64>,
}

/// Draw `glyph` into the key square at `(x, y)` of side `s`.
pub(crate) fn draw_key(
    backend: &mut dyn DrawBackend,
    glyph: KeyGlyph,
    props: &KeyProps,
    ink: (u8, u8, u8),
    (x, y, s): (f64, f64, f64),
) -> Result<(), RenderError> {
    let color = props.color.unwrap_or(ink);
    // A rect key is coloured by fill, else colour; a polygon key fills with
    // its fill (default grey35, as ggplot2's bars) and outlines in colour.
    let fill = props.fill.or(props.color).unwrap_or((89, 89, 89));
    let poly_fill = props.fill.unwrap_or((89, 89, 89));
    let alpha = props.alpha.unwrap_or(1.0);
    let lw = props.linewidth.unwrap_or(1.0).min(s);
    let lt = props.linetype.clone().unwrap_or(Linetype::Solid);
    let line = |w: f64| LineStyle {
        color,
        width: w,
        alpha,
        linetype: lt.clone(),
        ..Default::default()
    };
    let (cx, cy) = (x + s / 2.0, y + s / 2.0);
    let at = |fx: f64, fy: f64| (x + fx * s, y + fy * s);
    let rect = |fill: Option<(u8, u8, u8)>, stroke: Option<(u8, u8, u8)>| RectStyle {
        fill,
        stroke,
        stroke_width: lw.max(0.5),
        alpha,
        clip: false,
        corner_radius: 0.0,
    };
    let point = |backend: &mut dyn DrawBackend, r: f64| {
        let shape = props.shape.unwrap_or(PointShape::Circle);
        backend.draw_shape(
            (cx, cy),
            r,
            &PointStyle {
                color,
                alpha,
                filled: true,
                shape,
            },
        )
    };
    match glyph {
        KeyGlyph::Point => point(backend, props.size.unwrap_or(s / 5.0).min(s / 2.0))?,
        KeyGlyph::Dotplot => point(backend, s / 3.0)?,
        KeyGlyph::Path => backend.draw_line(&[at(0.1, 0.5), at(0.9, 0.5)], &line(lw))?,
        KeyGlyph::Vline | KeyGlyph::Linerange | KeyGlyph::Vpath => {
            backend.draw_line(&[at(0.5, 0.1), at(0.5, 0.9)], &line(lw))?
        }
        KeyGlyph::Abline => backend.draw_line(&[at(0.0, 1.0), at(1.0, 0.0)], &line(lw))?,
        KeyGlyph::Timeseries => backend.draw_line(
            &[at(0.0, 0.9), at(0.4, 0.1), at(0.6, 0.6), at(1.0, 0.1)],
            &line(lw),
        )?,
        KeyGlyph::Rect => backend.draw_rect(at(0.0, 0.0), at(1.0, 1.0), &rect(Some(fill), None))?,
        KeyGlyph::Polygon => backend.draw_rect(
            at(0.0, 0.0),
            at(1.0, 1.0),
            &rect(Some(poly_fill), props.color),
        )?,
        KeyGlyph::Smooth => {
            let band = props.fill.or(props.color).unwrap_or((153, 153, 153));
            backend.draw_rect(
                at(0.0, 0.0),
                at(1.0, 1.0),
                &RectStyle {
                    alpha: 0.4,
                    ..rect(Some(band), None)
                },
            )?;
            backend.draw_line(&[at(0.1, 0.5), at(0.9, 0.5)], &line(lw.max(1.0)))?;
        }
        KeyGlyph::Boxplot => {
            let fill = props.fill.unwrap_or((255, 255, 255));
            backend.draw_line(&[at(0.5, 0.1), at(0.5, 0.25)], &line(0.5))?;
            backend.draw_line(&[at(0.5, 0.75), at(0.5, 0.9)], &line(0.5))?;
            backend.draw_rect(
                at(0.125, 0.25),
                at(0.875, 0.75),
                &rect(Some(fill), Some(color)),
            )?;
            backend.draw_line(&[at(0.125, 0.5), at(0.875, 0.5)], &line(1.0))?;
        }
        KeyGlyph::Crossbar => {
            let fill = props.fill.unwrap_or((255, 255, 255));
            backend.draw_rect(
                at(0.125, 0.25),
                at(0.875, 0.75),
                &rect(Some(fill), Some(color)),
            )?;
            backend.draw_line(&[at(0.125, 0.5), at(0.875, 0.5)], &line(1.0))?;
        }
        KeyGlyph::Pointrange => {
            backend.draw_line(&[at(0.5, 0.1), at(0.5, 0.9)], &line(lw))?;
            point(backend, props.size.unwrap_or(s / 6.0).min(s / 3.0))?;
        }
        KeyGlyph::Text | KeyGlyph::Label => {
            if glyph == KeyGlyph::Label {
                backend.draw_rect(
                    at(0.1, 0.15),
                    at(0.9, 0.85),
                    &RectStyle {
                        fill: Some((255, 255, 255)),
                        stroke: Some(color),
                        stroke_width: 0.5,
                        alpha,
                        clip: false,
                        corner_radius: 1.5,
                    },
                )?;
            }
            backend.draw_text(
                "a",
                (cx, cy + s * 0.18),
                &TextStyle {
                    color,
                    size: s * 0.6,
                    anchor: TextAnchor::Middle,
                    ..Default::default()
                },
            )?;
        }
        KeyGlyph::Blank => {}
    }
    Ok(())
}
