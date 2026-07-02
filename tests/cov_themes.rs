//! Coverage-oriented tests that RENDER plots to exercise the theme, coord,
//! facet, and annotation code paths (which only run during an actual render).

use ggplot_rs::prelude::*;

/// Build a small column-oriented dataset with a discrete grouping column `g`,
/// a second discrete column `r`, and a third discrete column `c`.
fn sample_data() -> Vec<(String, Vec<Value>)> {
    let n = 12;
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 5) as f64 + 1.0)).collect();
    let g: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("grp{}", i % 3)))
        .collect();
    let r: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("row{}", i % 2)))
        .collect();
    let c: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("col{}", i % 2)))
        .collect();
    vec![
        ("x".to_string(), x),
        ("y".to_string(), y),
        ("g".to_string(), g),
        ("r".to_string(), r),
        ("c".to_string(), c),
    ]
}

/// A fully-labelled base plot with a mapped color aesthetic so a legend renders.
fn base_plot() -> GGPlot {
    GGPlot::new(sample_data())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .title("Title")
        .subtitle("Subtitle")
        .caption("Caption")
        .xlab("X axis")
        .ylab("Y axis")
}

fn assert_renders(plot: GGPlot) {
    let svg = plot.render_svg();
    assert!(svg.is_ok(), "render failed: {:?}", svg.err());
    assert!(!svg.unwrap().is_empty(), "rendered SVG was empty");
}

// ─── 1. Every theme preset literal (constructed AND drawn) ───────────

type PresetFn = fn() -> Theme;
type PresetBaseFn = fn(f64) -> Theme;

#[test]
fn theme_presets_render() {
    let presets: Vec<(&str, PresetFn)> = vec![
        ("gray", theme_gray),
        ("bw", theme_bw),
        ("classic", theme_classic),
        ("minimal", theme_minimal),
        ("dark", theme_dark),
        ("light", theme_light),
        ("linedraw", theme_linedraw),
        ("void", theme_void),
    ];
    for (name, f) in presets {
        let svg = base_plot().theme(f()).render_svg();
        assert!(svg.is_ok(), "preset {name} failed: {:?}", svg.err());
        assert!(!svg.unwrap().is_empty(), "preset {name} produced empty svg");
    }
}

#[test]
fn theme_preset_base_variants_render() {
    let presets: Vec<(&str, PresetBaseFn)> = vec![
        ("gray", theme_gray_base),
        ("bw", theme_bw_base),
        ("classic", theme_classic_base),
        ("minimal", theme_minimal_base),
        ("dark", theme_dark_base),
        ("light", theme_light_base),
        ("linedraw", theme_linedraw_base),
        ("void", theme_void_base),
    ];
    for (name, f) in presets {
        let svg = base_plot().theme(f(11.0)).render_svg();
        assert!(svg.is_ok(), "base preset {name} failed: {:?}", svg.err());
        assert!(
            !svg.unwrap().is_empty(),
            "base preset {name} produced empty svg"
        );
    }
}

#[test]
fn theme_convenience_methods_render() {
    assert_renders(base_plot().theme_gray());
    assert_renders(base_plot().theme_bw());
    assert_renders(base_plot().theme_classic());
    assert_renders(base_plot().theme_minimal());
    assert_renders(base_plot().theme_dark());
    assert_renders(base_plot().theme_light());
    assert_renders(base_plot().theme_linedraw());
    assert_renders(base_plot().theme_void());
}

// ─── 2. Theme accessors + runtime construction ──────────────────────

