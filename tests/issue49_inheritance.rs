//! Issue #49 — root `text` element cascades to child text elements.
use ggplot_rs::prelude::*;

fn render_with_text_family(fam: Option<&str>) -> String {
    let d = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ];
    let mut t = theme_minimal();
    if let Some(f) = fam {
        t.text.family = f.to_string(); // set ONLY the root
    }
    GGPlot::new(d)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme(t)
        .title("T")
        .render_svg()
        .unwrap()
}

#[test]
fn root_text_family_cascades_to_children() {
    // Setting only theme.text.family restyles every text element.
    let svg = render_with_text_family(Some("monospace"));
    assert!(
        svg.contains("monospace"),
        "child text should inherit the root family"
    );
}

#[test]
fn no_cascade_when_root_unchanged() {
    let svg = render_with_text_family(None);
    assert!(
        !svg.contains("monospace"),
        "default theme should not use monospace"
    );
}
