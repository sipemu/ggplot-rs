//! #221: allow-listed per-mark/per-layer SVG attributes for interactive
//! diagrams (DataZooDE/anofox-visualization#421) — `Aes::class` (per-row
//! CSS class tokens), `GGPlot::layer_svg_attr` (`class`, `aria-hidden`,
//! `pointer-events`, `data-*` on every mark of a layer) and
//! `Aes::group_wrap` (one `<g data-<name>="<value>">` per key, across
//! layers). No inline `style`, ever.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::theme::presets::theme_void;
use ggplot_rs::GGPlot;

fn fcol(name: &str, v: &[f64]) -> (String, Vec<Value>) {
    (name.into(), v.iter().map(|&x| Value::Float(x)).collect())
}

fn scol(name: &str, v: &[&str]) -> (String, Vec<Value>) {
    (
        name.into(),
        v.iter().map(|&s| Value::Str(s.into())).collect(),
    )
}

/// Two diagram "nodes" `a`/`b`: a box (xmin..xmax, ymin..ymax) plus a
/// label at its centre.
fn nodes() -> Vec<(String, Vec<Value>)> {
    vec![
        scol("id", &["a", "b"]),
        fcol("x0", &[0.0, 4.0]),
        fcol("x1", &[2.0, 6.0]),
        fcol("y0", &[0.0, 0.0]),
        fcol("y1", &[1.0, 1.0]),
        fcol("cx", &[1.0, 5.0]),
        fcol("cy", &[0.5, 0.5]),
        scol("cls", &["speed-1 hot", "speed-3"]),
    ]
}

fn render(p: GGPlot) -> (String, Vec<String>) {
    p.theme(theme_void())
        .render_svg_native_with_warnings(300, 200)
        .unwrap()
}

fn boxes() -> GGPlot {
    GGPlot::new(nodes())
        .geom_rect()
        .layer_aes(Aes::new().xmin("x0").xmax("x1").ymin("y0").ymax("y1"))
}

/// All elements (start tags) carrying `needle`.
fn tags_with<'a>(svg: &'a str, needle: &str) -> Vec<&'a str> {
    svg.split('<').filter(|t| t.contains(needle)).collect()
}

