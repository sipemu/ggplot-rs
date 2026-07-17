//! ggpubr-parity basics: ggsci journal palettes + theme_pubr().

use ggplot_rs::scale::palettes::{palette, PaletteName};
use ggplot_rs::theme::presets::theme_pubr;
use ggplot_rs::theme::LegendPosition;

/// (variant, expected color count) — ggsci canonical sizes.
const CASES: &[(PaletteName, usize)] = &[
    (PaletteName::Npg, 10),
    (PaletteName::Aaas, 10),
    (PaletteName::Nejm, 8),
    (PaletteName::Lancet, 9),
    (PaletteName::Jama, 7),
    (PaletteName::Jco, 10),
    (PaletteName::D3, 10),
];

#[test]
fn ggsci_palettes_resolve_with_expected_lengths() {
    for (variant, want_len) in CASES {
        assert_eq!(palette(variant).len(), *want_len, "{variant:?} length");
    }
}

#[test]
fn npg_first_color_is_nature_red() {
    // ggsci pal_npg("nrc")[1] == "#E64B35".
    let first = palette(&PaletteName::Npg)[0];
    assert_eq!((first.r, first.g, first.b), (230, 75, 53));
}

#[test]
fn theme_pubr_is_publication_style() {
    let t = theme_pubr();
    // Legend on top, no gridlines, visible black axis lines (classic base).
    assert!(matches!(t.legend_position, LegendPosition::Top));
    assert!(!t.panel_grid_major.visible, "pubr has no major grid");
    assert!(!t.panel_grid_minor.visible, "pubr has no minor grid");
    assert!(t.axis_line.visible, "pubr draws axis lines");
}

/// The ggsci names must be parseable by the (serde-gated) CLI palette parser.
#[cfg(feature = "serde")]
#[test]
fn ggsci_names_parse() {
    use ggplot_rs::theme::config::parse_palette;
    for name in ["npg", "aaas", "nejm", "lancet", "jama", "jco", "d3"] {
        assert!(parse_palette(name).is_ok(), "{name} should parse");
    }
    assert!(parse_palette("not-a-palette").is_err());
}
