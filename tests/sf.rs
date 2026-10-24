//! geom_sf (feature `sf`): simple-features rendering + scale training.
#![cfg(feature = "sf")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::geom::sf::{GeomSf, StatSf};
use ggplot_rs::prelude::*;
use ggplot_rs::scale::scale_set::ScaleSet;
use ggplot_rs::spatial::SfProjection;
use ggplot_rs::stat::Stat;

fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

#[test]
fn stat_sf_adds_bounds_from_geometry() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 4 0, 4 3, 0 3, 0 0))", "POINT (10 8)"]),
    );
    let out = StatSf::default().compute_group(&df, &ScaleSet::new());
    // Original column preserved + extent columns added.
    assert!(out.has_column("geometry"));
    for c in ["xmin", "xmax", "ymin", "ymax"] {
        assert!(out.has_column(c), "missing {c}");
    }
    assert_eq!(out.column("xmin").unwrap()[0].as_f64(), Some(0.0));
    assert_eq!(out.column("xmax").unwrap()[0].as_f64(), Some(4.0));
    assert_eq!(out.column("xmax").unwrap()[1].as_f64(), Some(10.0));
    assert_eq!(out.column("ymax").unwrap()[1].as_f64(), Some(8.0));
}

#[test]
fn polygon_choropleth_renders_with_fill() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "POLYGON ((0 0, 3 0, 3 3, 0 3, 0 0))",
            "POLYGON ((3 0, 6 0, 6 3, 3 3, 3 0))",
        ]),
    );
    df.add_column("pop".into(), vec![Value::Float(2.0), Value::Float(9.0)]);

    let svg = GGPlot::new(df)
        .aes(Aes::new().fill("pop"))
        .geom_sf()
        .scale_fill_viridis_c()
        .render_svg_native()
        .expect("sf render");
    assert!(
        svg.contains("<polygon") || svg.contains("<path"),
        "polygons drawn"
    );
}

#[test]
fn mixed_geometry_types_render() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "POLYGON ((0 0, 5 0, 5 5, 0 5, 0 0))",
            "LINESTRING (0 0, 5 5, 2 5)",
            "MULTIPOINT ((1 1), (4 2), (2 4))",
        ]),
    );
    assert!(GGPlot::new(df)
        .aes(Aes::new())
        .geom_sf()
        .render_svg_native()
        .is_ok());
}

#[test]
fn scales_train_over_geometry_extent() {
    // A single polygon spanning x in [10, 20] should give an x-axis that covers
    // it, i.e. the built layer exposes xmin/xmax matching the geometry.
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((10 5, 20 5, 20 12, 10 12, 10 5))"]),
    );
    let built = GGPlot::new(df).aes(Aes::new()).geom_sf().build();
    let d = &built.layers[0].data;
    assert_eq!(d.column("xmin").unwrap()[0].as_f64(), Some(10.0));
    assert_eq!(d.column("ymax").unwrap()[0].as_f64(), Some(12.0));
}

#[test]
fn mercator_projection_reshapes_extent() {
    // A polygon at latitude [60, 80]: under Mercator the y extent is the
    // projected value, not the raw degrees, and x is longitude in radians.
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 60, 20 60, 20 80, 0 80, 0 60))"]),
    );
    let out = StatSf {
        projection: SfProjection::Mercator,
    }
    .compute_group(&df, &ScaleSet::new());
    let ymax = out.column("ymax").unwrap()[0].as_f64().unwrap();
    let xmax = out.column("xmax").unwrap()[0].as_f64().unwrap();
    // 80° -> ln(tan(pi/4 + 40°)) ≈ 2.4362; 20° lon -> 0.349 rad.
    assert!((ymax - 2.4362).abs() < 1e-3, "mercator ymax = {ymax}");
    assert!((xmax - 20f64.to_radians()).abs() < 1e-9, "lon in radians");
}

#[test]
fn coord_sf_renders_projected_map() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "POLYGON ((0 0, 30 0, 30 40, 0 40, 0 0))",
            "POLYGON ((30 0, 60 0, 60 50, 30 40, 30 0))",
        ]),
    );
    df.add_column("v".into(), vec![Value::Float(1.0), Value::Float(2.0)]);
    let svg = GGPlot::new(df)
        .aes(Aes::new().fill("v"))
        .geom_sf_with(GeomSf::default().project(SfProjection::Mercator))
        .coord_sf()
        .scale_fill_viridis_c()
        .render_svg_native()
        .expect("mercator + coord_sf render");
    assert!(svg.contains("<polygon") || svg.contains("<path"));
}

