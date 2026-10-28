//! `annotation_map_tile` (#325): XYZ tiles aligned with Web-Mercator
//! `geom_sf` geometry, drawn under the data, on every backend.
#![cfg(feature = "tiles")]

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::map_tile::{tile_xy, TileSource};
use ggplot_rs::geom::sf::GeomSf;
use ggplot_rs::spatial::SfProjection;
use ggplot_rs::GGPlot;

fn cities() -> Vec<(String, Vec<Value>)> {
    let pts = [
        ("Berlin", 13.40, 52.52),
        ("Paris", 2.35, 48.86),
        ("Rome", 12.50, 41.90),
    ];
    vec![
        (
            "geometry".into(),
            pts.iter()
                .map(|(_, lon, lat)| Value::Str(format!("POINT ({lon} {lat})")))
                .collect(),
        ),
        (
            "city".into(),
            pts.iter()
                .map(|(c, ..)| Value::Str(c.to_string()))
                .collect(),
        ),
    ]
}

fn map(fetch: impl Fn(u32, u32, u32) -> Option<Vec<u8>> + Send + Sync + 'static) -> GGPlot {
    GGPlot::new(cities())
        .aes(Aes::new().label("city"))
        .geom_sf_with(GeomSf::default().project(SfProjection::Mercator))
        .annotation_map_tile(
            TileSource::xyz("https://tiles.example/{z}/{x}/{y}.png").attribution("© Example"),
            fetch,
        )
        .coord_sf()
}

fn attr(tag: &str, name: &str) -> f64 {
    tag.split(&format!(" {name}=\""))
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn svg_tiles_line_up_with_the_projected_points() {
    // Each tile's bytes name it, so the test can find the one under Berlin.
    let svg = map(|z, x, y| Some(format!("{z}/{x}/{y}").into_bytes()))
        .render_svg_native_with_size(500, 400)
        .unwrap();
    assert!(svg.contains("© Example"), "attribution");
    let first_image = svg.find("<image").expect("tiles drawn");
    let first_circle = svg.find("<circle").expect("points drawn");
    assert!(first_image < first_circle, "tiles under the data");

    // Berlin's marker (the first point).
    let circle = &svg[first_circle..];
    let (cx, cy) = (attr(circle, "cx"), attr(circle, "cy"));

    // Every tile: its nested clip <svg x y …> then <image x y w h data-tile>.
    let mut found = false;
    for chunk in svg.split("<svg x=\"").skip(1) {
        let Some(img) = chunk.find("<image").map(|i| &chunk[i..]) else {
            continue;
        };
        let clip_x: f64 = chunk.split('"').next().unwrap().parse().unwrap();
        let clip_y = attr(&format!(" {chunk}"), "y");
        let tile = img
            .split("data-tile=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        let z: u32 = tile.split('/').next().unwrap().parse().unwrap();
        let [mx, my] = SfProjection::Mercator.project([13.40, 52.52]);
        let (tx, ty) = tile_xy(z, mx, my);
        if tile != format!("{z}/{}/{}", tx.floor(), ty.floor()) {
            continue;
        }
        found = true;
        let (x, y) = (clip_x + attr(img, "x"), clip_y + attr(img, "y"));
        let (w, h) = (attr(img, "width"), attr(img, "height"));
        // Berlin sits at its fractional position inside its tile.
        let ex = x + tx.fract() * w;
        let ey = y + ty.fract() * h;
        assert!(
            (ex - cx).abs() < 1.0 && (ey - cy).abs() < 1.0,
            "({ex},{ey}) vs ({cx},{cy})"
        );
        // The bytes travel as a data URI.
        assert!(img.contains("href=\"data:image/png;base64,"));
    }
    assert!(found, "Berlin's tile drawn");
}

#[test]
fn missing_tiles_are_skipped() {
    let svg = map(|_, _, _| None)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(!svg.contains("<image"));
    assert!(svg.contains("<circle"));
}

#[cfg(any(feature = "plotters", feature = "canvas"))]
fn red_tile() -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(256, 256, image::Rgba([220, 30, 30, 255]));
    let mut out = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut out),
        image::ImageOutputFormat::Png,
    )
    .unwrap();
    out
}

#[cfg(feature = "plotters")]
#[test]
fn plotters_png_shows_the_tiles() {
    let tile = red_tile();
    let png = map(move |_, _, _| Some(tile.clone()))
        .render_png_with_size(400, 300)
        .unwrap();
    let img = image::load_from_memory(&png).unwrap().to_rgb8();
    let p = img.get_pixel(200, 150).0;
    assert!(p[0] > 200 && p[1] < 60, "panel centre is tile red: {p:?}");
}

#[cfg(feature = "canvas")]
#[test]
fn canvas_shows_the_tiles() {
    let tile = red_tile();
    let (w, _, rgba) = map(move |_, _, _| Some(tile.clone()))
        .render_rgba_with_size(400, 300)
        .unwrap();
    let i = ((150 * w + 200) * 4) as usize;
    assert!(rgba[i] > 200 && rgba[i + 1] < 60, "{:?}", &rgba[i..i + 4]);
}
