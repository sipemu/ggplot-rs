//! wasm-bindgen tests of the npm entry points across the JS boundary:
//! `wasm-pack test --node crates/ggplot-rs-wasm`.
#![cfg(target_arch = "wasm32")]

use ggplot_rs_wasm::{render, render_with_warnings, PlotSpecInput};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;

fn input(v: JsValue) -> PlotSpecInput {
    v.unchecked_into()
}

const SPEC: &str = r#"{"data": {"x": [1, 2, 3], "y": [3, 1, 2]},
    "aes": {"x": "x", "y": "y"}, "layers": [{"geom": "point"}]}"#;

#[wasm_bindgen_test]
fn renders_from_json_text() {
    let svg = render(input(JsValue::from_str(SPEC))).unwrap();
    assert!(svg.starts_with("<svg"));
}

#[wasm_bindgen_test]
fn renders_from_a_js_object() {
    let obj = js_sys::JSON::parse(SPEC).unwrap();
    let svg = render(input(obj)).unwrap();
    assert_eq!(svg.matches("<circle").count(), 3);
}

#[wasm_bindgen_test]
fn errors_are_js_errors() {
    let bad = r#"{"data": {"x": [1]}, "aes": {"x": "nope"}, "layers": [{"geom": "point"}]}"#;
    let err = render(input(JsValue::from_str(bad))).unwrap_err();
    let err: js_sys::Error = err.dyn_into().expect("an Error object");
    assert!(String::from(err.message()).contains("column 'nope' not found"));
}

#[wasm_bindgen_test]
fn warnings_come_back_with_the_svg() {
    let r = render_with_warnings(input(JsValue::from_str(SPEC))).unwrap();
    assert!(r.svg().starts_with("<svg"));
    assert!(r.warnings().is_empty());
}
