//! sipemu/ggplot-rs#362: the chart-type shortcuts build their default geom
//! through `geom_*_with`, which marked the layer as explicitly styled — so
//! the defaults ignored `primary_color` and the theme's ink/paper. The
//! shortcuts now style like `geom_line()`; `*_with` stays explicit.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::theme::presets::theme_gray;

const PRIMARY: (u8, u8, u8) = (0x11, 0x22, 0x33);
const INK: (u8, u8, u8) = (0x01, 0x02, 0x03);
const PAPER: (u8, u8, u8) = (0xFE, 0xFD, 0xFC);

fn s(v: &[&str]) -> Vec<Value> {
    v.iter().map(|x| Value::Str(x.to_string())).collect()
}
fn f(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&x| Value::Float(x)).collect()
}

fn svg(p: GGPlot) -> String {
    p.render_svg_native_with_size(500, 400).unwrap()
}
fn branded(p: GGPlot) -> String {
    svg(p.primary_color(PRIMARY))
}
fn inked(p: GGPlot) -> String {
    svg(p.theme(theme_gray().with_ink_paper(INK, PAPER)))
}

fn waterfall() -> GGPlot {
    GGPlot::new(vec![
        ("step".into(), s(&["Revenue", "COGS", "Opex"])),
        ("v".into(), f(&[100.0, -40.0, -25.0])),
    ])
    .aes(Aes::new().x("step").y("v"))
}
fn stages() -> GGPlot {
    GGPlot::new(vec![
        ("stage".into(), s(&["Visits", "Sign-ups", "Paid"])),
        ("n".into(), f(&[5000.0, 1800.0, 250.0])),
    ])
    .aes(Aes::new().x("stage").y("n"))
}
fn hierarchy() -> GGPlot {
    GGPlot::new(vec![
        ("a".into(), s(&["EU", "EU", "AS"])),
        ("b".into(), s(&["DE", "FR", "JP"])),
        ("w".into(), f(&[3.0, 2.0, 4.0])),
    ])
    .aes(Aes::new().weight("w"))
}
fn seats() -> GGPlot {
    GGPlot::new(vec![
        ("party".into(), s(&["A", "B"])),
        ("seats".into(), f(&[12.0, 8.0])),
    ])
    .aes(Aes::new().weight("seats"))
}
fn bubbles() -> GGPlot {
    GGPlot::new(vec![
        ("name".into(), s(&["a", "b", "c"])),
        ("w".into(), f(&[3.0, 2.0, 1.0])),
    ])
    .aes(Aes::new().weight("w").label("name"))
}
fn org() -> GGPlot {
    GGPlot::new(vec![
        ("boss".into(), s(&["CEO", "CEO", "CTO"])),
        ("report".into(), s(&["CTO", "CFO", "Dev"])),
    ])
    .aes(Aes::new().from("boss").to("report"))
}
fn venn() -> GGPlot {
    GGPlot::new(vec![
        ("s".into(), s(&["A", "B", "A&B"])),
        ("n".into(), f(&[100.0, 60.0, 25.0])),
    ])
    .aes(Aes::new().sets("s").weight("n"))
}
fn mekko() -> GGPlot {
    GGPlot::new(vec![
        ("country".into(), s(&["X", "Y", "Z"])),
        ("gdp".into(), f(&[10.0, 20.0, 15.0])),
        ("pop".into(), f(&[5.0, 2.0, 3.0])),
    ])
    .aes(Aes::new().x("country").y("gdp").weight("pop"))
}
fn tiles() -> GGPlot {
    GGPlot::new(vec![
        ("c".into(), f(&[0.0, 1.0, 0.0])),
        ("r".into(), f(&[0.0, 0.0, 1.0])),
        ("st".into(), s(&["AA", "BB", "CC"])),
    ])
    .aes(Aes::new().x("c").y("r").label("st"))
}

const BRAND: &str = "#112233";
const INK_HEX: &str = "#010203";
const PAPER_HEX: &str = "#FEFDFC";

#[test]
fn primary_color_reaches_single_series_shortcuts() {
    for (name, out) in [
        ("waffle", branded(seats().geom_waffle(5))),
        ("parliament", branded(seats().geom_parliament(2))),
        ("packed_bubble", branded(bubbles().geom_packed_bubble())),
        ("tree", branded(org().geom_tree())),
        ("variwide", branded(mekko().geom_variwide())),
        ("tilemap", branded(tiles().geom_tilemap(Default::default()))),
    ] {
        assert!(out.contains(BRAND), "{name}: brand colour not applied");
    }
}

#[test]
fn ink_and_paper_reach_shortcut_defaults() {
    for (name, out, hex) in [
        ("waterfall", inked(waterfall().geom_waterfall()), INK_HEX),
        ("funnel", inked(stages().geom_funnel()), INK_HEX),
        (
            // Narrow, so the labels sit outside the shape, in ink.
            "pyramid",
            stages()
                .geom_pyramid()
                .theme(theme_gray().with_ink_paper(INK, PAPER))
                .render_svg_native_with_size(160, 400)
                .unwrap(),
            INK_HEX,
        ),
        ("venn", inked(venn().geom_venn()), INK_HEX),
        ("tree", inked(org().geom_tree()), INK_HEX),
        (
            "sunburst",
            inked(hierarchy().geom_sunburst(["a", "b"])),
            PAPER_HEX,
        ),
        (
            "icicle",
            inked(hierarchy().geom_icicle(["a", "b"])),
            PAPER_HEX,
        ),
        ("marimekko", inked(mekko().geom_marimekko()), PAPER_HEX),
    ] {
        assert!(out.contains(hex), "{name}: theme ink/paper not applied");
    }
}

#[test]
fn with_builders_stay_explicit() {
    let out = branded(seats().geom_waffle_with(ggplot_rs::geom::waffle::GeomWaffle::waffle(5)));
    assert!(
        !out.contains(BRAND),
        "an explicit geom keeps its own colours"
    );
}