#[test]
fn hover_tooltip_emitted_as_title() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 2 0, 2 2, 0 2, 0 0))"]),
    );
    df.add_column("name".into(), strs(&["Region A"]));
    df.add_column("pop".into(), vec![Value::Float(12.0)]);
    let svg = GGPlot::new(df)
        .aes(Aes::new().fill("pop").label("name"))
        .geom_sf()
        .scale_fill_viridis_c()
        .render_svg_native()
        .unwrap();
    assert!(
        svg.contains("<title>Region A: 12</title>"),
        "tooltip in: {svg}"
    );
}

/// #178: a polygon with a hole draws one `<path>` whose `d` has two
/// subpaths (exterior + hole) under `fill-rule="evenodd"`, so the hole is
/// cut out of the fill rather than drawn as a separately-stroked ring.
#[test]
fn polygon_with_a_hole_renders_as_one_evenodd_path() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0), (3 3, 7 3, 7 7, 3 7, 3 3))"]),
    );
    let svg = GGPlot::new(df)
        .aes(Aes::new())
        .geom_sf()
        .render_svg_native()
        .expect("sf render");
    assert!(svg.contains("fill-rule=\"evenodd\""), "{svg}");
    let path_d = svg
        .split("d=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("a path d attribute");
    assert_eq!(path_d.matches('M').count(), 2, "exterior + hole: {path_d}");
    // #214: geom_sf's default encoding closes each subpath with lowercase
    // `z` (relative-command convention) now, not `Z` — functionally
    // identical, SVG's closepath takes no parameters.
    assert_eq!(path_d.matches('z').count(), 2, "{path_d}");
}

/// A multipolygon draws each part as its own evenodd path (so a part with
/// its own hole still cuts it out), independently of its sibling parts.
#[test]
fn multipolygon_parts_render_as_evenodd_subpaths_of_one_path() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "MULTIPOLYGON (((0 0, 4 0, 4 4, 0 4, 0 0)), ((10 0, 14 0, 14 4, 10 4, 10 0), (11 1, 13 1, 13 3, 11 3, 11 1)))",
        ]),
    );
    let svg = GGPlot::new(df)
        .aes(Aes::new())
        .geom_sf()
        .render_svg_native()
        .expect("sf render");
    // #213: one mark (one evenodd `<path>`) for the whole MultiPolygon
    // row, not one per part — 3 subpaths total (the first part's
    // exterior, the second part's exterior, the second part's hole).
    assert_eq!(svg.matches("<path ").count(), 1, "{svg}");
    let subpath_counts: Vec<usize> = svg
        .split("d=\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap_or("").matches('M').count())
        .collect();
    assert_eq!(subpath_counts, vec![3], "{svg}");
}

