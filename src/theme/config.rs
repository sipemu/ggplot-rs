//! A serde-deserialisable, partial theme overlay (`serde` feature). One config
//! type used by both library callers and the CLI — a `[title] size = 22` TOML/
//! JSON table overrides only that element on top of a base preset, so there's a
//! single schema rather than a hand-rolled parallel one.

use serde::Deserialize;

use super::elements::{ElementLine, ElementRect, ElementText};
use super::presets::{
    theme_bw, theme_classic, theme_dark, theme_gray, theme_light, theme_linedraw, theme_minimal,
    theme_void,
};
use super::{LegendDirection, LegendPosition, TagPosition, Theme, TitlePosition};
use crate::render::backend::{FontFace, Linetype};
use crate::scale::palettes::PaletteName;

/// Partial theme overlay. Every field is optional; only the present ones
/// override the base preset. Deserialise from TOML/JSON, then [`apply`](Self::apply).
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeConfig {
    /// Base preset name to start from.
    pub base: Option<String>,
    /// Brand/primary color `[r, g, b]` for single-series geoms.
    pub primary: Option<[u8; 3]>,
    /// Discrete color/fill palette name (e.g. "Set1", "viridis").
    pub palette: Option<String>,
    /// Fix the panel's height:width ratio (R's `aspect.ratio`).
    pub aspect_ratio: Option<f64>,
    /// Draw gridlines over the data (R's `panel.ontop`).
    pub panel_ontop: Option<bool>,
    /// Draw minor tick marks (R's `axis.minor.ticks`).
    pub axis_minor_ticks: Option<bool>,
    /// Title alignment reference: "panel" or "plot" (R's `plot.title.position`).
    pub title_position: Option<String>,
    /// Tag corner: "topleft", "topright", "bottomleft", "bottomright".
    pub tag_position: Option<String>,
    /// Legend layout: "vertical" or "horizontal" (R's `legend.direction`).
    pub legend_direction: Option<String>,

    /// Root text element — `family`/`color` cascade to all text elements.
    pub text: Option<TextCfg>,
    pub title: Option<TextCfg>,
    pub subtitle: Option<TextCfg>,
    pub caption: Option<TextCfg>,
    pub axis_title_x: Option<TextCfg>,
    pub axis_title_y: Option<TextCfg>,
    pub axis_text_x: Option<TextCfg>,
    pub axis_text_y: Option<TextCfg>,
    pub legend_title: Option<TextCfg>,
    pub legend_text: Option<TextCfg>,
    pub strip_text: Option<TextCfg>,

    pub axis_line: Option<LineCfg>,
    pub axis_ticks: Option<LineCfg>,
    pub panel_grid_major: Option<LineCfg>,
    pub panel_grid_minor: Option<LineCfg>,
    pub panel_border: Option<LineCfg>,

    pub panel_background: Option<RectCfg>,
    pub plot_background: Option<RectCfg>,
    pub legend_background: Option<RectCfg>,
    pub strip_background: Option<RectCfg>,

    pub legend: Option<LegendCfg>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct TextCfg {
    pub family: Option<String>,
    /// Font face: "plain", "bold", or "italic".
    pub face: Option<String>,
    pub size: Option<f64>,
    pub color: Option<[u8; 3]>,
    /// Horizontal justification: 0 = left, 0.5 = center, 1 = right.
    pub hjust: Option<f64>,
    pub vjust: Option<f64>,
    pub angle: Option<f64>,
    pub visible: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct LineCfg {
    pub color: Option<[u8; 3]>,
    pub width: Option<f64>,
    pub visible: Option<bool>,
    /// "solid", "dashed", "dotted", "dashdot", "longdash", "twodash".
    pub linetype: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct RectCfg {
    pub fill: Option<[u8; 3]>,
    pub color: Option<[u8; 3]>,
    pub width: Option<f64>,
    pub visible: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct LegendCfg {
    /// "top", "bottom", "left", "right", "none", or "inside".
    pub position: Option<String>,
    /// Panel-relative coords for `position = "inside"`.
    pub x: Option<f64>,
    pub y: Option<f64>,
}

impl ThemeConfig {
    /// Apply this overlay onto `base` (or onto `self.base`'s preset if set),
    /// returning the resulting theme. Palette (a scale, not a theme) is left for
    /// the caller via [`self.palette`] + [`parse_palette`].
    pub fn apply(&self, base: Theme) -> Result<Theme, String> {
        let mut t = match &self.base {
            Some(name) => preset(name)?,
            None => base,
        };
        text(&self.text, &mut t.text)?;
        text(&self.title, &mut t.title)?;
        text(&self.subtitle, &mut t.subtitle)?;
        text(&self.caption, &mut t.caption)?;
        text(&self.axis_title_x, &mut t.axis_title_x)?;
        text(&self.axis_title_y, &mut t.axis_title_y)?;
        text(&self.axis_text_x, &mut t.axis_text_x)?;
        text(&self.axis_text_y, &mut t.axis_text_y)?;
        text(&self.legend_title, &mut t.legend_title)?;
        text(&self.legend_text, &mut t.legend_text)?;
        text(&self.strip_text, &mut t.strip_text)?;

        line(&self.axis_line, &mut t.axis_line)?;
        line(&self.axis_ticks, &mut t.axis_ticks)?;
        line(&self.panel_grid_major, &mut t.panel_grid_major)?;
        line(&self.panel_grid_minor, &mut t.panel_grid_minor)?;
        line(&self.panel_border, &mut t.panel_border)?;

        rect(&self.panel_background, &mut t.panel_background);
        rect(&self.plot_background, &mut t.plot_background);
        rect(&self.legend_background, &mut t.legend_background);
        rect(&self.strip_background, &mut t.strip_background);

        if let Some([r, g, b]) = self.primary {
            t.primary = Some((r, g, b));
        }
        if let Some(r) = self.aspect_ratio {
            t.aspect_ratio = Some(r);
        }
        if let Some(b) = self.panel_ontop {
            t.panel_ontop = b;
        }
        if let Some(b) = self.axis_minor_ticks {
            t.axis_minor_ticks = b;
        }
        if let Some(p) = &self.title_position {
            t.title_position = match p.to_lowercase().as_str() {
                "panel" => TitlePosition::Panel,
                "plot" => TitlePosition::Plot,
                other => return Err(format!("unknown title_position '{other}'")),
            };
        }
        if let Some(p) = &self.tag_position {
            t.tag_position = match p.to_lowercase().as_str() {
                "topleft" => TagPosition::TopLeft,
                "topright" => TagPosition::TopRight,
                "bottomleft" => TagPosition::BottomLeft,
                "bottomright" => TagPosition::BottomRight,
                other => return Err(format!("unknown tag_position '{other}'")),
            };
        }
        if let Some(d) = &self.legend_direction {
            t.legend_direction = Some(match d.to_lowercase().as_str() {
                "vertical" => LegendDirection::Vertical,
                "horizontal" => LegendDirection::Horizontal,
                other => return Err(format!("unknown legend_direction '{other}'")),
            });
        }
        if let Some(l) = &self.legend {
            t.legend_position = legend_position(l)?;
        }
        Ok(t)
    }
}

/// Map a preset name to its [`Theme`].
pub fn preset(name: &str) -> Result<Theme, String> {
    Ok(match name {
        "gray" | "grey" => theme_gray(),
        "bw" => theme_bw(),
        "minimal" => theme_minimal(),
        "classic" => theme_classic(),
        "dark" => theme_dark(),
        "light" => theme_light(),
        "void" => theme_void(),
        "linedraw" => theme_linedraw(),
        other => return Err(format!("unknown theme preset '{other}'")),
    })
}

fn text(cfg: &Option<TextCfg>, el: &mut ElementText) -> Result<(), String> {
    if let Some(c) = cfg {
        if let Some(v) = &c.family {
            el.family = v.clone();
        }
        if let Some(f) = &c.face {
            el.face = match f.to_lowercase().as_str() {
                "plain" | "normal" => FontFace::Plain,
                "bold" => FontFace::Bold,
                "italic" | "oblique" => FontFace::Italic,
                other => return Err(format!("unknown face '{other}'")),
            };
        }
        if let Some(v) = c.size {
            el.size = v;
        }
        if let Some([r, g, b]) = c.color {
            el.color = (r, g, b);
        }
        if let Some(v) = c.angle {
            el.angle = v;
        }
        if let Some(v) = c.hjust {
            el.hjust = v;
        }
        if let Some(v) = c.vjust {
            el.vjust = v;
        }
        if let Some(v) = c.visible {
            el.visible = v;
        }
    }
    Ok(())
}

fn line(cfg: &Option<LineCfg>, el: &mut ElementLine) -> Result<(), String> {
    if let Some(c) = cfg {
        if let Some([r, g, b]) = c.color {
            el.color = (r, g, b);
        }
        if let Some(v) = c.width {
            el.width = v;
        }
        if let Some(v) = c.visible {
            el.visible = v;
        }
        if let Some(lt) = &c.linetype {
            el.linetype = parse_linetype(lt)?;
        }
    }
    Ok(())
}

fn rect(cfg: &Option<RectCfg>, el: &mut ElementRect) {
    if let Some(c) = cfg {
        if let Some([r, g, b]) = c.fill {
            el.fill = Some((r, g, b));
        }
        if let Some([r, g, b]) = c.color {
            el.color = Some((r, g, b));
        }
        if let Some(v) = c.width {
            el.width = v;
        }
        if let Some(v) = c.visible {
            el.visible = v;
        }
    }
}

fn legend_position(l: &LegendCfg) -> Result<LegendPosition, String> {
    let pos = l.position.as_deref().unwrap_or("right");
    Ok(match pos {
        "top" => LegendPosition::Top,
        "bottom" => LegendPosition::Bottom,
        "left" => LegendPosition::Left,
        "right" => LegendPosition::Right,
        "none" => LegendPosition::None,
        "inside" => LegendPosition::Inside(l.x.unwrap_or(0.85), l.y.unwrap_or(0.85)),
        other => return Err(format!("unknown legend position '{other}'")),
    })
}

fn parse_linetype(s: &str) -> Result<Linetype, String> {
    Ok(match s.to_lowercase().as_str() {
        "solid" => Linetype::Solid,
        "dashed" => Linetype::Dashed,
        "dotted" => Linetype::Dotted,
        "dashdot" => Linetype::DashDot,
        "longdash" => Linetype::LongDash,
        "twodash" => Linetype::TwoDash,
        other => return Err(format!("unknown linetype '{other}'")),
    })
}

/// Parse `"r,g,b"` into an RGB triple.
pub fn parse_rgb(s: &str) -> Result<(u8, u8, u8), String> {
    let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
    if parts.len() != 3 {
        return Err(format!("expected 'r,g,b', got '{s}'"));
    }
    let c = |p: &str| {
        p.parse::<u8>()
            .map_err(|_| format!("bad color component '{p}'"))
    };
    Ok((c(parts[0])?, c(parts[1])?, c(parts[2])?))
}

/// Parse a palette name (case-insensitive) into a [`PaletteName`].
pub fn parse_palette(name: &str) -> Result<PaletteName, String> {
    use PaletteName::*;
    Ok(match name.to_lowercase().as_str() {
        "set1" => Set1,
        "set2" => Set2,
        "set3" => Set3,
        "dark2" => Dark2,
        "paired" => Paired,
        "pastel1" => Pastel1,
        "pastel2" => Pastel2,
        "accent" => Accent,
        "blues" => Blues,
        "greens" => Greens,
        "reds" => Reds,
        "oranges" => Oranges,
        "purples" => Purples,
        "greys" | "grays" => Greys,
        "ylorrd" => YlOrRd,
        "ylgnbu" => YlGnBu,
        "bugn" => BuGn,
        "bupu" => BuPu,
        "gnbu" => GnBu,
        "orrd" => OrRd,
        "purd" => PuRd,
        "rdpu" => RdPu,
        "ylgn" => YlGn,
        "ylorbr" => YlOrBr,
        "rdbu" => RdBu,
        "spectral" => Spectral,
        "piyg" => PiYG,
        "prgn" => PRGn,
        "brbg" => BrBG,
        "puor" => PuOr,
        "rdgy" => RdGy,
        "rdylbu" => RdYlBu,
        "rdylgn" => RdYlGn,
        "viridis" => Viridis,
        "magma" => Magma,
        "plasma" => Plasma,
        "inferno" => Inferno,
        other => return Err(format!("unknown palette '{other}'")),
    })
}
