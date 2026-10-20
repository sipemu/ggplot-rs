//! Degenerate / hostile data must degrade gracefully: empty plots render an
//! empty panel, layers whose stat cannot estimate anything draw nothing (other
//! layers still render), non-finite positions are dropped with a warning, and
//! no `NaN`/`inf` ever reaches an SVG attribute.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::smooth::GeomSmooth;
use ggplot_rs::prelude::*;
use ggplot_rs::stat::smooth::SmoothMethod;
use ggplot_rs::GGPlot;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}

fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// Every attribute value in the document is free of non-finite numbers.
fn assert_finite_attrs(svg: &str) {
    for (i, _) in svg.match_indices("=\"") {
        let rest = &svg[i + 2..];
        let val = &rest[..rest.find('"').unwrap()];
        for tok in val.split([' ', ',', '(', ')']) {
            let t = tok.trim_start_matches('-');
            assert!(
                t != "NaN" && t != "inf" && t != "infinity",
                "non-finite attribute value {val:?} in svg"
            );
        }
    }
}

fn count(svg: &str, needle: &str) -> usize {
    svg.matches(needle).count()
}

#[test]
fn empty_frame_renders_empty_panel_with_title() {
    let data = frame(vec![("x", vec![]), ("y", vec![])]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_col()
        .title("Nothing here")
        .render_svg_native_with_size(400, 300)
        .expect("an empty plot renders");
    assert!(svg.contains("Nothing here"));
    assert!(svg.starts_with("<svg"));
    assert_finite_attrs(&svg);
}

#[test]
fn plot_without_columns_or_layers_renders() {
    let svg = GGPlot::new(ggplot_rs::data::DataFrame::new())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("T")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains(">T</text>"));
    assert_finite_attrs(&svg);

    let svg = GGPlot::new(ggplot_rs::data::DataFrame::new())
        .title("no layers")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("no layers"));
}

#[test]
fn missing_aesthetic_on_real_data_still_errors() {
    let data = frame(vec![("a", floats(&[1.0, 2.0]))]);
    let r = GGPlot::new(data)
        .aes(Aes::new().x("a"))
        .geom_point()
        .render_svg_native_with_size(300, 200);
    assert!(
        r.is_err(),
        "geom_point without y must still be a validation error"
    );
}

#[test]
fn single_value_density_draws_nothing_but_plot_renders() {
    let data = frame(vec![("x", floats(&[3.0])), ("y", floats(&[1.0]))]);
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_density()
        .render_svg_native_with_warnings(400, 300)
        .expect("a degenerate density layer must not fail the plot");
    assert!(count(&svg, "<circle") >= 1, "the point layer still renders");
    assert!(
        warnings.iter().any(|w| w.contains("stat_density")),
        "warning about the empty stat: {warnings:?}"
    );
    assert_finite_attrs(&svg);
}

#[test]
fn single_value_violin_and_qq_do_not_fail() {
    let data = frame(vec![
        ("g", vec![Value::Str("a".into())]),
        ("y", floats(&[2.0])),
    ]);
    let svg = GGPlot::new(data.clone())
        .aes(Aes::new().x("g").y("y"))
        .geom_violin()
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_finite_attrs(&svg);

    let svg = GGPlot::new(frame(vec![("y", floats(&[2.0]))]))
        .aes(Aes::new().y("y"))
        .geom_qq()
        .geom_qq_line()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_finite_attrs(&svg);
}

#[test]
fn loess_on_two_points_skips_only_the_smoother() {
    let data = frame(vec![("x", floats(&[1.0, 2.0])), ("y", floats(&[3.0, 5.0]))]);
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth {
            method: SmoothMethod::Loess { span: 0.75 },
            ..Default::default()
        })
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert!(count(&svg, "<circle") >= 2);
    assert!(warnings.iter().any(|w| w.contains("produced no data")));
    assert_finite_attrs(&svg);
}