/// #178: render-resolution simplification drops near-collinear vertices by
/// default (tolerance 0.5 px), and `.simplify(0.0)` keeps every one.
#[test]
fn render_resolution_simplification_reduces_near_collinear_points() {
    // A densely-sampled square ring (100 points per side, each side's points
    // nearly collinear — a 0.05-unit wiggle, far under half a screen pixel
    // once the panel is rendered at 400x400px over a 100x100 data extent).
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for i in 0..100 {
        let t = i as f64 / 100.0;
        pts.push((t * 100.0, if i % 2 == 0 { 0.0 } else { 0.05 }));
    }
    for i in 0..100 {
        let t = i as f64 / 100.0;
        pts.push((100.0, t * 100.0));
    }
    for i in 0..100 {
        let t = i as f64 / 100.0;
        pts.push((100.0 - t * 100.0, 100.0));
    }
    for i in 0..100 {
        let t = i as f64 / 100.0;
        pts.push((0.0, 100.0 - t * 100.0));
    }
    let wkt = {
        let coords: Vec<String> = pts
            .iter()
            .chain(pts.first())
            .map(|(x, y)| format!("{x} {y}"))
            .collect();
        format!("POLYGON (({}))", coords.join(", "))
    };
    let count_ls = |simplify_px: f64| {
        let mut df = DataFrame::new();
        df.add_column("geometry".into(), strs(&[&wkt]));
        let svg = GGPlot::new(df)
            .aes(Aes::new())
            .geom_sf_with(GeomSf::default().simplify(simplify_px))
            .render_svg_native_with_size(400, 400)
            .expect("sf render");
        let path_d = svg
            .split("d=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .expect("a path d attribute")
            .to_string();
        path_d.matches('L').count()
    };
    let simplified = count_ls(0.5);
    let unsimplified = count_ls(0.0);
    assert_eq!(unsimplified, 400, "every vertex kept at tolerance 0");
    assert!(
        simplified < unsimplified / 4,
        "simplified {simplified} should be far fewer than unsimplified {unsimplified}"
    );
}

/// #178: a faceted plot draws each panel through `PanelBackendAdapter`
/// (`render::renderer`), not the top-level backend directly — its own
/// `draw_polygon_rings` forwarding needs its own coverage, separate from
/// every other (unfaceted) test in this file.
#[test]
fn a_hole_renders_correctly_inside_a_facet_panel() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0), (3 3, 7 3, 7 7, 3 7, 3 3))",
            "POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))",
        ]),
    );
    df.add_column("region".into(), strs(&["A", "B"]));
    let svg = GGPlot::new(df)
        .aes(Aes::new())
        .geom_sf()
        .facet_wrap("region", Some(2))
        .render_svg_native()
        .expect("faceted sf render");
    assert!(svg.contains("fill-rule=\"evenodd\""), "{svg}");
    let subpath_counts: Vec<usize> = svg
        .split("d=\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap_or("").matches('M').count())
        .collect();
    // Panel A's polygon has a hole (2 subpaths); panel B's doesn't (1).
    assert!(subpath_counts.contains(&2), "{svg}");
}

/// #213: a MultiPolygon row (3 disjoint parts, one of them with a hole)
/// draws as **one** mark — one `<path>` carrying one set of row
/// attributes and 4 subpaths (3 exterior rings + 1 hole), not 3 separate
/// `<path>` elements. A single-`Polygon` row's output is unchanged.
#[test]
fn a_multipolygon_row_draws_as_one_mark_not_one_per_part() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            // 3 parts; the middle one has a hole.
            "MULTIPOLYGON (\
                ((0 0, 2 0, 2 2, 0 2, 0 0)), \
                ((10 0, 20 0, 20 10, 10 10, 10 0), (13 3, 17 3, 17 7, 13 7, 13 3)), \
                ((30 0, 32 0, 32 2, 30 2, 30 0)))",
            "POLYGON ((0 20, 10 20, 10 30, 0 30, 0 20))",
        ]),
    );
    df.add_column("rowid".into(), strs(&["multi", "single"]));

    let svg = GGPlot::new(df)
        .aes(Aes::new().data_attr("row", "rowid"))
        .geom_sf()
        .render_svg_native()
        .expect("sf render");

    // Exactly one mark (one `<path ... data-row="multi" ...>`) for the
    // 3-part row, not one per part.
    assert_eq!(
        svg.matches("data-row=\"multi\"").count(),
        1,
        "one mark for the whole MultiPolygon row: {svg}"
    );
    assert_eq!(
        svg.matches("data-row=\"single\"").count(),
        1,
        "single Polygon row unaffected: {svg}"
    );

    // Pull each `<path ...>` element out whole, so a `d=` attribute is
    // matched against its own element's `data-row`, not a neighbour's.
    fn path_for<'a>(svg: &'a str, needle: &str) -> &'a str {
        svg.split("<path ")
            .find(|el| el.contains(needle))
            .expect("mark present")
    }
    fn d_of(el: &str) -> &str {
        el.split("d=\"")
            .nth(1)
            .and_then(|after| after.split('"').next())
            .unwrap_or("")
    }

    // That one mark's path data has 4 subpaths (3 parts' exteriors + the
    // middle part's hole), drawn fill-rule evenodd.
    let multi_el = path_for(&svg, "data-row=\"multi\"");
    let multi_d = d_of(multi_el);
    assert_eq!(
        multi_d.matches('M').count(),
        4,
        "3 exterior rings + 1 hole in one path: {multi_d}"
    );
    assert!(multi_el.contains("fill-rule=\"evenodd\""), "{multi_el}");

    // Single Polygon's subpath count is still exactly 1 (unchanged).
    let single_d = d_of(path_for(&svg, "data-row=\"single\""));
    assert_eq!(single_d.matches('M').count(), 1, "{single_d}");
}
