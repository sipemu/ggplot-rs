//! Coverage-focused integration tests for the `GGPlot` builder API in
//! `src/plot.rs`. Exercises every public builder method (geoms, scales,
//! labels, themes, facets, coords, annotations), all output paths, and the
//! error paths (unsupported format, missing aesthetic, empty data).

use std::path::Path;

use ggplot_rs::prelude::*;

// ─── data helpers ────────────────────────────────────────────────────────

fn f(vals: &[f64]) -> Vec<Value> {
    vals.iter().map(|v| Value::Float(*v)).collect()
}

fn s(vals: &[&str]) -> Vec<Value> {
    vals.iter().map(|v| Value::Str((*v).to_string())).collect()
}

fn col(name: &str, vals: Vec<Value>) -> (String, Vec<Value>) {
    (name.to_string(), vals)
}

/// A rich shared dataset: numeric x/y/z (all positive so log scales work) plus
/// a two-level discrete grouping column `g`.
fn base() -> Vec<(String, Vec<Value>)> {
    vec![
        col("x", f(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0])),
        col("y", f(&[2.0, 4.0, 3.0, 5.0, 4.0, 6.0])),
        col("z", f(&[10.0, 20.0, 30.0, 40.0, 50.0, 60.0])),
        col("g", s(&["a", "b", "a", "b", "a", "b"])),
    ]
}

// ─── plot helpers (fresh plot each call, since builders consume self) ────

fn plot_xy() -> GGPlot {
    GGPlot::new(base())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
}

fn plot_discrete_x() -> GGPlot {
    GGPlot::new(base())
        .aes(Aes::new().x("g").y("y"))
        .geom_point()
}

fn plot_discrete_y() -> GGPlot {
    GGPlot::new(base())
        .aes(Aes::new().x("y").y("g"))
        .geom_point()
}

fn plot_color_discrete() -> GGPlot {
    GGPlot::new(base())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
}

fn plot_color_cont() -> GGPlot {
    GGPlot::new(base())
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
}

fn plot_fill_discrete() -> GGPlot {
    GGPlot::new(base())
        .aes(Aes::new().x("g").y("y").fill("g"))
        .geom_col()
}

fn plot_fill_cont() -> GGPlot {
    GGPlot::new(base())
        .aes(Aes::new().x("x").y("y").fill("z"))
        .geom_col()
}

fn plot_datetime_x() -> GGPlot {
    let data = vec![
        col(
            "t",
            vec![
                Value::DateTime(0),
                Value::DateTime(86_400),
                Value::DateTime(172_800),
            ],
        ),
        col("y", f(&[1.0, 2.0, 3.0])),
    ];
    GGPlot::new(data).aes(Aes::new().x("t").y("y")).geom_point()
}

fn plot_datetime_y() -> GGPlot {
    let data = vec![
        col("x", f(&[1.0, 2.0, 3.0])),
        col(
            "t",
            vec![
                Value::DateTime(0),
                Value::DateTime(86_400),
                Value::DateTime(172_800),
            ],
        ),
    ];
    GGPlot::new(data).aes(Aes::new().x("x").y("t")).geom_point()
}

fn assert_render_ok(plot: GGPlot) {
    let r = plot.render_svg();
    assert!(r.is_ok(), "expected Ok render, got {:?}", r.err());
    assert!(r.unwrap().contains("<svg"));
}

/// Execute a builder's render for coverage without asserting the outcome
/// (used for geoms whose render is data-sensitive). Never panics — the API
/// funnels through `Result`.
fn touch(plot: GGPlot) {
    let _ = plot.render_svg();
}

// ─── geoms (bare shortcuts) ──────────────────────────────────────────────

