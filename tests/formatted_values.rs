//! B2-4: per-mark formatted values. `GGPlot::svg_formatted_values(true)`
//! turns on `data-fvalue` (the value aesthetic's scale, formatted) and
//! `data-fx` (the x value's scale, formatted, when it differs from the
//! plain `data-x`) on marks; off (the default) adds nothing.

use ggplot_rs::locale::Locale;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::format::label_currency_locale;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// 2026-10-12T00:00:00Z.
const OCT_12_2026: f64 = 1_791_763_200.0;

#[test]
fn a_de_de_currency_value_gives_data_fvalue() {
    let data = frame(vec![
        ("label", strs(&["Berlin"])),
        ("x", floats(&[1.0])),
        ("y", floats(&[1234.5])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_continuous(
            ScaleContinuous::new()
                .with_value_formatter(label_currency_locale(&Locale::de_de(), Some(0.01))),
        )
        .svg_formatted_values(true)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(
        svg.contains("data-fvalue=\"1.234,50 €\""),
        "expected a de-DE currency data-fvalue: {svg}"
    );
}

#[test]
fn a_date_x_gives_data_fx_that_differs_from_data_x() {
    let data = frame(vec![("x", floats(&[OCT_12_2026])), ("y", floats(&[1.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_datetime(ScaleDateTime::new())
        .svg_formatted_values(true)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(
        svg.contains("data-fx=\"2026-10-12\""),
        "expected an ISO date data-fx: {svg}"
    );
    // The plain data-x is the raw epoch-seconds number, not the date —
    // that's exactly why data-fx exists and isn't suppressed as redundant.
    assert!(!svg.contains(&format!("data-x=\"{OCT_12_2026}\" data-fx")));
}

#[test]
fn the_flag_off_adds_nothing() {
    let data = frame(vec![
        ("label", strs(&["Berlin"])),
        ("x", floats(&[OCT_12_2026])),
        ("y", floats(&[1234.5])),
    ]);
    let plot = || {
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_x_datetime(ScaleDateTime::new())
            .scale_y_continuous(
                ScaleContinuous::new()
                    .with_value_formatter(label_currency_locale(&Locale::de_de(), None)),
            )
    };
    let off = plot().render_svg_native_with_size(300, 200).unwrap();
    assert!(!off.contains("data-fvalue"), "{off}");
    assert!(!off.contains("data-fx"), "{off}");

    let on = plot()
        .svg_formatted_values(true)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(on.contains("data-fvalue"));
    assert!(on.contains("data-fx"));
}

#[test]
fn a_vline_at_a_datetime_x_shows_the_iso_date_in_data_x_and_the_tooltip() {
    let base = || {
        GGPlot::new(frame(vec![("x", floats(&[1.0])), ("y", floats(&[1.0]))]))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_x_datetime(ScaleDateTime::new())
    };

    let off = base()
        .geom_vline(OCT_12_2026)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    // Off: the plain epoch-seconds number, not the date.
    assert!(off.contains(&format!("data-x=\"{OCT_12_2026}\"")), "{off}");

    let on = base()
        .geom_vline(OCT_12_2026)
        .svg_formatted_values(true)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(on.contains("data-x=\"2026-10-12\""), "{on}");
    assert!(
        on.contains("x = 2026-10-12"),
        "tooltip should show the date: {on}"
    );
}

#[test]
fn a_hex_fvalue_comes_from_the_fill_aesthetic() {
    let data = frame(vec![
        ("x", floats(&[1.0, 1.0, 1.0])),
        ("y", floats(&[1.0, 1.0, 1.0])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").after_stat_fill("count"))
        .geom_hex()
        .svg_formatted_values(true)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("data-fvalue=\"3\""), "{svg}");
}

#[test]
fn a_log10_y_scale_formats_the_inverse_transformed_value() {
    // Review PR#141 finding 2: data-fvalue/{y} templates must show the
    // human-readable value (y = 100), not the transformed-space number
    // the built data actually holds (log10(100) = 2).
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[10.0, 100.0, 1000.0])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_log10()
        .svg_formatted_values(true)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("data-fvalue=\"100\""), "{svg}");
    assert!(svg.contains("data-fvalue=\"10\""), "{svg}");
    assert!(svg.contains("data-fvalue=\"1000\""), "{svg}");
    assert!(!svg.contains("data-fvalue=\"1\""), "{svg}");
    assert!(!svg.contains("data-fvalue=\"2\""), "{svg}");
    assert!(!svg.contains("data-fvalue=\"3\""), "{svg}");
}

#[test]
fn a_log10_y_tooltip_template_also_shows_the_inverse_transformed_value() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[100.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_log10()
        .tooltip("y = {y}")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<title>y = 100</title>"), "{svg}");
}

#[test]
fn a_date_x_with_date_labels_gives_data_fx_in_that_format() {
    // Review PR#141 finding 5: data-fx was suppressed entirely when
    // with_date_labels was set, because it ignored the format and fell
    // back to ISO, which then equalled the plain data-x and got dropped
    // as redundant.
    let data = frame(vec![("x", floats(&[OCT_12_2026])), ("y", floats(&[1.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_datetime(ScaleDateTime::new().with_date_labels("%d.%m.%Y"))
        .svg_formatted_values(true)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("data-fx=\"12.10.2026\""), "{svg}");
}

#[test]
fn a_vline_at_a_datetime_x_with_date_labels_shows_that_format() {
    let base = || {
        GGPlot::new(frame(vec![("x", floats(&[1.0])), ("y", floats(&[1.0]))]))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_x_datetime(ScaleDateTime::new().with_date_labels("%d.%m.%Y"))
    };
    let svg = base()
        .geom_vline(OCT_12_2026)
        .svg_formatted_values(true)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("data-x=\"12.10.2026\""), "{svg}");
    assert!(svg.contains("x = 12.10.2026"), "tooltip: {svg}");
}

#[test]
fn an_x_tooltip_template_on_a_datetime_scale_with_date_labels_uses_that_format() {
    let data = frame(vec![("x", floats(&[OCT_12_2026])), ("y", floats(&[1.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_datetime(ScaleDateTime::new().with_date_labels("%d.%m.%Y"))
        .tooltip("{x}")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<title>12.10.2026</title>"), "{svg}");
}

#[test]
fn plotgrid_cells_honour_their_own_svg_formatted_values() {
    // Review PR#141 finding 6 (LOW): PlotGrid never carried the flag
    // through to a cell's own backend, so data-fvalue/data-fx never
    // appeared inside a grid even when the cell's GGPlot asked for them.
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[1234.5]))]);
    let on = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .svg_formatted_values(true);
    let off = GGPlot::new(data).aes(Aes::new().x("x").y("y")).geom_point();
    let svg = (on | off).render_svg_native_with_size(600, 300).unwrap();
    assert!(svg.contains("data-fvalue="), "{svg}");
    // Only the cell that asked for it gets the attribute: exactly one
    // occurrence, not one per cell.
    assert_eq!(svg.matches("data-fvalue=").count(), 1, "{svg}");
}
