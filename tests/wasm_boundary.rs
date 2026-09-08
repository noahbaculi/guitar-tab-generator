//! WASM-target boundary tests.
//!
//! The host `cargo test` suite exercises the public API on the native target. These tests
//! compile the boundary to `wasm32` and run it under `wasm-pack test --node`, covering the
//! `#[wasm_bindgen]` ABI path (input deserialization, `ArrangementSet` handle construction,
//! and drop) that the host tests cannot reach. The thrown-error wire shape is guarded
//! separately by the `tests/snapshots/wasm.d.ts` surface diff. These tests confirm the code
//! executes correctly under the wasm target.
//!
//! The malformed-input case is covered here too, since a host test cannot build a `Ts<TabInput>`
//! without aborting.
//!
//! Empty on non-wasm targets so the host `cargo test` lane skips it.
#![cfg(target_arch = "wasm32")]

use guitar_tab_generator::wasm::{Ts, generate_arrangements_js};
use guitar_tab_generator::{TabError, TabInput, generate_arrangements};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn generate_and_render_under_wasm() {
    let set = generate_arrangements(TabInput::new("E2\nA2\nD3", "standard", 18, 0, 1))
        .expect("valid input must produce a set");
    assert_eq!(set.len(), 1);
    let tab = set.render(0, 30, 2, None).expect("render must succeed");
    assert!(!tab.is_empty(), "rendered tab must not be empty");
}

#[wasm_bindgen_test]
fn error_path_surfaces_typed_variant_under_wasm() {
    // A1 is below every string's range, so the boundary reports UnplayablePitches.
    let err = generate_arrangements(TabInput::new("A1", "standard", 18, 0, 1)).unwrap_err();
    assert!(
        matches!(err, TabError::UnplayablePitches { .. }),
        "got {err:?}"
    );
}

#[wasm_bindgen_test]
fn malformed_input_returns_typed_variant_under_wasm() {
    // A bare string is not a `TabInput`, so the shim must return `InputMalformed`
    // rather than throwing.
    let bad = Ts::new_unchecked(JsValue::from_str("not a TabInput"));
    let err = generate_arrangements_js(bad).unwrap_err();
    assert!(
        matches!(err, TabError::InputMalformed { .. }),
        "got {err:?}"
    );
}

#[wasm_bindgen_test]
fn handle_drops_cleanly_under_wasm() {
    let set = generate_arrangements(TabInput::new("E2\nA2\nD3", "standard", 18, 0, 2)).unwrap();
    assert_eq!(set.len(), 2);
    // Dropping the handle exercises the wasm-bindgen free path. It must not trap.
    drop(set);
}
