# Move the WASM boundary types from the tsify ABI attributes to tsify::Ts

Status: accepted
Date: 2026-09-07

`tsify` 0.5.8 deprecated `#[tsify(into_wasm_abi)]` and `#[tsify(from_wasm_abi)]`, citing memory leaks ([tsify#65](https://github.com/madonoharu/tsify/issues/65)). Six derives in this crate used them, so `cargo clippy --all-targets --all-features -- -D warnings` failed with six errors.

The deprecation note is attached once per derive whenever either attribute is present, which hides the fact that the two attributes fail differently. `from_wasm_abi` leaks: its generated `from_abi` calls `wasm_bindgen::throw_str` when deserialization fails, and `throw_str` does not run destructors. JS catches the exception, the instance keeps running, and a caller can repeat it until the instance dies with "memory access out of bounds". `into_wasm_abi` does not leak: its failure path is a plain `panic!`, and `rustc --print cfg --target wasm32-unknown-unknown` reports `panic="abort"`, so a failure kills the instance rather than accumulating.

One site actually leaked: `TabInput` as the `generateArrangements` parameter. It is reachable by any JS caller passing a malformed object, and the whole public surface of this crate is the WASM boundary, so it was worth fixing rather than silencing.

Three different fixes, because the six sites are not one problem.

The two dead attributes are deleted. `ParseError` and `UnplayablePitch` appear only nested inside `TabError` variants, where serde handles them as part of the parent value. Removing their attributes produced a byte-identical `.d.ts` and a byte-identical `.wasm` (975,862 bytes both), which confirms the generated impls were never instantiated. Both keep the bare `Tsify` derive, which is what emits the TypeScript declaration.

`TabError` gets a hand-rolled `From<TabError> for JsValue`. wasm-bindgen's `Result<T, E>` needs only `E: Into<JsValue>` for the error type, not `WasmDescribe`, so an eight-line impl calling `Tsify::into_js` replaces the macro-generated conversion and every `?` in the codebase keeps working untouched. This avoids `Result<_, Ts<TabError>>`, which would have turned every error return into a fallible serialization step. Serializing a `TabError` cannot fail, because every variant is plain data with string keys, and the conversion degrades to the `Display` string rather than aborting the instance, which is a better failure mode than the `panic!` it replaces.

`TabInput`, `NormalizedBeat`, and `TuningName` cross the ABI as values, so they need the full `IntoWasmAbi` or `FromWasmAbi` machinery that `Ts<T>` provides. Their shims live in `src/wasm.rs` so the pure Rust core and the JS binding layer are two visible things rather than interleaved ones.

## Why the binding layer cannot be removed

`Ts<T>` is `#[repr(transparent)]` over a JS handle, and constructing one runs `serde_wasm_bindgen` to allocate a real `JsValue`, which needs a live JS runtime. On the host target, wasm-bindgen's `JsValue` operations are non-wasm stubs, and a host test calling `TuningName::DropD.into_ts()` aborts the process with SIGABRT.

So `Ts<T>` cannot appear in a native Rust signature. If `generate_arrangements` itself took `Ts<TabInput>`, nothing outside wasm could construct the argument: not the 268 host tests, not the criterion benches (host-only by design, per the `cfg(not(target_arch = "wasm32"))` scoping in `Cargo.toml`), and not any crates.io consumer of the `rlib`. This is a target constraint, not an API-compatibility one. The only way to avoid a binding layer would be to become wasm-only, which would mean dropping the `rlib`, moving all 268 tests to `wasm-bindgen-test`, and deleting the criterion and proptest suites.

The binding layer is not new indirection either. `#[tsify(from_wasm_abi)]` already generated a `FromWasmAbi` impl that did the deserialize inside the ABI boundary, and that generated impl is what leaked, because the only way to report failure from inside `from_abi` is `throw_str`. Moving to `Ts` takes the same conversion step out of a generated impl and puts it in a function where failure is a `return`.

## Consequences

- Every TypeScript signature is unchanged. `Ts<T>` describes through `T::JsType::describe()`, the same path the deprecated attributes used, so `generateArrangements(tab_input: TabInput): ArrangementSet`, `getTuningNames(): TuningName[]`, and `readonly normalizedInput: NormalizedBeat[]` all survive. The `tests/snapshots/wasm.d.ts` gate confirmed it.
- `TabError` gains one additive union member, `{ kind: "inputMalformed"; message: string }`. The enum is `#[non_exhaustive]` and its docs already instruct JS consumers to keep a `default` arm in any `switch (err.kind)`.
- Passing a malformed object to `generateArrangements` returns a typed error instead of throwing a raw string, and no longer leaks on each rejected call.
- The `.wasm` grows from 975,862 to 979,977 bytes, about 0.4%. `Ts` carries real conversion code where the old path was inlined into the ABI shim.
- The exports move off the native functions and onto shims, so consumer-facing doc comments have to live on the shims. The native functions keep their rustdoc for docs.rs. Some duplication between the two audiences is unavoidable.
- `ArrangementSet` now has a plain `impl` block alongside its `#[wasm_bindgen]` one, because a method returning `Vec<NormalizedBeat>` cannot sit in an exported block once `NormalizedBeat` loses `WasmDescribe`.
- `src/lib.rs` gains `#[doc(hidden)] pub mod wasm;`. It is public only so `tests/wasm_boundary.rs` can call the shims, and is not part of the stable Rust API.
- [ADR-0010](0010-tsify-bindings.md) still holds. This changes how boundary types reach the ABI, not which crate derives the TypeScript.
