//! Custom theming for the CLI: a TOML/JSON config of element overrides applied
//! on top of a base preset, plus palette and brand-color helpers.

use ggplot_rs::prelude::*;
use serde::Deserialize;

/// A theme config file (`--theme-config`). Every field is optional; only the
/// ones present override the base preset.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeConfig {
    /// Base preset to start from (overrides `--theme` when set).
    pub base: Option<String>,
    /// Brand/primary color `[r, g, b]` for single-series geoms.
    pub primary: Option<[u8; 3]>,
    /// Discrete color/fill palette name (e.g. "Set1", "viridis").
    pub palette: Option<String>,

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
    pub size: Option<f64>,
    pub color: Option<[u8; 3]>,
    pub angle: Option<f64>,
    pub visible: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct LineCfg {
    pub color: Option<[u8; 3]>,
    pub width: Option<f64>,
    pub visible: Option<bool>,
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

/// Parse a theme config from a `.toml` or `.json` file.
pub fn load(path: &str) -> Result<ThemeConfig, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    if path.ends_with(".json") {
        serde_json::from_str(&text).map_err(|e| format!("parsing {path}: {e}"))
    } else {
        toml::from_str(&text).map_err(|e| format!("parsing {path}: {e}"))
    }
}

/// Map a preset name to its `Theme`.
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

/// Apply the config's element overrides onto `base`.
pub fn apply(cfg: &ThemeConfig, mut t: Theme) -> Result<Theme, String> {
    text(&cfg.title, &mut t.title);
    text(&cfg.subtitle, &mut t.subtitle);
    text(&cfg.caption, &mut t.caption);
    text(&cfg.axis_title_x, &mut t.axis_title_x);
    text(&cfg.axis_title_y, &mut t.axis_title_y);
    text(&cfg.axis_text_x, &mut t.axis_text_x);
    text(&cfg.axis_text_y, &mut t.axis_text_y);
    text(&cfg.legend_title, &mut t.legend_title);
    text(&cfg.legend_text, &mut t.legend_text);
    text(&cfg.strip_text, &mut t.strip_text);

    line(&cfg.axis_line, &mut t.axis_line);
    line(&cfg.axis_ticks, &mut t.axis_ticks);
    line(&cfg.panel_grid_major, &mut t.panel_grid_major);
    line(&cfg.panel_grid_minor, &mut t.panel_grid_minor);
    line(&cfg.panel_border, &mut t.panel_border);

    rect(&cfg.panel_background, &mut t.panel_background);
    rect(&cfg.plot_background, &mut t.plot_background);
    rect(&cfg.legend_background, &mut t.legend_background);
    rect(&cfg.strip_background, &mut t.strip_background);

    if let Some([r, g, b]) = cfg.primary {
        t.primary = Some((r, g, b));
    }
    if let Some(l) = &cfg.legend {
        t.legend_position = legend_position(l)?;
    }
    Ok(t)
}

fn text(cfg: &Option<TextCfg>, el: &mut ElementText) {
    if let Some(c) = cfg {
        if let Some(v) = &c.family {
            el.family = v.clone();
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
        if let Some(v) = c.visible {
            el.visible = v;
        }
    }
}

fn line(cfg: &Option<LineCfg>, el: &mut ElementLine) {
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
    }
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

/// Parse a palette name (case-insensitive).
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