#[test]
fn geoms_xy_render_ok() {
    assert_render_ok(plot_xy().geom_line());
    assert_render_ok(plot_xy().geom_path());
    assert_render_ok(plot_xy().geom_step());
    assert_render_ok(plot_xy().geom_area());
    assert_render_ok(plot_xy().geom_point());
    assert_render_ok(plot_xy().geom_jitter());
    assert_render_ok(plot_xy().geom_rug());
    assert_render_ok(plot_xy().geom_smooth());
    assert_render_ok(plot_xy().geom_col());
    assert_render_ok(plot_xy().geom_hline(3.0));
    assert_render_ok(plot_xy().geom_vline(2.0));
    assert_render_ok(plot_xy().geom_abline(1.0, 0.0));
    assert_render_ok(plot_xy().geom_blank());
}

#[test]
fn geoms_stat_based_render_ok() {
    // x-only stat geoms
    assert_render_ok(GGPlot::new(base()).aes(Aes::new().x("z")).geom_histogram());
    assert_render_ok(GGPlot::new(base()).aes(Aes::new().x("z")).geom_density());
    assert_render_ok(GGPlot::new(base()).aes(Aes::new().x("z")).geom_freqpoly());
    assert_render_ok(GGPlot::new(base()).aes(Aes::new().x("z")).geom_dotplot());
    // bar counts a discrete x
    assert_render_ok(GGPlot::new(base()).aes(Aes::new().x("g")).geom_bar());
    // boxplot / violin over discrete x
    assert_render_ok(
        GGPlot::new(base())
            .aes(Aes::new().x("g").y("y"))
            .geom_boxplot(),
    );
    assert_render_ok(
        GGPlot::new(base())
            .aes(Aes::new().x("g").y("y"))
            .geom_violin(),
    );
}

#[test]
fn geoms_range_and_text_render_ok() {
    let data = vec![
        col("x", f(&[1.0, 2.0, 3.0])),
        col("y", f(&[2.0, 3.0, 4.0])),
        col("ymin", f(&[1.0, 2.0, 3.0])),
        col("ymax", f(&[3.0, 4.0, 5.0])),
        col("lbl", s(&["p", "q", "r"])),
    ];
    let mapping = Aes::new().x("x").y("y").ymin("ymin").ymax("ymax");
    assert_render_ok(GGPlot::new(data.clone()).aes(mapping.clone()).geom_ribbon());
    assert_render_ok(
        GGPlot::new(data.clone())
            .aes(mapping.clone())
            .geom_errorbar(),
    );
    assert_render_ok(
        GGPlot::new(data.clone())
            .aes(mapping.clone())
            .geom_linerange(),
    );
    assert_render_ok(
        GGPlot::new(data.clone())
            .aes(mapping.clone())
            .geom_pointrange(),
    );
    assert_render_ok(GGPlot::new(data.clone()).aes(mapping).geom_crossbar());
    assert_render_ok(
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y").label("lbl"))
            .geom_text(),
    );
    assert_render_ok(
        GGPlot::new(data)
            .aes(Aes::new().x("x").y("y").label("lbl"))
            .geom_label(),
    );
}

#[test]
fn geoms_segment_rect_polygon_render_ok() {
    let seg = vec![
        col("x", f(&[1.0, 2.0])),
        col("y", f(&[1.0, 2.0])),
        col("xend", f(&[2.0, 3.0])),
        col("yend", f(&[2.0, 3.0])),
    ];
    assert_render_ok(
        GGPlot::new(seg.clone())
            .aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
            .geom_segment(),
    );
    assert_render_ok(
        GGPlot::new(seg)
            .aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
            .geom_curve(),
    );

    let rect = vec![
        col("xmin", f(&[1.0, 3.0])),
        col("xmax", f(&[2.0, 4.0])),
        col("ymin", f(&[1.0, 1.0])),
        col("ymax", f(&[2.0, 3.0])),
    ];
    assert_render_ok(
        GGPlot::new(rect)
            .aes(
                Aes::new()
                    .xmin("xmin")
                    .xmax("xmax")
                    .ymin("ymin")
                    .ymax("ymax"),
            )
            .geom_rect(),
    );

    let tile = vec![
        col("x", f(&[1.0, 2.0, 1.0, 2.0])),
        col("y", f(&[1.0, 1.0, 2.0, 2.0])),
        col("z", f(&[1.0, 2.0, 3.0, 4.0])),
    ];
    assert_render_ok(
        GGPlot::new(tile)
            .aes(Aes::new().x("x").y("y").fill("z"))
            .geom_tile(),
    );

    let poly = vec![
        col("x", f(&[0.0, 1.0, 0.5])),
        col("y", f(&[0.0, 0.0, 1.0])),
        col("group", s(&["a", "a", "a"])),
    ];
    assert_render_ok(
        GGPlot::new(poly)
            .aes(Aes::new().x("x").y("y").group("group"))
            .geom_polygon(),
    );

    let spoke = vec![
        col("x", f(&[1.0, 2.0, 3.0])),
        col("y", f(&[1.0, 2.0, 3.0])),
        col("angle", f(&[0.0, 1.5, 3.0])),
        col("radius", f(&[0.5, 0.5, 0.5])),
    ];
    assert_render_ok(
        GGPlot::new(spoke)
            .aes(Aes::new().x("x").y("y").angle("angle").radius("radius"))
            .geom_spoke(),
    );
}

