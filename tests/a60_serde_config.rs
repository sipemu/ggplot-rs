//! A-grade #60 — the library's serde ThemeConfig overlay applies onto a preset.
#![cfg(feature = "serde")]

use ggplot_rs::prelude::*;
use ggplot_rs::theme::config::{GeomCfg, LineCfg, TextCfg, ThemeConfig};

#[test]
fn config_overrides_only_specified_elements() {
    let cfg = ThemeConfig {
        title: Some(TextCfg {
            face: Some("bold".into()),
            size: Some(20.0),
            ..Default::default()
        }),
        panel_grid_minor: Some(LineCfg {
            visible: Some(false),
            ..Default::default()
        }),
        aspect_ratio: Some(1.0),
        ..Default::default()
    };
    let theme = cfg.apply(theme_minimal()).unwrap();

    assert_eq!(theme.title.face, FontFace::Bold);
    assert_eq!(theme.title.size, 20.0);
    assert!(!theme.panel_grid_minor.visible);
    assert_eq!(theme.aspect_ratio, Some(1.0));
    // An element the config didn't touch keeps the preset's value.
    assert_eq!(theme.axis_text_x.face, FontFace::Plain);
}

#[test]
fn config_base_field_switches_preset() {
    let cfg = ThemeConfig {
        base: Some("void".into()),
        ..Default::default()
    };
    // base="void" overrides the passed-in gray base.
    let theme = cfg.apply(theme_gray()).unwrap();
    assert!(!theme.axis_text_x.visible, "theme_void hides axis text");
}

#[test]
fn geom_config_keys_set_ink_paper_accent_and_family() {
    let cfg = ThemeConfig {
        geom: Some(GeomCfg {
            ink: Some([255, 255, 255]),
            paper: Some([0, 0, 0]),
            accent: Some([10, 20, 30]),
            family: Some("Georgia".to_string()),
        }),
        ..Default::default()
    };
    let theme = cfg.apply(Theme::default()).unwrap();
    assert_eq!(theme.geom.ink, Some((255, 255, 255)));
    assert_eq!(theme.geom.paper, Some((0, 0, 0)));
    assert_eq!(theme.geom.accent, Some((10, 20, 30)));
    assert_eq!(theme.geom.family, Some("Georgia".to_string()));
}

#[test]
fn geom_config_absent_leaves_the_default_untouched() {
    let theme = ThemeConfig::default().apply(Theme::default()).unwrap();
    assert_eq!(theme.geom.ink, None);
    assert_eq!(theme.geom.paper, None);
    assert_eq!(theme.geom.accent, None);
    assert_eq!(theme.geom.family, None);
}