#[test]
fn non_finite_rows_are_dropped_with_warning() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0, 4.0, f64::NAN])),
        ("y", floats(&[1.0, f64::NAN, f64::INFINITY, 4.0, 5.0])),
    ]);
    let built = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .try_build()
        .unwrap();
    assert_eq!(built.layers[0].data.nrows(), 2);
    assert_eq!(built.warnings().len(), 1);
    assert!(built.warnings()[0].contains("removed 3 rows containing non-finite values"));

    for plot in [
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_line(),
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_area(),
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_col(),
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_smooth(),
        GGPlot::new(data.clone())
            .aes(Aes::new().x("y"))
            .geom_histogram(),
        GGPlot::new(data.clone())
            .aes(Aes::new().x("y"))
            .geom_density(),
    ] {
        let svg = plot.render_svg_native_with_size(300, 200).unwrap();
        assert_finite_attrs(&svg);
    }
}

#[test]
fn all_non_finite_renders_empty_panel() {
    let data = frame(vec![
        ("x", floats(&[f64::NAN, f64::INFINITY])),
        ("y", floats(&[f64::NEG_INFINITY, f64::NAN])),
    ]);
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("all bad")
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert!(svg.contains("all bad"));
    assert_eq!(count(&svg, "<circle"), 0);
    assert_eq!(warnings.len(), 1);
    assert_finite_attrs(&svg);
}

#[test]
fn zero_spread_density_uses_positive_bandwidth() {
    let data = frame(vec![("x", floats(&[5.0, 5.0, 5.0, 5.0]))]);
    let built = GGPlot::new(data.clone())
        .aes(Aes::new().x("x"))
        .geom_density()
        .try_build()
        .unwrap();
    let ys = built.layers[0].data.column("y").unwrap();
    assert!(!ys.is_empty());
    assert!(ys.iter().all(|v| v.as_f64().unwrap().is_finite()));
    assert!(ys.iter().any(|v| v.as_f64().unwrap() > 0.0));

    // A constant zero sample falls back to bw = 0.9 · 1 · n^-0.2 (R's bw.nrd0).
    let zeros = frame(vec![("x", floats(&[0.0, 0.0]))]);
    let built = GGPlot::new(zeros)
        .aes(Aes::new().x("x"))
        .geom_density()
        .try_build()
        .unwrap();
    let xs = built.layers[0].data.column("x").unwrap();
    let lo = xs.first().unwrap().as_f64().unwrap();
    let bw = 0.9 * 2f64.powf(-0.2);
    assert!((lo + 3.0 * bw).abs() < 1e-9, "x range is ±3 bw: {lo}");

    // Violins of constant groups render without NaN.
    let v = frame(vec![
        ("g", vec![Value::Str("a".into()); 3]),
        ("y", floats(&[2.0, 2.0, 2.0])),
    ]);
    let svg = GGPlot::new(v)
        .aes(Aes::new().x("g").y("y"))
        .geom_violin()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_finite_attrs(&svg);
}

#[test]
fn log_of_zero_is_dropped_not_drawn() {
    let data = frame(vec![
        ("x", floats(&[0.0, 1.0, 10.0])),
        ("y", floats(&[1.0, 2.0, 3.0])),
    ]);
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_log10()
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert_eq!(count(&svg, "<circle"), 2);
    assert_eq!(warnings.len(), 1);
    assert_finite_attrs(&svg);
}

// ─── B2-1: non-finite guard ─────────────────────────────────────────────
//
// `draw_line` now splits at non-finite vertices into separate `<polyline>`
// runs instead of silently dropping the bad row and joining the finite
// points across the gap — line/path/step opt into keeping the row (as a
// break) via `Geom::keeps_na_as_break`.