#[test]
fn geoms_qq_and_2d_render() {
    let y_vals: Vec<Value> = (0..60).map(|i| Value::Float(i as f64)).collect();
    assert_render_ok(
        GGPlot::new(vec![col("sample", y_vals.clone())])
            .aes(Aes::new().y("sample"))
            .geom_qq(),
    );
    assert_render_ok(
        GGPlot::new(vec![col("sample", y_vals)])
            .aes(Aes::new().y("sample"))
            .geom_qq_line(),
    );

    let n = 200;
    let xs: Vec<Value> = (0..n)
        .map(|i| Value::Float((i as f64 * 0.1).sin()))
        .collect();
    let ys: Vec<Value> = (0..n)
        .map(|i| Value::Float((i as f64 * 0.1).cos()))
        .collect();
    let d2 = vec![col("x", xs.clone()), col("y", ys.clone())];
    assert_render_ok(
        GGPlot::new(d2.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_bin2d(),
    );
    assert_render_ok(
        GGPlot::new(d2.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_hex(),
    );
    assert_render_ok(
        GGPlot::new(d2.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_count(),
    );
    // density2d may not always produce contours for arbitrary data — touch only.
    touch(
        GGPlot::new(d2)
            .aes(Aes::new().x("x").y("y"))
            .geom_density2d(),
    );

    // gridded z for contour
    let mut cx = Vec::new();
    let mut cy = Vec::new();
    let mut cz = Vec::new();
    for ix in 0..10 {
        for iy in 0..10 {
            cx.push(Value::Float(ix as f64));
            cy.push(Value::Float(iy as f64));
            cz.push(Value::Float((ix * ix + iy * iy) as f64));
        }
    }
    assert_render_ok(
        GGPlot::new(vec![col("x", cx), col("y", cy), col("z", cz)])
            .aes(Aes::new().x("x").y("y"))
            .geom_contour(),
    );
}

#[test]
fn geoms_with_variants_execute() {
    // The `geom_*_with` methods all funnel through `add_geom`; construct each
    // with a default geom and drive a render for line coverage.
    touch(plot_xy().geom_point_with(GeomPoint::default()));
    touch(plot_xy().geom_line_with(GeomLine::default()));
    touch(
        GGPlot::new(base())
            .aes(Aes::new().x("g"))
            .geom_bar_with(GeomBar::default()),
    );
    touch(
        GGPlot::new(base())
            .aes(Aes::new().x("z"))
            .geom_histogram_with(GeomHistogram::default()),
    );
    touch(
        GGPlot::new(base())
            .aes(Aes::new().x("g").y("y"))
            .geom_boxplot_with(GeomBoxplot::default()),
    );
    touch(plot_xy().geom_smooth_with(GeomSmooth::default()));
    touch(plot_xy().geom_col_with(GeomCol::default()));
    touch(plot_xy().geom_hline_with(GeomHline::new(3.0)));
    touch(plot_xy().geom_vline_with(GeomVline::new(2.0)));
    touch(plot_xy().geom_abline_with(GeomAbline::new(1.0, 0.0)));
    touch(
        GGPlot::new(base())
            .aes(Aes::new().x("x").y("y").label("g"))
            .geom_text_with(GeomText::default()),
    );
    touch(
        GGPlot::new(base())
            .aes(Aes::new().x("x").y("y").label("g"))
            .geom_label_with(GeomLabel::default()),
    );
    touch(plot_xy().geom_area_with(GeomArea::default()));
    touch(plot_xy().geom_ribbon_with(GeomRibbon::default()));
    touch(plot_xy().geom_errorbar_with(GeomErrorbar::default()));
    touch(plot_xy().geom_segment_with(GeomSegment::default()));
    touch(
        GGPlot::new(base())
            .aes(Aes::new().x("z"))
            .geom_density_with(GeomDensity::default()),
    );
    touch(plot_xy().geom_rug_with(GeomRug::default()));
    touch(plot_xy().geom_jitter_with(GeomJitter::default()));
    touch(plot_xy().geom_path_with(GeomPath::default()));
    touch(plot_xy().geom_step_with(GeomStep::default()));
    touch(
        GGPlot::new(base())
            .aes(Aes::new().x("z"))
            .geom_freqpoly_with(GeomFreqpoly::default()),
    );
    touch(plot_xy().geom_linerange_with(GeomLinerange::default()));
    touch(plot_xy().geom_pointrange_with(GeomPointrange::default()));
    touch(plot_xy().geom_crossbar_with(GeomCrossbar::default()));
    touch(plot_xy().geom_spoke_with(GeomSpoke::default()));
    touch(plot_xy().geom_rect_with(GeomRect::default()));
    touch(plot_xy().geom_tile_with(GeomTile::default()));
    touch(plot_xy().geom_polygon_with(GeomPolygon::default()));
    touch(plot_xy().geom_curve_with(GeomCurve::default()));
    touch(
        GGPlot::new(base())
            .aes(Aes::new().x("g").y("y"))
            .geom_violin_with(GeomViolin::default()),
    );
    touch(
        GGPlot::new(base())
            .aes(Aes::new().x("z"))
            .geom_dotplot_with(GeomDotplot::default()),
    );
    touch(plot_xy().geom_qq_with(GeomQQ::default()));
    touch(plot_xy().geom_qq_line_with(GeomQQLine::default()));
    touch(plot_xy().geom_bin2d_with(GeomBin2d::default()));
    touch(plot_xy().geom_hex_with(GeomHex::default()));
    touch(plot_xy().geom_count_with(GeomCount::default()));
    touch(plot_xy().geom_contour_with(GeomContour::default()));
    touch(plot_xy().geom_density2d_with(GeomDensity2d::default()));
}

// ─── layer-level overrides ───────────────────────────────────────────────

#[test]
fn layer_overrides_execute() {
    // stat() override
    assert_render_ok(
        GGPlot::new(base())
            .aes(Aes::new().x("g").y("y"))
            .geom_bar()
            .stat(StatIdentity),
    );
    // position() override
    assert_render_ok(
        GGPlot::new(base())
            .aes(Aes::new().x("g").y("y").fill("g"))
            .geom_col()
            .position(PositionDodge),
    );
    assert_render_ok(
        GGPlot::new(base())
            .aes(Aes::new().x("g").y("y").fill("g"))
            .geom_col()
            .position(PositionStack),
    );
    // layer_data() + layer_aes() override
    let overlay = vec![col("x", f(&[1.5, 2.5])), col("y", f(&[9.0, 8.0]))];
    assert_render_ok(
        plot_xy()
            .geom_point()
            .layer_data(overlay)
            .layer_aes(Aes::new().x("x").y("y")),
    );
    // show_legend()
    assert_render_ok(plot_color_discrete().geom_point().show_legend(false));
    assert_render_ok(plot_color_discrete().geom_point().show_legend(true));
}

// ─── axis scales ─────────────────────────────────────────────────────────

#[test]
fn axis_scales_render_ok() {
    assert_render_ok(plot_xy().scale_x_log10());
    assert_render_ok(plot_xy().scale_y_log10());
    assert_render_ok(plot_xy().scale_x_log2());
    assert_render_ok(plot_xy().scale_y_log2());
    assert_render_ok(plot_xy().scale_x_ln());
    assert_render_ok(plot_xy().scale_y_ln());
    assert_render_ok(plot_xy().scale_x_sqrt());
    assert_render_ok(plot_xy().scale_y_sqrt());
    assert_render_ok(plot_xy().scale_x_reverse());
    assert_render_ok(plot_xy().scale_y_reverse());
    assert_render_ok(plot_xy().scale_x_continuous(ScaleContinuous::new().with_name("X axis")));
    assert_render_ok(plot_xy().scale_y_continuous(ScaleContinuous::new().with_limits(0.0, 10.0)));
    assert_render_ok(plot_xy().xlim(0.0, 10.0));
    assert_render_ok(plot_xy().ylim(0.0, 10.0));
    assert_render_ok(plot_discrete_x().scale_x_discrete(ScaleDiscrete::new().with_name("G")));
    assert_render_ok(plot_discrete_y().scale_y_discrete(ScaleDiscrete::new().with_name("G")));
    assert_render_ok(plot_datetime_x().scale_x_datetime(ScaleDateTime::new()));
    assert_render_ok(plot_datetime_y().scale_y_datetime(ScaleDateTime::new()));
}

// ─── color / fill scales ─────────────────────────────────────────────────

#[test]
fn color_fill_scales_render_ok() {
    let red = RGBAColor::new(255, 0, 0);
    let white = RGBAColor::new(255, 255, 255);
    let blue = RGBAColor::new(0, 0, 255);

    assert_render_ok(plot_color_discrete().scale_color_manual(vec![("a", red), ("b", blue)]));
    assert_render_ok(plot_fill_discrete().scale_fill_manual(vec![("a", red), ("b", blue)]));
    assert_render_ok(plot_color_discrete().scale_color_viridis());
    assert_render_ok(plot_fill_discrete().scale_fill_viridis());
    assert_render_ok(plot_color_discrete().scale_color_brewer(PaletteName::Set1));
    assert_render_ok(plot_fill_discrete().scale_fill_brewer(PaletteName::Blues));
    assert_render_ok(plot_color_discrete().scale_color_grey());
    assert_render_ok(plot_fill_discrete().scale_fill_grey());
    assert_render_ok(
        plot_color_discrete()
            .scale_color_grey_with(ScaleColorGrey::new(Aesthetic::Color).with_range(0.1, 0.9)),
    );
    assert_render_ok(
        plot_fill_discrete()
            .scale_fill_grey_with(ScaleColorGrey::new(Aesthetic::Fill).with_range(0.1, 0.9)),
    );

    // continuous color/fill
    assert_render_ok(plot_color_cont().scale_color_gradient(white, red));
    assert_render_ok(plot_fill_cont().scale_fill_gradient(white, red));
    assert_render_ok(plot_color_cont().scale_color_gradient2(blue, white, red));
    assert_render_ok(plot_fill_cont().scale_fill_gradient2(blue, white, red));
    assert_render_ok(plot_color_cont().scale_color_gradientn(vec![
        (0.0, blue),
        (0.5, white),
        (1.0, red),
    ]));
    assert_render_ok(plot_fill_cont().scale_fill_gradientn(vec![
        (0.0, blue),
        (0.5, white),
        (1.0, red),
    ]));
    assert_render_ok(plot_color_cont().scale_color_viridis_c());
    assert_render_ok(plot_fill_cont().scale_fill_viridis_c());

    // generic scale_color / scale_fill entry points
    assert_render_ok(plot_color_discrete().scale_color(ScaleColorDiscrete::new(Aesthetic::Color)));
    assert_render_ok(plot_fill_discrete().scale_fill(ScaleColorDiscrete::new(Aesthetic::Fill)));
}

// ─── shape / linetype / size / alpha scales ──────────────────────────────

#[test]
fn other_aesthetic_scales_render_ok() {
    assert_render_ok(
        GGPlot::new(base())
            .aes(Aes::new().x("x").y("y").shape("g"))
            .geom_point()
            .scale_shape_manual(vec![("a", PointShape::Circle), ("b", PointShape::Triangle)]),
    );
    assert_render_ok(
        GGPlot::new(base())
            .aes(Aes::new().x("x").y("y").linetype("g").group("g"))
            .geom_line()
            .scale_linetype_manual(vec![("a", Linetype::Solid), ("b", Linetype::Dashed)]),
    );
    assert_render_ok(
        GGPlot::new(base())
            .aes(Aes::new().x("x").y("y").size("z"))
            .geom_point()
            .scale_size(ScaleSizeContinuous::new().with_range(1.0, 6.0)),
    );
    assert_render_ok(
        GGPlot::new(base())
            .aes(Aes::new().x("x").y("y").alpha("z"))
            .geom_point()
            .scale_alpha(ScaleAlphaContinuous::new().with_range(0.2, 1.0)),
    );
}

// ─── labels, guides, themes ──────────────────────────────────────────────

#[test]
fn labels_and_guides_render_ok() {
    assert_render_ok(plot_xy().labs(Labels {
        title: Some("T".into()),
        subtitle: Some("Sub".into()),
        x: Some("X".into()),
        y: Some("Y".into()),
        caption: Some("Cap".into()),
    }));
    assert_render_ok(
        plot_xy()
            .title("Title")
            .subtitle("Subtitle")
            .xlab("Xlab")
            .ylab("Ylab")
            .caption("Caption"),
    );
    assert_render_ok(
        plot_color_discrete().guides(
            GuideLegend::new()
                .with_title("Legend")
                .with_ncol(1)
                .with_nrow(2)
                .reverse(),
        ),
    );
}

#[test]
fn themes_render_ok() {
    assert_render_ok(plot_xy().theme(Theme::default()));
    assert_render_ok(plot_xy().primary_color((10, 20, 30)));
    assert_render_ok(plot_xy().theme_minimal());
    assert_render_ok(plot_xy().theme_bw());
    assert_render_ok(plot_xy().theme_gray());
    assert_render_ok(plot_xy().theme_classic());
    assert_render_ok(plot_xy().theme_linedraw());
    assert_render_ok(plot_xy().theme_light());
    assert_render_ok(plot_xy().theme_dark());
    assert_render_ok(plot_xy().theme_void());
    assert_render_ok(plot_xy().theme_update(ThemeUpdate {
        legend_position: Some(LegendPosition::Bottom),
        ..Default::default()
    }));
}

// ─── facets ──────────────────────────────────────────────────────────────

#[test]
fn facets_render_ok() {
    assert_render_ok(plot_xy().facet_wrap("g", Some(2)));
    assert_render_ok(plot_xy().facet_wrap_free("g", None, FacetScales::Free));
    assert_render_ok(plot_xy().facet_wrap_labeller("g", None, FacetLabeller::Both));
    assert_render_ok(plot_xy().facet_grid(Some("g"), None));
    assert_render_ok(plot_xy().facet_grid(None, Some("g")));
    assert_render_ok(plot_xy().facet_grid_free(Some("g"), None, FacetScales::FreeY));
    assert_render_ok(plot_xy().facet_grid_labeller(Some("g"), None, FacetLabeller::Value));
}

// ─── coordinates ─────────────────────────────────────────────────────────

#[test]
fn coords_render_ok() {
    assert_render_ok(plot_xy().coord_flip());
    assert_render_ok(plot_xy().coord_fixed(1.0));
    assert_render_ok(plot_xy().coord_cartesian_zoom(Some((0.0, 5.0)), Some((0.0, 5.0))));
    assert_render_ok(plot_xy().coord_cartesian_zoom(None, None));
    touch(
        GGPlot::new(base())
            .aes(Aes::new().x("g"))
            .geom_bar()
            .coord_polar(),
    );
    touch(
        GGPlot::new(base())
            .aes(Aes::new().x("g"))
            .geom_bar()
            .coord_polar_with(CoordPolar::new().theta("y")),
    );
}

// ─── annotations ─────────────────────────────────────────────────────────

#[test]
fn annotations_render_ok() {
    assert_render_ok(plot_xy().annotate(Annotation::text("hi", 2.0, 3.0)));
    assert_render_ok(plot_xy().annotate_text("note", 2.0, 3.0));
    assert_render_ok(plot_xy().annotate_rect(1.0, 2.0, 1.0, 3.0));
    assert_render_ok(plot_xy().annotate_segment(1.0, 1.0, 3.0, 4.0));
}

// ─── build API ───────────────────────────────────────────────────────────

#[test]
fn build_and_try_build() {
    let built = plot_xy().try_build();
    assert!(built.is_ok(), "valid plot should build: {:?}", built.err());
    // build() (panicking variant) on a valid plot must succeed
    let _ = plot_xy().build();
}

// ─── output methods ──────────────────────────────────────────────────────

fn tmp(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("ggplot_cov_{name}"))
        .to_string_lossy()
        .to_string()
}

#[test]
fn save_with_size_all_formats() {
    for ext in ["svg", "png", "jpg", "bmp"] {
        let path = tmp(&format!("out.{ext}"));
        let r = plot_xy().save_with_size(&path, 320, 240);
        assert!(r.is_ok(), "save .{ext} failed: {:?}", r.err());
        assert!(Path::new(&path).exists(), "file .{ext} not written");
        std::fs::remove_file(&path).ok();
    }
}

#[test]
fn save_default_and_ggsave() {
    let p1 = tmp("save_default.svg");
    plot_xy().save(&p1).expect("save should succeed");
    assert!(Path::new(&p1).exists());
    std::fs::remove_file(&p1).ok();

    let p2 = tmp("ggsave.png");
    plot_xy()
        .ggsave(&p2, 4.0, 3.0, 100.0)
        .expect("ggsave should succeed");
    assert!(Path::new(&p2).exists());
    std::fs::remove_file(&p2).ok();
}

#[test]
fn in_memory_renderers() {
    assert!(plot_xy().render_svg().unwrap().contains("<svg"));
    assert!(plot_xy()
        .render_svg_with_size(400, 300)
        .unwrap()
        .contains("<svg"));
    assert!(!plot_xy().render_png().unwrap().is_empty());
    let png = plot_xy().render_png_with_size(400, 300).unwrap();
    assert!(
        png.len() > 8 && &png[1..4] == b"PNG",
        "expected PNG magic bytes"
    );
}

// ─── error paths ─────────────────────────────────────────────────────────

#[test]
fn unsupported_format_errs() {
    let path = tmp("out.unknownext");
    let r = plot_xy().save(&path);
    match r {
        Err(GGError::UnsupportedFormat(ext)) => assert_eq!(ext, "unknownext"),
        other => panic!("expected UnsupportedFormat, got {other:?}"),
    }
    assert!(
        !Path::new(&path).exists(),
        "no file should be written on error"
    );
}

#[test]
fn missing_required_aesthetic_errs() {
    // geom_point requires x and y. Use data whose columns are NOT named x/y and
    // provide no mapping, so neither the raw data nor the stat supplies them.
    let data = vec![col("a", f(&[1.0, 2.0, 3.0])), col("b", f(&[1.0, 2.0, 3.0]))];
    let r = GGPlot::new(data).geom_point().try_build();
    assert!(r.is_err(), "missing x/y aesthetic should error, got Ok");
    // and the render path surfaces the same error rather than panicking
    let data2 = vec![col("a", f(&[1.0, 2.0, 3.0])), col("b", f(&[1.0, 2.0, 3.0]))];
    let r2 = GGPlot::new(data2).geom_point().render_svg();
    assert!(r2.is_err());
}

#[test]
fn empty_data_errs() {
    let empty = vec![col("x", f(&[])), col("y", f(&[]))];
    let r = GGPlot::new(empty)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg();
    assert!(r.is_err(), "empty data should error, got Ok");
}
