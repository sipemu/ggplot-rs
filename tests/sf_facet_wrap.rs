//! #293: small-multiple maps — `facet_wrap` under `coord_sf`: panels shaped
//! like the data, a per-panel clip and `<g>`, viewport attributes, the
//! attribution drawn once under the grid, legends without counts, and a
//! north arrow.
#![cfg(feature = "sf")]

use ggplot_rs::data::Value;
use ggplot_rs::geom::annotation_scale::{GeomAttribution, GeomNorthArrow, MapCorner};
use ggplot_rs::prelude::*;
use ggplot_rs::scale::binned::ScaleFillBinned;
use ggplot_rs::theme::LegendPosition;

/// `n` facets, each a copy of the same 4 points spanning a 200 × 100 data
/// window (aspect 0.5).
fn data(n: usize) -> Vec<(String, Vec<Value>)> {
    let pts = [(0.0, 0.0), (200.0, 0.0), (0.0, 100.0), (200.0, 100.0)];
    let (mut x, mut y, mut f, mut v) = (vec![], vec![], vec![], vec![]);
    for k in 0..n {
        for (i, (px, py)) in pts.iter().enumerate() {
            x.push(Value::Float(*px));
            y.push(Value::Float(*py));
            f.push(Value::Str(format!("w{k}")));
            v.push(Value::Float((i * 30 + k) as f64));
        }
    }
    vec![
        ("x".into(), x),
        ("y".into(), y),
        ("f".into(), f),
        ("v".into(), v),
    ]
}

fn map(n: usize, ncol: Option<usize>) -> GGPlot {
    GGPlot::new(data(n))
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_sf_with(ggplot_rs::coord::sf::CoordSf::new().expand(false))
        .theme_void()
        .legend_position(LegendPosition::None)
        .facet_wrap("f", ncol)
}

/// The root `data-panels` JSON, parsed into `(i, label, x, y, w, h)`.
fn panels(svg: &str) -> Vec<(usize, String, f64, f64, f64, f64)> {
    let start = svg.find("data-panels=\"").expect("data-panels") + "data-panels=\"".len();
    let raw = &svg[start..start + svg[start..].find('"').unwrap()];
    let json = raw.replace("&quot;", "\"");
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["i"].as_u64().unwrap() as usize,
                p["label"].as_str().unwrap().to_string(),
                p["x"].as_f64().unwrap(),
                p["y"].as_f64().unwrap(),
                p["w"].as_f64().unwrap(),
                p["h"].as_f64().unwrap(),
            )
        })
        .collect()
}

fn attr<'a>(svg: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let start = svg.find(&key)? + key.len();
    Some(&svg[start..start + svg[start..].find('"')?])
}

#[test]
fn panels_take_the_data_aspect_and_the_grid_is_centred() {
    let svg = map(4, None).render_svg_native_with_size(1600, 400).unwrap();
    let ps = panels(&svg);
    assert_eq!(ps.len(), 4);
    // one row of four (the largest panels for aspect 0.5 in 1600 × 400)
    let ys: Vec<f64> = ps.iter().map(|p| p.3).collect();
    assert!(ys.iter().all(|y| (y - ys[0]).abs() < 0.01), "{ys:?}");
    for p in &ps {
        assert!((p.5 / p.4 - 0.5).abs() < 0.01, "aspect {p:?}");
        assert!((p.4 - ps[0].4).abs() < 0.01 && (p.5 - ps[0].5).abs() < 0.01);
    }
    // centred horizontally inside data-plot
    let plot: Vec<f64> = attr(&svg, "data-plot")
        .unwrap()
        .split(' ')
        .map(|s| s.parse().unwrap())
        .collect();
    let left = ps[0].2 - plot[0];
    let right = plot[0] + plot[2] - (ps[3].2 + ps[3].4);
    assert!((left - right).abs() < 1.0, "left {left} right {right}");
    // labels in facet order
    let labels: Vec<&str> = ps.iter().map(|p| p.1.as_str()).collect();
    assert_eq!(labels, ["w0", "w1", "w2", "w3"]);
}