#[test]
fn theme_accessors_exercised() {
    let mut t = theme_gray();
    // Set per-axis overrides so the Option accessors return the override branch.
    t = t
        .set_axis_line_x(Some(ElementLine {
            color: (10, 20, 30),
            width: 0.7,
            visible: true,
        }))
        .set_axis_line_y(Some(ElementLine::blank()))
        .set_axis_ticks_x(Some(ElementLine::default()))
        .set_axis_ticks_y(Some(ElementLine::blank()))
        .set_panel_grid_major_x(Some(ElementLine::default()))
        .set_panel_grid_major_y(Some(ElementLine::blank()))
        .set_panel_grid_minor_x(Some(ElementLine::default()))
        .set_panel_grid_minor_y(Some(ElementLine::blank()))
        .set_panel_spacing_x(Some(6.0))
        .set_panel_spacing_y(Some(8.0));

    // Call every fallback accessor.
    let _ = t.get_axis_line_x();
    let _ = t.get_axis_line_y();
    let _ = t.get_axis_ticks_x();
    let _ = t.get_axis_ticks_y();
    let _ = t.get_panel_grid_major_x();
    let _ = t.get_panel_grid_major_y();
    let _ = t.get_panel_grid_minor_x();
    let _ = t.get_panel_grid_minor_y();
    assert_eq!(t.get_panel_spacing_x(), 6.0);
    assert_eq!(t.get_panel_spacing_y(), 8.0);
    let _ = t.primary_or((1, 2, 3));

    assert_renders(base_plot().theme(t));

    // And exercise the inherit (None) branch of every accessor via a fresh theme.
    let inherit = theme_bw();
    let _ = inherit.get_axis_line_x();
    let _ = inherit.get_axis_line_y();
    let _ = inherit.get_axis_ticks_x();
    let _ = inherit.get_axis_ticks_y();
    let _ = inherit.get_panel_grid_major_x();
    let _ = inherit.get_panel_grid_major_y();
    let _ = inherit.get_panel_grid_minor_x();
    let _ = inherit.get_panel_grid_minor_y();
    assert_eq!(inherit.get_panel_spacing_x(), inherit.panel_spacing);
    assert_renders(base_plot().theme(inherit));
}

