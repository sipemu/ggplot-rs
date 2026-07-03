//! CLI theming: reuse the library's serde theme config, adding only file I/O.

pub use ggplot_rs::theme::config::{parse_palette, parse_rgb, preset, ThemeConfig};

/// Parse a theme config from a `.toml` or `.json` file.
pub fn load(path: &str) -> Result<ThemeConfig, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    if path.ends_with(".json") {
        serde_json::from_str(&text).map_err(|e| format!("parsing {path}: {e}"))
    } else {
        toml::from_str(&text).map_err(|e| format!("parsing {path}: {e}"))
    }
}
