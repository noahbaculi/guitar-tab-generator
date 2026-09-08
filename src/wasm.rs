//! WASM binding layer.
//!
//! Each `#[wasm_bindgen]` shim here wraps an unchanged native function from the crate root,
//! converting its boundary values through `tsify::Ts`.
//!
//! `Ts<T>` is `#[repr(transparent)]` over a JS handle, and building one allocates a real
//! `JsValue`, which needs a live JS runtime. On the host target wasm-bindgen's `JsValue`
//! operations are non-wasm stubs and a conversion aborts the process, so `Ts<T>` never
//! appears in a native Rust signature.

use crate::error::TabError;
use crate::parser::TuningName;
use crate::{ArrangementSet, TabInput};
// Re-exported so `tests/wasm_boundary.rs` can build a `Ts<TabInput>` without a dev-dependency
// on `tsify`. The module is `#[doc(hidden)]`, so this is not a public API commitment.
pub use tsify::Ts;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

/// Generates an `ArrangementSet` from a `TabInput`.
///
/// The WASM entry point. `generate_arrangements` is the equivalent native Rust function.
///
/// # Errors
///
/// Returns the typed [`TabError`] variant for each failure mode reachable from this entry point:
///
/// - Argument shape: [`TabError::InputMalformed`] when the supplied object cannot be read as a
///   `TabInput`. JavaScript is dynamically typed, so this is reachable with any argument.
/// - Input-shape validation: [`TabError::NumArrangementsOutOfRange`], [`TabError::TuningNameUnknown`],
///   [`TabError::NumFretsTooHigh`], [`TabError::CapoTooHigh`], [`TabError::CapoExceedsFrets`].
/// - Parser: [`TabError::Parse`] (carries `Vec<ParseError>` with line/text per unparseable substring),
///   [`TabError::InputTooManyLines`] (input exceeds the 65,535-line cap).
/// - Pathfinding: [`TabError::UnplayablePitches`] (one or more pitches reach no string),
///   [`TabError::NoArrangementsFound`] (every pitch reaches the guitar but no valid combination exists,
///   for example duplicate pitches in a single beat that the no-duplicate-strings constraint filters away).
///
/// [`TabError::OpenPitchOutOfRange`], [`TabError::StringNumberOutOfRange`], and
/// [`TabError::FretRangeExceedsPitchRange`] are members of the enum and live on the [`crate::Guitar::new`] path
/// this function calls, but no `TabInput` reachable today can trip them: the preset tunings and fixed
/// 1..=6 string numbering keep every open-string pitch and fret range well inside the supported `Pitch`
/// range. They fire only when constructed directly through the lower-level Rust API ([`crate::Guitar::new`],
/// [`crate::create_string_tuning`]) with out-of-range inputs, such as a custom tuning (deferred to a later
/// release).
///
/// # Validation order
///
/// Argument-shape errors come first, because nothing else can be read until the object
/// deserializes. Input-shape errors (currently `numArrangements` range) are then reported before
/// `parse_lines` runs. The ordering is deliberate: shape checks are O(1) and unambiguous, while
/// parse errors depend on the full input. When both are present the shape error wins because the
/// parser's output would be discarded anyway.
///
/// Guitar-configuration errors (`TuningNameUnknown`, `NumFretsTooHigh`, `CapoTooHigh`,
/// `CapoExceedsFrets`) are checked before the normalized input is built, so an invalid guitar
/// config does not pay for the per-beat allocation. `parse_lines` still runs first, so a `Parse`
/// error outranks a guitar-config error.
///
/// # Performance
///
/// `tab_input.input` is cloned once per call because `parse_lines` is `#[memoize]`d on owned
/// `String`. Memoization makes a repeat call with the same input cheap, but the clone runs
/// on every call (including cache hits). Hot loops over `generateArrangements` should expect
/// one `String::clone` per invocation in addition to the boundary deserialization cost.
#[wasm_bindgen(js_name = "generateArrangements")]
pub fn generate_arrangements_js(tab_input: Ts<TabInput>) -> Result<ArrangementSet, TabError> {
    // The generated `FromWasmAbi` impl could only report failure with `throw_str`, which skips
    // destructors and leaks on every rejected call. Here failure is a return.
    let tab_input = tab_input.to_rust().map_err(|e| TabError::InputMalformed {
        message: e.to_string(),
    })?;
    crate::generate_arrangements(tab_input)
}

/// Returns the supported `TuningName` variants, typed for JS consumption via tsify.
#[wasm_bindgen(js_name = "getTuningNames")]
#[must_use]
pub fn get_tuning_names_js() -> Vec<Ts<TuningName>> {
    crate::get_tuning_names()
        .iter()
        // Unreachable: `TuningName` is a plain unit-variant enum. `panic=abort` on wasm32
        // kills the instance rather than leaking.
        .map(|t| t.into_ts().expect("BUG: TuningName should serialize"))
        .collect()
}