#[test]
fn a_square_area_gets_two_by_two_and_ncol_is_respected() {
    let sq = GGPlot::new(data(4))
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_sf_with(ggplot_rs::coord::sf::CoordSf::new().expand(false))
        .theme_void()
        .facet_wrap("f", None);
    // aspect 0.5 in a 600 × 600 area: 2 columns beat 1 (taller) and 4 (narrower)
    let ps = panels(&sq.render_svg_native_with_size(600, 600).unwrap());
    let rows: std::collections::BTreeSet<i64> = ps.iter().map(|p| p.3.round() as i64).collect();
    assert_eq!(rows.len(), 2, "{ps:?}");
    // ncol = 1: four rows
    let ps = panels(
        &map(4, Some(1))
            .render_svg_native_with_size(600, 900)
            .unwrap(),
    );
    let rows: std::collections::BTreeSet<i64> = ps.iter().map(|p| p.3.round() as i64).collect();
    assert_eq!(rows.len(), 4);
}

#[test]
fn each_panel_is_a_clipped_group_with_its_own_clip_id() {
    let a = map(3, None).render_svg_native_with_size(900, 300).unwrap();
    let b = map(3, None).render_svg_native_with_size(900, 300).unwrap();
    for i in 0..3 {
        assert!(a.contains(&format!("<g data-facet-panel=\"{i}\" clip-path=\"url(#")));
    }
    assert_eq!(a.matches("<clipPath id=").count(), 3);
    // every point mark sits inside a panel group
    let first_g = a.find("<g data-facet-panel=").unwrap();
    let first_mark = a.find("data-geom=\"point\"").unwrap();
    assert!(first_mark > first_g);
    // clip ids differ between two renders (safe to inline both in one page)
    let ida = a
        .split("<clipPath id=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string();
    let idb = b
        .split("<clipPath id=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string();
    assert_ne!(ida, idb);
}

#[test]
fn viewport_is_on_single_coord_sf_plots_only() {
    let single = GGPlot::new(data(1))
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_sf()
        .render_svg_native_with_size(400, 400)
        .unwrap();
    let vp: Vec<f64> = attr(&single, "data-viewport")
        .expect("data-viewport")
        .split(' ')
        .map(|s| s.parse().unwrap())
        .collect();
    assert_eq!(vp.len(), 4);
    assert!(!single.contains("data-panels"));
    let cart = GGPlot::new(data(1))
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg_native_with_size(400, 400)
        .unwrap();
    assert!(!cart.contains("data-viewport"));
    // a cartesian facet_wrap is unchanged: no panel groups, no data-panels
    let cart_facets = GGPlot::new(data(3))
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .facet_wrap("f", None)
        .render_svg_native_with_size(600, 400)
        .unwrap();
    assert!(!cart_facets.contains("data-facet-panel"));
    assert!(!cart_facets.contains("data-panels"));
    // svg_root_data(false) drops it along with data-plot
    let off = GGPlot::new(data(1))
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_sf()
        .svg_root_data(false)
        .render_svg_native_with_size(400, 400)
        .unwrap();
    assert!(!off.contains("data-viewport"));
}

#[test]
fn a_one_row_layer_carrying_a_facet_value_draws_in_that_panel_only() {
    let one = vec![
        ("x".into(), vec![Value::Float(100.0)]),
        ("y".into(), vec![Value::Float(50.0)]),
        ("f".into(), vec![Value::Str("w2".into())]),
    ];
    let svg = map(3, None)
        .geom_text_with(GeomText::default())
        .layer_data(one)
        .layer_aes(Aes::new().x("x").y("y").label("f"))
        .render_svg_native_with_size(900, 300)
        .unwrap();
    // the text mark appears once, inside panel 2's group
    let mark = "data-geom=\"text\">w2</text>";
    assert_eq!(svg.matches(mark).count(), 1, "{svg}");
    let g2 = svg.find("data-facet-panel=\"2\"").unwrap();
    assert!(svg.find(mark).unwrap() > g2);
}

#[test]
fn a_figure_attribution_draws_once_under_the_grid() {
    let base = map(3, None).render_svg_native_with_size(900, 400).unwrap();
    let svg = map(3, None)
        .annotation_attribution_with(GeomAttribution::new("© Test").figure(true))
        .render_svg_native_with_size(900, 400)
        .unwrap();
    assert_eq!(svg.matches("© Test").count(), 1);
    // drawn after every panel group
    assert!(svg.find("© Test").unwrap() > svg.rfind("data-facet-panel=").unwrap());
    // the grid shrank: panels are no taller than without the band, and
    // the text sits below the lowest panel
    let (pb, ps) = (panels(&base), panels(&svg));
    assert!(ps[0].5 <= pb[0].5 + 0.01);
    let bottom = ps.iter().map(|p| p.3 + p.5).fold(0.0, f64::max);
    let ty: f64 = svg[..svg.find("© Test").unwrap()]
        .rsplit(" y=\"")
        .next()
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(ty > bottom, "text y {ty} vs grid bottom {bottom}");
    // per-panel without .figure(true)
    let per = map(3, None)
        .annotation_attribution("© Test")
        .render_svg_native_with_size(900, 400)
        .unwrap();
    assert_eq!(per.matches("© Test").count(), 3);
    // a single-panel plot ignores the flag
    let single = |fig: bool| {
        GGPlot::new(data(1))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .coord_sf()
            .annotation_attribution_with(GeomAttribution::new("© Test").figure(fig))
            .render_svg_native_with_size(400, 300)
            .unwrap()
    };
    let text_tag = |svg: &str| {
        let i = svg.find("© Test").unwrap();
        svg[svg[..i].rfind("<text").unwrap()..i].to_string()
    };
    assert_eq!(text_tag(&single(true)), text_tag(&single(false)));
}

#[test]
fn show_counts_false_drops_the_counts() {
    let scale = |counts: bool| {
        ScaleFillBinned::new(
            Aesthetic::Fill,
            vec![0.0, 50.0, 100.0],
            vec![(240, 240, 200), (30, 90, 30)],
        )
        .show_counts(counts)
    };
    let render = |counts: bool| {
        GGPlot::new(data(1))
            .aes(Aes::new().x("x").y("y").fill("v"))
            .geom_tile()
            .scale_fill(scale(counts))
            .render_svg_native_with_size(500, 300)
            .unwrap()
    };
    let with = render(true);
    let without = render(false);
    assert!(with.contains("0–50 ("), "{with}");
    assert!(without.contains(">0–50<"), "{without}");
    assert!(!without.contains("0–50 ("));
}

#[test]
fn north_arrow_rotates_and_stacks_clear_of_a_scale_bar() {
    let arrow = |deg: f64| {
        GGPlot::new(data(1))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .coord_sf()
            .annotation_north_arrow_with(GeomNorthArrow::default().rotation_deg(deg))
            .render_svg_native_with_size(400, 300)
            .unwrap()
    };
    let up = arrow(0.0);
    let right = arrow(90.0);
    assert!(up.contains(">N</text>"));
    // the triangle's first vertex (the tip) is above the centre at 0°, to
    // the right at 90°
    let tip = |svg: &str| -> (f64, f64, f64, f64) {
        let pts = svg
            .split("<polygon points=\"")
            .last()
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .to_string();
        let v: Vec<(f64, f64)> = pts
            .split(' ')
            .map(|p| {
                let mut it = p.split(',').map(|s| s.parse::<f64>().unwrap());
                (it.next().unwrap(), it.next().unwrap())
            })
            .collect();
        let cx = v.iter().map(|p| p.0).sum::<f64>() / v.len() as f64;
        let cy = v.iter().map(|p| p.1).sum::<f64>() / v.len() as f64;
        (v[0].0, v[0].1, cx, cy)
    };
    let (tx, ty, cx, cy) = tip(&up);
    assert!(ty < cy && (tx - cx).abs() < 0.5);
    let (tx, ty, cx, cy) = tip(&right);
    assert!(tx > cx && (ty - cy).abs() < 0.5);
    // with a scale bar in the same corner the two don't overlap: the bar
    // is pushed down below the arrow
    let both = GGPlot::new(vec![
        ("x".into(), vec![Value::Float(0.0), Value::Float(500_000.0)]),
        ("y".into(), vec![Value::Float(0.0), Value::Float(400_000.0)]),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .coord_sf()
    .annotation_north_arrow_with(GeomNorthArrow::default().corner(MapCorner::TopRight))
    .annotation_scale_with(
        ggplot_rs::geom::annotation_scale::GeomAnnotationScale::default()
            .corner(MapCorner::TopRight),
    )
    .render_svg_native_with_size(400, 300)
    .unwrap();
    let n_y: f64 = both[..both.find(">N</text>").unwrap()]
        .rsplit(" y=\"")
        .next()
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let bar = both.split("km").next().unwrap();
    let bar_y: f64 = bar
        .rsplit(" y=\"")
        .next()
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(bar_y > n_y + 18.0, "bar label at {bar_y}, N at {n_y}");
}
