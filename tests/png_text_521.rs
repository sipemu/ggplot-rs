//! #521 — the plotters PNG path sets bold / italic faces, centres rotated
//! text on its anchor, and keeps a rotated rich-text title's runs on one line.
#![cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
use ggplot_rs::prelude::*;

const W: u32 = 480;
const H: u32 = 360;

fn plot(title: &str, ylab: &str, face: FontFace, rich: bool) -> image::RgbaImage {
    let d = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ];
    let mut t = theme_minimal();
    t.title.face = face;
    t.rich_text = rich;
    let png = GGPlot::new(d)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme(t)
        .title(title)
        .ylab(ylab)
        .render_png_with_size(W, H)
        .unwrap();
    image::load_from_memory(&png).unwrap().to_rgba8()
}

fn dark(img: &image::RgbaImage, x: u32, y: u32) -> bool {
    let p = img.get_pixel(x, y);
    (p[0] as u32 + p[1] as u32 + p[2] as u32) < 3 * 160
}

/// Dark pixels in the title band (the top 30 px).
fn title_ink(img: &image::RgbaImage) -> usize {
    (0..30)
        .flat_map(|y| (0..W).map(move |x| (x, y)))
        .filter(|&(x, y)| dark(img, x, y))
        .count()
}

/// The first and last row with ink in the y title's strip (the left 22 px).
fn y_title_extent(img: &image::RgbaImage) -> (u32, u32) {
    let rows: Vec<u32> = (0..H)
        .filter(|&y| (0..22).any(|x| dark(img, x, y)))
        .collect();
    (*rows.first().unwrap(), *rows.last().unwrap())
}

#[test]
fn a_bold_title_has_more_ink_than_a_plain_one() {
    let plain = title_ink(&plot("FACEMARK title", "y", FontFace::Plain, false));
    let bold = title_ink(&plot("FACEMARK title", "y", FontFace::Bold, false));
    assert!(
        bold as f64 > plain as f64 * 1.2,
        "bold {bold} vs plain {plain}"
    );
}

#[test]
fn an_italic_title_is_drawn_with_the_italic_face() {
    let plain = plot("FACEMARK title", "y", FontFace::Plain, false);
    let italic = plot("FACEMARK title", "y", FontFace::Italic, false);
    let differ = (0..30)
        .flat_map(|y| (0..W).map(move |x| (x, y)))
        .filter(|&(x, y)| dark(&plain, x, y) != dark(&italic, x, y))
        .count();
    assert!(differ > 50, "{differ} pixels differ");
}

#[test]
fn a_rotated_y_title_is_centred_on_the_panel() {
    let (top, bottom) = y_title_extent(&plot(
        "t",
        "A long y axis title here",
        FontFace::Plain,
        false,
    ));
    let mid = (top + bottom) as f64 / 2.0;
    // the panel's middle sits a little above the image's (the x axis below)
    assert!((mid - H as f64 / 2.0).abs() < 20.0, "title {top}..{bottom}");
}

#[test]
fn a_rotated_rich_title_keeps_its_runs_on_one_line() {
    let rich = y_title_extent(&plot("t", "Response, log~10~ units", FontFace::Plain, true));
    let plain = y_title_extent(&plot("t", "Response, log10 units", FontFace::Plain, false));
    let (lr, lp) = ((rich.1 - rich.0) as f64, (plain.1 - plain.0) as f64);
    assert!(
        (lr - lp).abs() < lp * 0.15,
        "rich {rich:?} vs plain {plain:?}"
    );
    let (mr, mp) = (
        (rich.0 + rich.1) as f64 / 2.0,
        (plain.0 + plain.1) as f64 / 2.0,
    );
    assert!((mr - mp).abs() < 10.0, "rich {rich:?} vs plain {plain:?}");
}

#[test]
fn rich_bold_and_italic_runs_take_their_faces() {
    let marked = title_ink(&plot(
        "**Bold** words and *more*",
        "y",
        FontFace::Plain,
        true,
    ));
    let plain = title_ink(&plot("Bold words and more", "y", FontFace::Plain, true));
    assert!(marked > plain, "marked {marked} vs plain {plain}");
}
