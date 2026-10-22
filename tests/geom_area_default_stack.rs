//! Review #167 finding 5 on sipemu/ggplot-rs#171: `geom_area()`'s default
//! position must be `position_stack()`, the same as `geom_col()`'s/
//! `geom_bar()`'s (#169) — ggplot2 parity. Landed in the same 0.24.0
//! minor as #169 so every default-position parity break lands in one
//! release, rather than as a follow-up issue later.

use ggplot_rs::position::identity::PositionIdentity;
use ggplot_rs::position::stack::PositionStack;
use ggplot_rs::prelude::*;

fn two_series_data() -> Vec<(String, Vec<Value>)> {
    let x: Vec<Value> = [1.0, 2.0, 3.0, 1.0, 2.0, 3.0]
        .iter()
        .map(|v| Value::Float(*v))
        .collect();
    let grp: Vec<Value> = ["a", "a", "a", "b", "b", "b"]
        .iter()
        .map(|s| Value::Str(s.to_string()))
        .collect();
    let y: Vec<Value> = [1.0, 2.0, 1.0, 1.0, 2.0, 1.0]
        .iter()
        .map(|v| Value::Float(*v))
        .collect();
    vec![("x".into(), x), ("grp".into(), grp), ("y".into(), y)]
}

fn base() -> GGPlot {
    GGPlot::new(two_series_data())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_area()
}

fn render(plot: GGPlot) -> String {
    plot.render_svg_native_with_size(320, 400)
        .expect("native svg render")
}

/// `geom_area()` with no explicit `.position(...)` must render exactly
/// like an explicit `PositionStack` — not like `PositionIdentity`, which
/// drew every series' band on top of the others from `y = 0`.
#[test]
fn geom_area_default_position_matches_explicit_stack() {
    let default_svg = render(base());
    let stack_svg = render(base().position(PositionStack));
    assert_eq!(
        default_svg, stack_svg,
        "geom_area()'s default position must be position_stack() (ggplot2 parity)"
    );
}

/// Before this fix, `geom_area()`'s default rendered identically to an
/// explicit `PositionIdentity` — this must no longer hold once the data
/// has more than one series at a shared x.
#[test]
fn geom_area_default_position_no_longer_matches_identity() {
    let default_svg = render(base());
    let identity_svg = render(base().position(PositionIdentity));
    assert_ne!(
        default_svg, identity_svg,
        "geom_area()'s default must no longer be PositionIdentity"
    );
}
