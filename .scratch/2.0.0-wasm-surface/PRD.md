# 2.0.0 WASM Surface Improvements

Status: ready-for-agent
Date: 2026-05-19

## Goal

Land a coherent set of breaking changes at the WASM boundary in one release, motivated by the [noahbaculi.com guitar-tab-generator demo](https://noahbaculi.com/projects/guitar-tab-generator). The major version bump is already in flight for the `Composition` to `RenderedTab` rename; this work extends that partial rename into a complete API redesign before 2.0.0 ships.

## Motivation

The demo currently treats every WASM call as `any`, switches on the string sentinels `"REST"` and `"MEASURE_BREAK"` to render the input echo, regex-parses error messages to highlight bad input lines, and re-runs pathfinding on every width slider change. Each of those is fixable individually, but doing them in separate breaking releases would force the demo through multiple migrations. Bundling resolves them in one pass.

## Scope

### In scope

- Type and entry-point renames (the in-flight `Composition` rename plus `CompositionInput` to `TabInput`, `wasm_create_guitar_compositions` to `generate_arrangements`, raw text field `pitches` to `input`).
- Adopt `tsify-next` to generate TypeScript bindings for boundary types.
- Replace string sentinels in the normalized input with tagged `NormalizedBeat` variants.
- Move `normalized_input` from per-`RenderedTab` to a top-level `ArrangementSet`, eliminating the wire duplication that came with `num_arrangements > 1`.
- Split parse-and-arrange from render. The first call returns an `ArrangementSet` (a `#[wasm_bindgen]` opaque handle). Render parameters move onto a method on the handle, so width or playback changes do not re-run pathfinding.
- Structured `TabError` enum with `ParseError { line, text }` per unparseable substring.
- New `TabInput.max_fret_span_filter: Option<u8>` (long-standing README TODO).
- Boundary validation of `num_arrangements` (1 to 20 inclusive), surfaced as `TabError::InvalidInput`.

### Out of scope

- Internal restructure of `Guitar` and `Arrangement` error types. Anyhow strings stay; the boundary wraps them as `TabError::Guitar { message }` and `TabError::Arrangement { message }`. They can become structured in a non-breaking 2.x release without affecting the typed `Parse` path.
- Algorithm or performance work in `parser.rs` or `arrangement.rs`.
- Backwards-compatibility shims. 2.0.0 is breaking; the demo updates in one pass.

## Architecture decisions

The defining decision is the opaque-handle pattern for `ArrangementSet`, captured in [ADR-0001](../../docs/adr/0001-arrangement-set-opaque-handle.md). Arrangements stay Rust-side; the JS demo holds a handle and calls methods on it. This is the deliberate exception to the codebase's otherwise serde-wasm-bindgen boundary; every other boundary type (`TabInput`, `NormalizedBeat`, `TabError`, `TuningName`) keeps the serde-via-tsify path.

The full set of resolved domain terms (`TabInput`, `ArrangementSet`, `Arrangement`, `RenderedTab`, `Normalized input`, `NormalizedBeat`) is in [CONTEXT.md](../../CONTEXT.md). The grilling session that resolved them recorded the alternatives considered and rejected.

## Type surface

### Boundary types

```rust
// src/lib.rs

#[derive(Debug, Clone, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct TabInput {
    pub input: String,
    pub tuning_name: String,
    pub guitar_num_frets: u8,
    pub guitar_capo: u8,
    pub num_arrangements: u8,                  // 1 to 20 inclusive, validated at boundary
    pub max_fret_span_filter: Option<u8>,      // new in 2.0.0
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NormalizedBeat {
    Playable { pitches: Vec<String> },
    Rest,
    MeasureBreak,
}

#[wasm_bindgen]
pub struct ArrangementSet { /* opaque, no public fields */ }
```

Render parameters (`width`, `padding`, `playback`) are deliberately not on `TabInput`. They live on `ArrangementSet::render(...)` so a width change does not trigger pathfinding.

### Error types

```rust
// src/error.rs (new module)

#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct ParseError {
    pub line: u32,      // 1-indexed
    pub text: String,   // the offending substring
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Input '{}' on line {} could not be parsed into a pitch.",
               self.text, self.line)
    }
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TabError {
    Parse { errors: Vec<ParseError> },
    Guitar { message: String },
    Arrangement { message: String },
    InvalidInput { field: String, message: String },
}
```

`ParseError` is defined once and used in both `parser.rs` (the internal error path) and `lib.rs` (the boundary type). The parser imports it from `crate::error::ParseError`; the Tsify derive lives next to the type definition, so the parser module never imports `tsify_next` itself. The boundary conversion is `Arc::try_unwrap` (succeeds on memoize cache miss, clones the `Vec` on cache hit). No per-element rewrapping.

`Display` on `ParseError` reproduces the existing message format, so any test or caller that still wants the legacy joined string can produce it via `format!("{}", parse_error)`.

## Entry points

```rust
#[wasm_bindgen(js_name = "generateArrangements")]
pub fn generate_arrangements(input: TabInput) -> Result<ArrangementSet, TabError> {
    // 1. Validate num_arrangements (1..=20)  -> TabError::InvalidInput
    // 2. parse_lines(input)                  -> TabError::Parse { errors }
    // 3. parse_tuning + build Guitar         -> TabError::Guitar
    // 4. create_arrangements with filter     -> TabError::Arrangement
    // 5. Build NormalizedBeat sequence       -> stored on the handle
    // 6. Construct ArrangementSet            -> owns guitar, arrangements, normalized_input
}

#[wasm_bindgen(js_name = "getTuningNames")]
pub fn get_tuning_names() -> Vec<TuningName> { /* tsify-typed enum */ }

#[wasm_bindgen]
impl ArrangementSet {
    #[wasm_bindgen(getter)]
    pub fn len(&self) -> usize { ... }

    #[wasm_bindgen(getter, js_name = "normalizedInput")]
    pub fn normalized_input(&self) -> Vec<NormalizedBeat> { ... }

    #[wasm_bindgen(js_name = "maxFretSpan")]
    pub fn max_fret_span(&self, index: usize) -> Result<u8, TabError> { ... }

    pub fn difficulty(&self, index: usize) -> Result<i32, TabError> { ... }

    pub fn render(
        &self,
        index: usize,
        width: u16,
        padding: u8,
        playback: Option<u16>,
    ) -> Result<String, TabError> { ... }
}
```

Out-of-bounds index on any indexed method returns `TabError::InvalidInput { field: "index", ... }`. The signed Rust convention of panicking on bounds violations would surface in JS as a generic WebAssembly trap; a structured throw is more useful to the demo.

`TuningName` gains `#[derive(Tsify, Serialize)]` so JS receives `getTuningNames() -> TuningName[]` typed as a string-literal union, not `string[]`.

### JS demo usage

```ts
const set = generateArrangements({
  input: "E2\nA2\n\nG3",
  tuningName: "standard",
  guitarNumFrets: 20,
  guitarCapo: 0,
  numArrangements: 5,
  maxFretSpanFilter: null,
});

const beats = set.normalizedInput;

for (let i = 0; i < set.len; i++) {
  const tab = set.render(i, width, padding, playbackIndex);
  const span = set.maxFretSpan(i);
  const difficulty = set.difficulty(i);
  // display
}

// Width slider change: only re-call set.render(), no re-pathfinding.
```

## Internal restructure

```
src/
├── lib.rs           TabInput, ArrangementSet, generate_arrangements
├── error.rs         NEW: ParseError, TabError
├── parser.rs        parse_lines (typed errors), TuningName (Tsify), get_tuning_names
├── arrangement.rs   Arrangement, create_arrangements (+ optional max_fret_span_filter)
├── guitar.rs        Guitar, PitchFingering (unchanged)
├── renderer.rs      render_tab (unchanged)
├── pitch.rs         Pitch (unchanged)
└── string_number.rs StringNumber (unchanged)
```

### Cargo.toml

- `version = "2.0.0"` (already staged in the working tree).
- Add `tsify-next = { version = "0.5", features = ["js"] }`.

### src/lib.rs

Drop: `CompositionInput`, the `Composition` / `RenderedTab` struct, `wasm_create_guitar_compositions`, `wrapper_create_arrangements`, the `memoized_original_*` re-exports.

Add: `TabInput`, `NormalizedBeat`, `ArrangementSet`, `generate_arrangements`. The handle struct holds (all fields private by design; JS reaches them through the `#[wasm_bindgen]` getters and methods):

```rust
pub struct ArrangementSet {
    arrangements: Vec<arrangement::Arrangement>,
    guitar: Guitar,
    normalized_input: Vec<NormalizedBeat>,
}
```

No `Rc`. The normalized input is owned by a single handle, not shared across many `RenderedTab` clones.

### src/parser.rs

- `parse_pitch` returns `Result<Line<Vec<Pitch>>, Vec<ParseError>>`. The `consecutive_slices` loop pushes structured errors instead of formatted strings.
- `parse_line` and `parse_lines` ripple the type: `Result<_, ParseError>` and `Result<_, Arc<Vec<ParseError>>>` respectively.
- `TuningName` gains `Tsify` and `Serialize`. Keeps `#[non_exhaustive]`.
- `get_tuning_names` returns `Vec<TuningName>` directly. Its `js_name` is `getTuningNames`.
- The `#[memoize(Capacity: 10)]` on `parse_lines` is preserved. It is the demo's keystroke-rate cache. The new error type `Arc<Vec<ParseError>>` already satisfies `Clone` so memoize continues to work without further changes.

### src/arrangement.rs

`create_arrangements` gains an `Option<u8>` filter parameter:

```rust
pub fn create_arrangements(
    guitar: Guitar,
    input_lines: Vec<Line<BeatVec<Pitch>>>,
    num_arrangements: u8,
    max_fret_span_filter: Option<u8>,
) -> Result<Vec<Arrangement>, ...>
```

The implementation is a post-pathfinding `retain` on yen's `num_arrangements` results, with a "return what we have" fallback when filtering would drop the count below `num_arrangements`. The first release does not retry yen with a larger `N` to backfill; if the filter is too strict the caller sees fewer arrangements and can relax it. Detailed implementation belongs to the next plan; this spec commits to the parameter shape and the post-filter semantics, not to the algorithm.

### src/renderer.rs, src/guitar.rs, src/pitch.rs, src/string_number.rs

Unchanged.

### Public Rust API

Kept: `generate_arrangements`, `ArrangementSet`, `TabInput`, `TabError`, `ParseError`, `NormalizedBeat`, plus the low-level types `parse_lines`, `parse_tuning`, `create_string_tuning_offset`, `create_arrangements`, `render_tab`, `Guitar`, `Pitch`, `StringNumber`, `TuningName`, `Arrangement`, `Line`, `BeatVec`, `PitchFingering`.

Dropped: `memoized_original_create_arrangements`, `memoized_original_parse_lines`, `CompositionInput`, `Composition`, `wrapper_create_arrangements`, `wasm_create_guitar_compositions`.

## Implementation order

Each step is a self-contained commit; build and tests stay green throughout.

0. **Pre-step.** Commit (or stash) the in-flight `Composition` to `RenderedTab` rename so 2.0.0 work starts from a clean tree. The `RenderedTab` struct goes away entirely in step 6, but committing the partial rename first keeps git history honest.
1. **Deps and scaffolding.** Add `tsify-next` to `Cargo.toml`. Create `src/error.rs` with `ParseError` and `TabError`. No callers yet.
2. **Internal parser refactor.** `parse_pitch` returns `Result<Line, Vec<ParseError>>`; `parse_line` and `parse_lines` ripple. Parser tests update to assert on the structured `Vec`.
3. **Boundary types land.** Add `TabInput`, `NormalizedBeat`, `ArrangementSet`, `generate_arrangements` alongside the existing `wasm_create_guitar_compositions`. Both APIs coexist; new paths share parser and arrangement internals.
4. **`max_fret_span_filter`.** Add the parameter to `create_arrangements` (defaulting to `None` everywhere). Proptests pass `None`.
5. **Tests migrate.** Rewrite `test_wrapper_create_arrangements` to exercise the new API. Add the new tests listed below.
6. **Drop the old surface.** Delete `CompositionInput`, the `Composition` / `RenderedTab` struct, `wasm_create_guitar_compositions`, `wrapper_create_arrangements`, `memoized_original_*` re-exports, and the related tests.
7. **Docs sweep.** Update `types.md` to the new pipeline shape; add a 2.0.0 CHANGELOG entry; drop the `max_fret_span` bullet from the README's "Future Improvements" list.

## Tests

### Rewritten

- `test_wrapper_create_arrangements` becomes `test_generate_arrangements_and_render`. Same input fixtures; assertions run via `set.len`, `set.normalized_input`, `set.render(0, ...)`, `set.max_fret_span(0)`. The exact ASCII output of `set.render(...)` stays byte-identical to the legacy expectation.
- `test_parse_lines::reports_line_and_content_for_unparseable_input` asserts on `Vec<ParseError>` with `[ParseError { line: 1, text: "xyz" }, ParseError { line: 4, text: "BB.2" }]` instead of the joined string.

### New

- `set.render(out_of_range_index, ...)` returns `TabError::InvalidInput { field: "index", ... }`.
- `set.max_fret_span(out_of_range)` and `set.difficulty(out_of_range)` likewise.
- `num_arrangements = 0` and `= 21` at the boundary return `TabError::InvalidInput { field: "numArrangements", ... }`. The field name is the camelCase form the JS side sees.
- `max_fret_span_filter = Some(0)` on a wide-stretch input returns fewer than `num_arrangements` arrangements without error.
- Two successive `set.render(0, w1, ...)` and `set.render(0, w2, ...)` produce two correctly-widthed outputs, confirming render is stateless and cheap.
- A smoke test for `getTuningNames()` returning a non-empty `Vec<TuningName>`.

### Preserved

- All proptests in `src/arrangement.rs`. Call sites pass `None` for the new filter parameter to preserve existing behavior coverage.
- All unit tests in `parser.rs`, `guitar.rs`, `renderer.rs`, `pitch.rs`, `string_number.rs`.

## Migration notes for downstream consumers

The noahbaculi.com demo is the only known consumer. CHANGELOG entry will list:

- `wasm_create_guitar_compositions(input)` becomes `generateArrangements(input)`.
- `get_tuning_names()` becomes `getTuningNames()` and now returns the typed `TuningName[]` union.
- Input field `pitches` becomes `input`.
- Output shape changes from `Composition[]` to a single `ArrangementSet` handle. Access pattern moves from `result.map(...)` to `for (let i = 0; i < set.len; i++) set.render(i, ...)`.
- `normalized_input` sentinels (`"REST"` and `"MEASURE_BREAK"` strings) become tagged variants (`{ kind: "rest" }`, `{ kind: "measureBreak" }`) at `set.normalizedInput`.
- Thrown errors are now typed `TabError` objects with `.kind`. `Parse` errors carry `.errors[].line` for inline editor highlights.

The demo must call `set.free()` (or rely on `FinalizationRegistry` in modern runtimes) to release the handle when no longer needed.

## Open questions

None at spec time. The grilling session resolved all design-level branches; remaining decisions belong to the implementation plan (post-filter algorithm details, exact wasm-bindgen attribute choices, tsify-next minor version pin).