#[test]
fn mapped_class_multi_token_round_trips_into_the_one_class_attribute() {
    let (svg, warnings) = render(
        boxes().layer_aes(
            Aes::new()
                .xmin("x0")
                .xmax("x1")
                .ymin("y0")
                .ymax("y1")
                .class("cls"),
        ),
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(
        svg.contains(r#"class="gg-mark gg-bar speed-1 hot""#)
            || svg.contains(r#"class="gg-mark gg-band speed-1 hot""#),
        "{svg}"
    );
    assert!(svg.contains(" speed-3\""), "{svg}");
    for t in tags_with(&svg, "speed-") {
        assert_eq!(t.matches("class=").count(), 1, "one class attr: {t}");
    }
    assert!(!svg.contains("style="), "never an inline style: {svg}");
}

#[test]
fn off_allow_list_class_tokens_are_dropped_with_a_warning() {
    let data = vec![
        fcol("x", &[1.0, 2.0]),
        fcol("y", &[1.0, 2.0]),
        scol(
            "cls",
            &["ok-1 \"><script>x</script>", "1bad also_ok style=color:red"],
        ),
    ];
    let (svg, warnings) = render(
        GGPlot::new(data)
            .aes(Aes::new().x("x").y("y").class("cls"))
            .geom_point(),
    );
    assert!(svg.contains(r#"class="gg-mark gg-point ok-1""#), "{svg}");
    assert!(svg.contains(r#"class="gg-mark gg-point also_ok""#), "{svg}");
    assert!(!svg.contains("script") && !svg.contains("1bad"), "{svg}");
    assert!(!svg.contains("style="), "{svg}");
    assert!(
        warnings.iter().any(|w| w.contains("invalid class token")),
        "{warnings:?}"
    );
}

#[test]
fn layer_flags_appear_only_on_the_flagged_layer() {
    let (svg, warnings) = render(
        boxes()
            .geom_text()
            .layer_aes(Aes::new().x("cx").y("cy").label("id"))
            .layer_svg_attr("aria-hidden", "true")
            .layer_svg_attr("pointer-events", "none")
            .layer_svg_attr("class", "node-label"),
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    let texts = tags_with(&svg, "gg-text");
    assert_eq!(texts.len(), 2, "{svg}");
    for t in &texts {
        assert!(
            t.contains(
                r#"aria-hidden="true" pointer-events="none" class="gg-mark gg-text node-label""#
            ),
            "{t}"
        );
    }
    // The box layer underneath is untouched.
    for t in tags_with(&svg, "data-geom=\"rect\"") {
        assert!(
            !t.contains("aria-hidden") && !t.contains("pointer-events"),
            "{t}"
        );
        assert!(!t.contains("node-label"), "{t}");
    }

    // Absent when never set.
    let (plain, _) = render(
        boxes()
            .geom_text()
            .layer_aes(Aes::new().x("cx").y("cy").label("id")),
    );
    assert!(
        !plain.contains("aria-hidden") && !plain.contains("pointer-events"),
        "{plain}"
    );
}

#[test]
fn layer_svg_attr_rejects_anything_off_the_allow_list() {
    let (svg, warnings) = render(
        boxes()
            .layer_svg_attr("style", "fill:red")
            .layer_svg_attr("onclick", "alert(1)")
            .layer_svg_attr("pointer-events", "none\" onload=\"x")
            .layer_svg_attr("aria-hidden", "yes")
            .layer_svg_attr("class", "ok \"bad")
            .layer_svg_attr("data-node", "n\"1"),
    );
    assert!(!svg.contains("style=") && !svg.contains("onclick") && !svg.contains("onload"));
    assert!(
        !svg.contains("aria-hidden") && !svg.contains("pointer-events"),
        "{svg}"
    );
    assert!(svg.contains(" ok\""), "valid tokens kept: {svg}");
    assert!(
        svg.contains(r#"data-node="n&quot;1""#),
        "data-* escaped: {svg}"
    );
    for needle in [
        "\"style\"",
        "\"onclick\"",
        "\"pointer-events\"",
        "\"aria-hidden\"",
        "invalid class token",
    ] {
        assert!(
            warnings.iter().any(|w| w.contains(needle)),
            "missing warning for {needle}: {warnings:?}"
        );
    }
    let (_, warnings) = render(GGPlot::new(nodes()).layer_svg_attr("class", "x"));
    assert!(
        warnings.iter().any(|w| w.contains("before any layer")),
        "{warnings:?}"
    );
}

#[test]
fn group_wrap_holds_exactly_each_keys_marks_from_the_grouped_layers() {
    let (svg, warnings) = render(
        GGPlot::new(nodes())
            // An ungrouped layer first: its marks must stay outside.
            .geom_point()
            .layer_aes(Aes::new().x("cx").y("cy"))
            .geom_rect()
            .layer_aes(
                Aes::new()
                    .xmin("x0")
                    .xmax("x1")
                    .ymin("y0")
                    .ymax("y1")
                    .group_wrap("of", "id"),
            )
            .geom_text()
            .layer_aes(
                Aes::new()
                    .x("cx")
                    .y("cy")
                    .label("id")
                    .group_wrap("data-of", "id"),
            )
            .layer_svg_attr("aria-hidden", "true")
            .layer_svg_attr("pointer-events", "none"),
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    for (key, other) in [("a", "b"), ("b", "a")] {
        let open = format!(r#"<g data-of="{key}">"#);
        assert_eq!(svg.matches(&open).count(), 1, "{svg}");
        let start = svg.find(&open).unwrap() + open.len();
        let inner = &svg[start..start + svg[start..].find("</g>").unwrap()];
        assert_eq!(inner.matches("data-geom=\"rect\"").count(), 1, "{inner}");
        assert_eq!(inner.matches("data-geom=\"text\"").count(), 1, "{inner}");
        assert!(inner.contains(&format!(">{key}</text>")), "{inner}");
        assert!(!inner.contains(&format!(">{other}</text>")), "{inner}");
        assert!(
            !inner.contains("data-geom=\"point\""),
            "ungrouped layer leaked: {inner}"
        );
        assert!(!inner.contains("<g"), "no nesting: {inner}");
    }
    // The ungrouped points are before the first group, outside any group.
    let first_group = svg.find("<g data-of=").unwrap();
    assert_eq!(svg[..first_group].matches("data-geom=\"point\"").count(), 2);
    assert_eq!(svg.matches("<g data-of=").count(), 2);
}

#[test]
fn group_wrap_value_is_escaped_na_is_ungrouped_and_bad_names_warn() {
    let data = vec![
        fcol("x", &[1.0, 2.0]),
        fcol("y", &[1.0, 2.0]),
        ("k".to_string(), vec![Value::Str("a\"<b".into()), Value::Na]),
    ];
    let (svg, _) = render(
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y").group_wrap("of", "k"))
            .geom_point(),
    );
    assert_eq!(
        svg.matches("<g data-of=\"a&quot;&lt;b\">").count(),
        1,
        "{svg}"
    );
    assert_eq!(
        svg.matches("<g data-of").count(),
        1,
        "NA row ungrouped: {svg}"
    );

    let (svg, warnings) = render(
        GGPlot::new(data)
            .aes(Aes::new().x("x").y("y").group_wrap("Bad Name", "k"))
            .geom_point(),
    );
    assert!(!svg.contains("<g data-"), "{svg}");
    assert!(
        warnings.iter().any(|w| w.contains("group_wrap")),
        "{warnings:?}"
    );
}

#[test]
fn layer_svg_attrs_plural_and_later_call_replaces_earlier() {
    let (svg, warnings) = render(boxes().layer_svg_attrs([
        ("class", "first"),
        ("class", "second"),
        ("data-kind", "box"),
    ]));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains(" second\"") && !svg.contains("first"), "{svg}");
    assert_eq!(svg.matches(r#"data-kind="box""#).count(), 2, "{svg}");
}

#[test]
fn geom_label_box_and_text_share_the_group_and_flags() {
    let (svg, _) = render(
        GGPlot::new(nodes())
            .geom_label()
            .layer_aes(
                Aes::new()
                    .x("cx")
                    .y("cy")
                    .label("id")
                    .class("cls")
                    .group_wrap("of", "id")
                    .data_attr("node", "id"),
            )
            .layer_svg_attr("pointer-events", "none"),
    );
    let open = r#"<g data-of="a">"#;
    let start = svg.find(open).unwrap() + open.len();
    let inner = &svg[start..start + svg[start..].find("</g>").unwrap()];
    // The label's background rect and its text, both classed and inert.
    assert_eq!(inner.matches("speed-1 hot").count(), 2, "{inner}");
    assert_eq!(
        inner.matches(r#"pointer-events="none""#).count(),
        2,
        "{inner}"
    );
    assert_eq!(inner.matches(r#"data-node="a""#).count(), 2, "{inner}");
    assert!(!inner.contains("<title>"), "{inner}");
}

#[test]
fn decorative_layer_keeps_class_group_and_layer_attrs() {
    let (svg, _) = render(
        GGPlot::new(nodes())
            .geom_point()
            .layer_aes(
                Aes::new()
                    .x("cx")
                    .y("cy")
                    .class("cls")
                    .group_wrap("of", "id"),
            )
            .decorative()
            .layer_svg_attr("pointer-events", "none"),
    );
    assert_eq!(svg.matches("<g data-of=").count(), 2, "{svg}");
    assert!(svg.contains("speed-3"), "{svg}");
    // A decorative mark already writes aria-hidden="true" — never twice.
    for t in tags_with(&svg, "gg-decorative") {
        assert_eq!(t.matches("aria-hidden").count(), 1, "{t}");
        assert!(t.contains(r#"pointer-events="none""#), "{t}");
    }
}

#[test]
fn faceted_and_grid_composed_plots_group_too() {
    let mut data = nodes();
    data.push(scol("f", &["p", "q"]));
    let faceted = GGPlot::new(data)
        .geom_point()
        .layer_aes(
            Aes::new()
                .x("cx")
                .y("cy")
                .group_wrap("of", "id")
                .class("cls"),
        )
        .layer_svg_attr("aria-hidden", "true")
        .facet_wrap("f", None);
    let (svg, _) = render(faceted);
    assert_eq!(svg.matches("<g data-of=").count(), 2, "{svg}");
    assert!(
        svg.contains(r#"aria-hidden="true" class="gg-mark gg-point speed-3""#),
        "{svg}"
    );

    let cell = GGPlot::new(nodes())
        .geom_point()
        .layer_aes(
            Aes::new()
                .x("cx")
                .y("cy")
                .color("id")
                .group_wrap("of", "id"),
        )
        .svg_attr("data-cell", "one");
    let grid = ggplot_rs::compose::PlotGrid::new()
        .add(cell)
        .add(boxes())
        .render_svg_native_with_size(400, 200)
        .unwrap();
    assert_eq!(grid.matches("<g data-of=").count(), 2, "{grid}");
    // The cell's own svg_attr group wraps its whole content, groups inside.
    let cell_g = grid.find(r#"<g data-cell="one">"#).unwrap();
    assert!(grid[cell_g..].find("<g data-of=").is_some(), "{grid}");
}

#[cfg(feature = "canvas")]
#[test]
fn raster_backend_ignores_the_new_attributes() {
    let p = boxes()
        .layer_aes(
            Aes::new()
                .xmin("x0")
                .xmax("x1")
                .ymin("y0")
                .ymax("y1")
                .class("cls")
                .group_wrap("of", "id"),
        )
        .layer_svg_attr("pointer-events", "none");
    let (w, h, px) = p.render_rgba_with_size(100, 100).unwrap();
    assert_eq!(px.len(), (w * h * 4) as usize);
}
