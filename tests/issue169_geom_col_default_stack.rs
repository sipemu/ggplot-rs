//! #169: `geom_col()`'s default position must be `position_stack()`, the
//! same as `geom_bar()`'s (ggplot2 parity) — 0.23 left it defaulting to
//! `PositionIdentity`, which `tests/default_output_023.rs`'s
//! `mixed_sign_data` comment called out as "a pre-existing parity gap
//! from ggplot2... tracked in #169".

use ggplot_rs::position::identity::PositionIdentity;
use ggplot_rs::position::stack::PositionStack;
use ggplot_rs::prelude::*;

fn mixed_group_data() -> Vec<(String, Vec<Value>)> {
    vec![
        ("x".into(), vec![Value::Float(1.0), Value::Float(1.0)]),
        (
            "grp".into(),
            vec![Value::Str("a".into()), Value::Str("b".into())],
        ),
        ("y".into(), vec![Value::Float(3.0), Value::Float(5.0)]),
    ]
}

fn base() -> GGPlot {
    GGPlot::new(mixed_group_data())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_col()
}

fn render(plot: GGPlot) -> String {
    plot.render_svg_native_with_size(320, 400)
        .expect("native svg render")
}

/// `geom_col()` with no explicit `.position(...)` must render exactly
/// like an explicit `PositionStack` — not like `PositionIdentity`, which
/// drew every group's bar on top of the others starting from `y = 0`.
#[test]
fn geom_col_default_position_matches_explicit_stack() {
    let default_svg = render(base());
    let stack_svg = render(base().position(PositionStack));
    assert_eq!(
        default_svg, stack_svg,
        "geom_col()'s default position must be position_stack() (ggplot2 parity)"
    );
}

/// Before the #169 fix, `geom_col()`'s default rendered identically to
/// an explicit `PositionIdentity` — this must no longer hold once the
/// data has more than one group at a shared x.
#[test]
fn geom_col_default_position_no_longer_matches_identity() {
    let default_svg = render(base());
    let identity_svg = render(base().position(PositionIdentity));
    assert_ne!(
        default_svg, identity_svg,
        "geom_col()'s default must no longer be position_stack()'s old \
         PositionIdentity default"
    );
}