#[test]
fn a_nan_in_the_middle_of_a_line_splits_it_into_separate_polylines() {
    // Review PR#141 finding 7: a run shorter than 2 points is dropped
    // (ggplot2 drops a one-observation path piece), so this isn't simply
    // "N breaks give N+1 polylines" — the lone `3` between the two breaks
    // vanishes, leaving exactly the two real (>=2-point) runs.
    //
    // `geom_step` is deliberately not in this loop: `stepped()` inserts an
    // extra corner point (the previous row's y at the current row's x, or
    // vice versa) between every pair of *adjacent source rows*, finite or
    // not, so a row next to a break still ends up paired with a corner
    // derived from its finite neighbour instead of standing alone as a
    // length-1 run. Its own break behaviour is covered by the dedicated
    // `geom_step_with_a_nan_row_breaks_the_line...` test below.
    let data = frame(vec![
        ("x", floats(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0])),
        ("y", floats(&[1.0, 2.0, f64::NAN, 3.0, f64::NAN, 4.0, 5.0])),
    ]);
    for (geom, plot) in [
        (
            "line",
            GGPlot::new(data.clone())
                .aes(Aes::new().x("x").y("y"))
                .geom_line(),
        ),
        (
            "path",
            GGPlot::new(data.clone())
                .aes(Aes::new().x("x").y("y"))
                .geom_path(),
        ),
    ] {
        let svg = plot.render_svg_native_with_size(300, 200).unwrap();
        assert_eq!(
            count(&svg, "class=\"gg-mark gg-line\""),
            2,
            "{geom}: [1,2,NaN,3,NaN,4,5] should give 2 polylines (the lone 3 is dropped): {svg}"
        );
        assert_finite_attrs(&svg);
    }
}

#[test]
fn a_run_of_one_finite_point_between_breaks_draws_nothing() {
    // x=2 is the only finite point between two NaN neighbours (and x=4 is
    // isolated at the end the same way): too short to be a line segment
    // (ggplot2 drops a one-observation path piece too), so it draws
    // nothing rather than an invisible 1-point mark that would still carry
    // a `<title>` and hover attributes. y has two distinct finite values
    // (2.0, 8.0) so the y scale's domain isn't degenerate — a degenerate
    // (single-value) domain would make `map()` return a constant 0.5
    // regardless of input, masking the very NaNs this test checks.
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0, 4.0])),
        ("y", floats(&[f64::NAN, 2.0, f64::NAN, 8.0])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(
        count(&svg, "class=\"gg-mark gg-line\""),
        0,
        "a lone finite point between two breaks draws nothing: {svg}"
    );
    assert_finite_attrs(&svg);
}