#[test]
fn theme_setters_and_primary_render() {
    let t = theme_minimal()
        .set_text(ElementText::default())
        .set_title(ElementText::default())
        .set_subtitle(ElementText::default())
        .set_caption(ElementText::default())
        .set_axis_text_x(ElementText::default())
        .set_axis_text_y(ElementText::default())
        .set_axis_title_x(ElementText::default())
        .set_axis_title_y(ElementText::default())
        .set_axis_line(ElementLine::default())
        .set_axis_ticks(ElementLine::default())
        .set_panel_background(ElementRect::default())
        .set_panel_grid_major(ElementLine::default())
        .set_panel_grid_minor(ElementLine::default())
        .set_panel_border(ElementLine::default())
        .set_plot_background(ElementRect::default())
        .set_legend_title(ElementText::default())
        .set_legend_text(ElementText::default())
        .set_legend_background(ElementRect::default())
        .set_legend_key(ElementRect::default())
        .set_strip_text(ElementText::default())
        .set_strip_background(ElementRect::default())
        .set_axis_ticks_length(3.0)
        .set_legend_key_width(14.0)
        .set_legend_key_height(20.0)
        .set_legend_spacing(6.0)
        .set_legend_margin(Margin::default())
        .set_panel_spacing(5.0)
        .set_plot_margin(Margin::default())
        .with_primary((200, 100, 50));
    assert_renders(base_plot().theme(t));

    // primary_color builder on the plot.
    assert_renders(
        GGPlot::new(sample_data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .title("Primary")
            .primary_color((12, 34, 56)),
    );
}

#[test]
fn theme_update_render() {
    let update = ThemeUpdate {
        text: Some(ElementText::default()),
        title: Some(ElementText::default()),
        subtitle: Some(ElementText::default()),
        caption: Some(ElementText::default()),
        axis_text_x: Some(ElementText {
            angle: 45.0,
            ..Default::default()
        }),
        axis_text_y: Some(ElementText::default()),
        axis_title_x: Some(ElementText::default()),
        axis_title_y: Some(ElementText::default()),
        axis_line: Some(ElementLine::default()),
        axis_ticks: Some(ElementLine::default()),
        panel_background: Some(ElementRect::default()),
        panel_grid_major: Some(ElementLine::default()),
        panel_grid_minor: Some(ElementLine::default()),
        panel_border: Some(ElementLine::default()),
        plot_background: Some(ElementRect::default()),
        legend_position: Some(LegendPosition::Bottom),
        legend_title: Some(ElementText::default()),
        legend_text: Some(ElementText::default()),
        legend_background: Some(ElementRect::default()),
        strip_text: Some(ElementText::default()),
        strip_background: Some(ElementRect::default()),
        plot_margin: Some(Margin::default()),
    };
    assert_renders(base_plot().theme_update(update));

    // Default (all-None) update: exercises the no-op branches.
    assert_renders(base_plot().theme_update(ThemeUpdate::default()));
}

#[test]
fn legend_positions_render() {
    for pos in [
        LegendPosition::Top,
        LegendPosition::Bottom,
        LegendPosition::Left,
        LegendPosition::Right,
        LegendPosition::None,
    ] {
        let t = theme_gray().set_legend_position(pos);
        assert_renders(base_plot().theme(t));
    }
}

// ─── 3. Coordinates ─────────────────────────────────────────────────

#[test]
fn coord_flip_render() {
    assert_renders(base_plot().coord_flip());
}

#[test]
fn coord_fixed_render() {
    assert_renders(base_plot().coord_fixed(1.0));
    assert_renders(base_plot().coord_fixed(2.0));
}

#[test]
fn coord_polar_render() {
    assert_renders(base_plot().coord_polar());
    let cp = CoordPolar::new().theta("x").start(0.5).direction(-1.0);
    assert_renders(base_plot().coord_polar_with(cp));
}

#[test]
fn coord_cartesian_zoom_render() {
    assert_renders(base_plot().coord_cartesian_zoom(Some((0.0, 10.0)), Some((0.0, 6.0))));
    assert_renders(base_plot().coord_cartesian_zoom(Some((1.0, 8.0)), None));
    assert_renders(base_plot().coord_cartesian_zoom(None, Some((0.0, 5.0))));
    assert_renders(base_plot().coord_cartesian_zoom(None, None));
}

// ─── 4. Facets ──────────────────────────────────────────────────────

#[test]
fn facet_wrap_render() {
    assert_renders(base_plot().facet_wrap("g", Some(2)));
    assert_renders(base_plot().facet_wrap("g", None));
}

#[test]
fn facet_wrap_free_scales_render() {
    for scales in [
        FacetScales::Fixed,
        FacetScales::FreeX,
        FacetScales::FreeY,
        FacetScales::Free,
    ] {
        assert_renders(base_plot().facet_wrap_free("g", Some(2), scales));
    }
}

#[test]
fn facet_wrap_labeller_render() {
    assert_renders(base_plot().facet_wrap_labeller("g", Some(2), FacetLabeller::default()));
}

#[test]
fn facet_grid_render() {
    assert_renders(base_plot().facet_grid(Some("r"), Some("c")));
    assert_renders(base_plot().facet_grid(Some("r"), None));
    assert_renders(base_plot().facet_grid(None, Some("c")));
}

#[test]
fn facet_grid_free_scales_render() {
    for scales in [
        FacetScales::Fixed,
        FacetScales::FreeX,
        FacetScales::FreeY,
        FacetScales::Free,
    ] {
        assert_renders(base_plot().facet_grid_free(Some("r"), Some("c"), scales));
    }
}

#[test]
fn facet_grid_labeller_render() {
    assert_renders(base_plot().facet_grid_labeller(Some("r"), Some("c"), FacetLabeller::default()));
}

// ─── 5. Annotations ─────────────────────────────────────────────────

#[test]
fn annotations_render() {
    let plot = base_plot()
        .annotate_text("hello", 5.0, 3.0)
        .annotate_rect(1.0, 4.0, 1.0, 3.0)
        .annotate_segment(0.0, 0.0, 8.0, 5.0);
    assert_renders(plot);
}

#[test]
fn annotation_via_annotate_enum_render() {
    let plot = base_plot()
        .annotate(Annotation::text("a", 2.0, 2.0))
        .annotate(Annotation::rect(0.5, 2.5, 0.5, 2.5))
        .annotate(Annotation::segment(1.0, 1.0, 5.0, 4.0));
    assert_renders(plot);
}
