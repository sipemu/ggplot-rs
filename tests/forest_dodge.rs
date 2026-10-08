//! U8 — multi-model coefficient forests: pointrange / errorbar / linerange /
//! point honour `position_dodge(width)` and `position_dodge2(reverse)` on a
//! discrete term axis, including under `coord_flip`.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::position::dodge::position_dodge;
use ggplot_rs::prelude::*;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

/// broom-style `terms` table: 3 terms × 3 models.
fn terms() -> Vec<(String, Vec<Value>)> {
    let t = ["wt", "hp", "cyl"];
    let m = ["ols", "ridge", "lasso"];
    let mut term = Vec::new();
    let mut model = Vec::new();
    let mut est = Vec::new();
    for (i, tt) in t.iter().enumerate() {
        for (j, mm) in m.iter().enumerate() {
            term.push(*tt);
            model.push(*mm);
            est.push(-1.0 + i as f64 + 0.1 * j as f64);
        }
    }
    vec![
        ("term".into(), strs(&term)),
        ("model_id".into(), strs(&model)),
        ("estimate".into(), floats(&est)),
        (
            "conf_low".into(),
            floats(&est.iter().map(|e| e - 0.5).collect::<Vec<_>>()),
        ),
        (
            "conf_high".into(),
            floats(&est.iter().map(|e| e + 0.5).collect::<Vec<_>>()),
        ),
    ]
}

fn forest() -> GGPlot {
    GGPlot::new(terms()).aes(
        Aes::new()
            .x("term")
            .y("estimate")
            .ymin("conf_low")
            .ymax("conf_high")
            .color("model_id"),
    )
}

/// `(cx, cy)` of the circle tagged with this term / model.
fn circle(svg: &str, term: &str, model: &str) -> (f64, f64) {
    let key = format!("data-x=\"{term}\" data-series=\"{model}\"");
    let tag = svg
        .split("<circle ")
        .map(|p| &p[..p.find('>').unwrap()])
        .find(|t| t.contains(&key))
        .unwrap_or_else(|| panic!("no circle for {term}/{model}"));
    let attr = |n: &str| -> f64 {
        let i = tag.find(&format!("{n}=\"")).unwrap() + n.len() + 2;
        tag[i..i + tag[i..].find('"').unwrap()].parse().unwrap()
    };
    (attr("cx"), attr("cy"))
}

fn panel(svg: &str) -> (f64, f64, f64, f64) {
    let s = svg.split("data-plot=\"").nth(1).unwrap();
    let v: Vec<f64> = s[..s.find('"').unwrap()]
        .split(' ')
        .map(|t| t.parse().unwrap())
        .collect();
    (v[0], v[1], v[2], v[3])
}