#[test]
fn geom_step_with_a_nan_row_breaks_the_line_not_a_phantom_spike_to_the_edge() {
    // Review PR#141 finding 1 (HIGH): GeomStep used to clamp *every*
    // non-finite pixel coordinate to the panel edge (a leftover from
    // stat_ecdf's +-Inf padding), so the NaN row became a phantom vertical
    // spike to the top of the panel instead of a break. Only +-Inf may
    // still clamp to the edge; NaN must split the line.
    let data = frame(vec![
        ("x", floats(&[0.0, 1.0, 2.0, 3.0])),
        ("y", floats(&[1.0, f64::NAN, 3.0, 4.0])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_step()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    // `stepped()`'s corner insertion turns the break at x=1 into two
    // genuine 2-point runs — (x=0,y=1)-(x=1,y=1) and
    // (x=2,y=3)-(x=3,y=3)-(x=3,y=4) — rather than one polyline spanning
    // the whole panel with a spike at the NaN row (the pre-fix bug).
    assert_eq!(count(&svg, "class=\"gg-mark gg-line\""), 2, "{svg}");
    assert_finite_attrs(&svg);
    // No vertex should ever sit at the bare panel-top edge (y = plot_area.y)
    // as a stand-in for NaN — the only legitimate edge-clamped coordinate
    // is +-Inf, which this plot never produces, and none of this data's
    // mapped y values (1, 3, 4, under a y domain of roughly [1, 4] with the
    // usual expansion) land exactly on the panel edge.
    let plot_area_top = svg
        .split("data-plot=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .and_then(|s| s.split(' ').nth(1))
        .expect("data-plot's y");
    let needle = format!(",{plot_area_top} ");
    let needle_end = format!(",{plot_area_top}\"");
    for p in count_all_tags(&svg, "polyline") {
        if !p.contains("gg-mark") {
            continue; // grid line
        }
        assert!(
            !p.contains(&needle) && !p.ends_with(&needle_end),
            "a data polyline should not touch the bare top edge from a clamped NaN: {p}"
        );
    }
}

/// Every `<tag …>` start tag in `svg` whose name is `tag`.
fn count_all_tags<'a>(svg: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{tag} ");
    svg.match_indices(&open)
        .map(|(i, _)| {
            let rest = &svg[i..];
            &rest[..rest.find('>').map_or(rest.len(), |e| e + 1)]
        })
        .collect()
}

#[test]
fn constant_single_row_and_nonfinite_data_render_through_every_connecting_geom() {
    let constant = frame(vec![
        ("x", floats(&[1.0, 1.0, 1.0])),
        ("y", floats(&[2.0, 2.0, 2.0])),
    ]);
    let single = frame(vec![("x", floats(&[1.0])), ("y", floats(&[2.0]))]);
    let with_gap = frame(vec![
        ("x", floats(&[0.0, 1.0, 2.0, 3.0])),
        ("y", floats(&[0.0, f64::NAN, f64::INFINITY, 3.0])),
    ]);
    for data in [constant, single, with_gap] {
        for plot in [
            GGPlot::new(data.clone())
                .aes(Aes::new().x("x").y("y"))
                .geom_line(),
            GGPlot::new(data.clone())
                .aes(Aes::new().x("x").y("y"))
                .geom_path(),
            GGPlot::new(data.clone())
                .aes(Aes::new().x("x").y("y"))
                .geom_step(),
            GGPlot::new(data.clone())
                .aes(Aes::new().x("x").y("y").xend("x").yend("y"))
                .geom_segment(),
            GGPlot::new(data.clone())
                .aes(Aes::new().x("x").y("y").xend("x").yend("y"))
                .geom_curve(),
        ] {
            let svg = plot.render_svg_native_with_size(300, 200).unwrap();
            assert_finite_attrs(&svg);
        }
    }
}

#[test]
fn arrows_drop_entirely_on_a_nonfinite_or_zero_length_path() {
    // A segment with a non-finite endpoint gets no arrowhead at all, not a
    // partial/NaN one.
    let data = frame(vec![
        ("x", floats(&[0.0, 0.0])),
        ("y", floats(&[0.0, 0.0])),
        ("xend", floats(&[f64::NAN, 0.0])),
        ("yend", floats(&[1.0, 0.0])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
        .geom_segment_with(GeomSegment::default().arrow(arrow().closed()))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(count(&svg, "data-part=\"arrow\""), 0);
    assert_finite_attrs(&svg);
}

#[test]
fn hexbin_handles_constant_single_row_and_nonfinite_weight() {
    let constant = frame(vec![
        ("x", floats(&[1.0, 1.0, 1.0])),
        ("y", floats(&[1.0, 1.0, 1.0])),
    ]);
    let single = frame(vec![("x", floats(&[1.0])), ("y", floats(&[1.0]))]);
    let with_bad = frame(vec![
        ("x", floats(&[0.0, 1.0, f64::NAN])),
        ("y", floats(&[0.0, 1.0, f64::INFINITY])),
    ]);
    for data in [constant, single, with_bad] {
        let svg = GGPlot::new(data)
            .aes(Aes::new().x("x").y("y"))
            .geom_hex()
            .render_svg_native_with_size(300, 200)
            .unwrap();
        assert_finite_attrs(&svg);
    }
}

#[test]
fn degenerate_data_renders_through_plotgrid_too() {
    // Two real (>=2-point) runs on each side of the break.
    let with_gap = frame(vec![
        ("x", floats(&[0.0, 1.0, 2.0, 3.0, 4.0])),
        ("y", floats(&[0.0, 1.0, f64::NAN, 2.0, 3.0])),
    ]);
    let p = || {
        GGPlot::new(with_gap.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_line()
    };
    let svg = (p() | p()).render_svg_native_with_size(600, 300).unwrap();
    assert!(
        count(&svg, "class=\"gg-mark gg-line\"") >= 4,
        "2 split runs per cell, 2 cells: {svg}"
    );
    assert_finite_attrs(&svg);
}