#[test]
fn pointrange_dodges_models_within_each_term() {
    let svg = forest()
        .geom_pointrange()
        .position(position_dodge(0.6))
        .render_svg_native_with_size(600, 400)
        .unwrap();
    let (px, _, pw, _) = panel(&svg);
    let band = pw / 3.0; // one category
    for (k, term) in ["wt", "hp", "cyl"].iter().enumerate() {
        let centre = px + (k as f64 + 0.5) * band;
        for (j, model) in ["ols", "ridge", "lasso"].iter().enumerate() {
            let (cx, _) = circle(&svg, term, model);
            // ggplot2: offsets (j − 1) · 0.6/3 category units.
            let want = centre + (j as f64 - 1.0) * 0.2 * band;
            assert!((cx - want).abs() < 0.02, "{term}/{model}: {cx} vs {want}");
        }
    }
    assert!(svg.contains(r#"data-x="hp" data-series="ridge" data-value="0.1""#));
}

#[test]
fn dodged_forest_under_coord_flip() {
    let svg = forest()
        .geom_hline(0.0)
        .geom_pointrange()
        .position(position_dodge(0.6))
        .coord_flip()
        .render_svg_native_with_size(600, 400)
        .unwrap();
    let (_, py, _, ph) = panel(&svg);
    let band = ph / 3.0;
    let (_, y_ols) = circle(&svg, "wt", "ols");
    let (_, y_lasso) = circle(&svg, "wt", "lasso");
    assert!(((y_ols - y_lasso).abs() - 0.4 * band).abs() < 0.02);
    // Models of one term share the term's band.
    assert!(y_ols > py && y_ols < py + ph);
    // The interval lines are horizontal under the flip (not collapsed).
    let line = svg
        .split("<polyline points=\"")
        .find(|p| p[..p.find('>').unwrap()].contains("data-x=\"wt\" data-series=\"ols\""))
        .unwrap();
    let pts: Vec<(f64, f64)> = line[..line.find('"').unwrap()]
        .split(' ')
        .map(|xy| {
            let (a, b) = xy.split_once(',').unwrap();
            (a.parse().unwrap(), b.parse().unwrap())
        })
        .collect();
    assert!((pts[0].1 - pts[1].1).abs() < 0.01, "horizontal");
    assert!((pts[0].0 - pts[1].0).abs() > 10.0, "non-degenerate");
}

#[test]
fn errorbar_linerange_and_points_dodge_consistently() {
    let svg = forest()
        .geom_errorbar()
        .position(position_dodge(0.6))
        .geom_linerange()
        .position(position_dodge(0.6))
        .geom_point()
        .position(position_dodge(0.6))
        .render_svg_native_with_size(600, 400)
        .unwrap();
    let (cx, _) = circle(&svg, "cyl", "lasso");
    for p in svg.split("<polyline points=\"").skip(1) {
        let tag = &p[..p.find('>').unwrap()];
        if !tag.contains("data-x=\"cyl\" data-series=\"lasso\"") {
            continue;
        }
        let xs: Vec<f64> = p[..p.find('"').unwrap()]
            .split(' ')
            .map(|xy| xy.split_once(',').unwrap().0.parse().unwrap())
            .collect();
        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        assert!((mean - cx).abs() < 0.02, "interval centred on the point");
    }
}

#[test]
fn dodge2_reverse_flips_the_model_order() {
    let normal = forest()
        .geom_pointrange()
        .position(PositionDodge2::new(0.0).with_width(0.6))
        .render_svg_native_with_size(600, 400)
        .unwrap();
    let reversed = forest()
        .geom_pointrange()
        .position(PositionDodge2::new(0.0).with_width(0.6).with_reverse(true))
        .render_svg_native_with_size(600, 400)
        .unwrap();
    let (a, _) = circle(&normal, "wt", "ols");
    let (b, _) = circle(&reversed, "wt", "lasso");
    assert!((a - b).abs() < 0.02, "lasso takes ols' slot when reversed");
    let (c, _) = circle(&reversed, "wt", "ols");
    assert!(c > a);
}

#[test]
fn a_model_missing_a_term_is_centred() {
    let mut data = terms();
    // Drop the ridge and lasso rows of "cyl" (rows 7, 8).
    for (_, col) in data.iter_mut() {
        col.truncate(7);
    }
    let svg = GGPlot::new(data)
        .aes(
            Aes::new()
                .x("term")
                .y("estimate")
                .ymin("conf_low")
                .ymax("conf_high")
                .color("model_id"),
        )
        .geom_pointrange()
        .position(position_dodge(0.6))
        .render_svg_native_with_size(600, 400)
        .unwrap();
    let (px, _, pw, _) = panel(&svg);
    let (cx, _) = circle(&svg, "cyl", "ols");
    assert!((cx - (px + 2.5 * pw / 3.0)).abs() < 0.02);
}

#[test]
fn continuous_x_dodge_moves_x() {
    let data: Vec<(String, Vec<Value>)> = vec![
        ("x".into(), floats(&[1.0, 1.0, 2.0, 2.0])),
        ("y".into(), floats(&[1.0, 2.0, 3.0, 4.0])),
        ("g".into(), strs(&["a", "b", "a", "b"])),
    ];
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .position(PositionDodge::new(0.4))
        .try_build()
        .unwrap();
    let mut x: Vec<f64> = built.layers[0]
        .data
        .column("x")
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    x.sort_by(f64::total_cmp);
    for (got, want) in x.iter().zip([0.9, 1.1, 1.9, 2.1]) {
        assert!((got - want).abs() < 1e-12);
    }
}
