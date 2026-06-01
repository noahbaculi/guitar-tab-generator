# Migration: 1.x to 2.0

Guitar Tab Generator 2.0.0 reworks the WASM surface for typed boundaries, structured errors, and cheap re-renders. This guide walks 1.x callers through every breaking change with before / after snippets.

For a flat list of changes, see [`CHANGELOG.md`](CHANGELOG.md). For the architectural decisions behind each change, see [`docs/adr/`](docs/adr/).

## Quick reference

### JS-facing renames

| 1.x | 2.0 |
|---|---|
| `wasm_create_guitar_compositions(input)` | `generateArrangements(input)` |
| `get_tuning_names()` | `getTuningNames()` |
| `Composition[]` (output) | `ArrangementSet` handle; `set.render(i, ...)` returns the rendered string |
| `pitches` (input field) | `input` |
| `num_arrangements`, `tuning_name`, `guitar_num_frets`, `guitar_capo` | camelCase: `numArrangements`, `tuningName`, `guitarNumFrets`, `guitarCapo` |
| `width`, `padding`, `playback` (on the input bundle) | moved to `ArrangementSet.render(i, width, padding, playback)` |
| `"REST"` / `"MEASURE_BREAK"` (normalized_input sentinels) | `{ kind: "rest" }` / `{ kind: "measureBreak" }` |

### Rust-facing renames

| 1.x | 2.0 |
|---|---|
| `wrapper_create_arrangements(...)` | `generate_arrangements(TabInput::new(...))?` |
| `CompositionInput` | `TabInput` |
| `Composition` / `RenderedTab` structs | dropped; rendered string comes from `set.render(i, ...)` |
| `create_arrangements(..., num: u8, ...)` | `create_arrangements(..., NumArrangements, ...)`; see [Direct Rust callers](#direct-rust-callers) |
| `&arrangement.lines` (field) | `arrangement.lines()` (getter) |
| `parse_tuning`, `create_string_tuning_offset`, `STD_6_STRING_TUNING_OPEN_PITCHES` re-exports | removed from crate root; non-preset tunings build a `BTreeMap<StringNumber, Pitch>` directly, or use `create_string_tuning(&[Pitch; N])` |
| `memoized_original_create_arrangements`, `memoized_original_parse_lines` re-exports | moved to `__bench_internals::*`, `#[doc(hidden)]`, not part of the stable 2.x API |

Rust-only callers: see [Direct Rust callers](#direct-rust-callers) for migration snippets.

### Optional fields

`TabInput.maxFretSpanFilter` is optional in the generated TypeScript declaration (`maxFretSpanFilter?: number`). Callers under TypeScript strict mode may omit the key; passing `undefined` continues to work. The Rust-side type is `Option<u8>` and accepts `None`.

### What you have to do

1. Replace the single `wasm_create_guitar_compositions` call with `generateArrangements` plus a loop over `set.render(i, ...)`.
2. Move `width`, `padding`, `playback` out of the input object and into the `render` call.
3. Switch error handling from string-matching to `switch (err.kind)`.
4. Call `set.free()` (or `using` on TS 5.2+) when you are done with the handle.
5. If you read `normalized_input`, switch from `["REST"]` / `["MEASURE_BREAK"]` string checks to `.kind`-based discrimination.

## Toolchain

2.0.0 builds on Rust edition 2024 and requires Rust 1.85 or newer
(`rust-version = "1.85"` in `Cargo.toml`). Run `rustup update` if your
toolchain is below 1.85.

## Generating arrangements

### TypeScript

Before (1.x):

```ts
const compositions = wasm_create_guitar_compositions({
  pitches: "E2\nA2\nD3",
  tuning_name: "standard",
  guitar_num_frets: 18,
  guitar_capo: 0,
  num_arrangements: 1,
  width: 30,
  padding: 1,
  playback: null,
});

for (const composition of compositions) {
  console.log(composition.tab);
}
```

After (2.0):

```ts
import init, { generateArrangements } from "./pkg/wasm_guitar_tab_generator/guitar_tab_generator.js";

await init();
const set = generateArrangements({
  input: "E2\nA2\nD3",
  tuningName: "standard",
  guitarNumFrets: 18,
  guitarCapo: 0,
  numArrangements: 1,
  maxFretSpanFilter: undefined,
});

for (let i = 0; i < set.len; i++) {
  console.log(set.render(i, 30, 1, null));
}
set.free();
```

Key shifts:

- `pitches` becomes `input`.
- Render parameters (`width`, `padding`, `playback`) move off the input object.
- The result is one opaque `ArrangementSet` handle, not a `Composition[]`.
- Iteration is by index (`for (let i = 0; i < set.len; i++)`) plus `set.render(i, ...)`.
- Call `set.free()` when done; see [Lifecycle](#lifecycle-js-only).

### Rust

Before (1.x):

```rust
use guitar_tab_generator::wrapper_create_arrangements;
let compositions = wrapper_create_arrangements(CompositionInput { /* ... */ })?;
```

After (2.0):

```rust
use guitar_tab_generator::{generate_arrangements, TabInput};

let set = generate_arrangements(TabInput::new("E2\nA2\nD3", "standard", 18, 0, 1))?;
let tab = set.render(0, 30, 1, None)?;
```

`CompositionInput` is renamed `TabInput`. Per-arrangement metadata lives on the set: `set.difficulty(i)?`, `set.max_fret_span(i)?`.

If you call `create_arrangements` directly, it now takes a `NumArrangements` newtype:

```rust
use guitar_tab_generator::{create_arrangements, NumArrangements};
let n = NumArrangements::try_new(1)?;
let arrangements = create_arrangements(guitar, lines, n, None)?;
```

See [ADR-0005](docs/adr/0005-num-arrangements-newtype.md).

## Render parameters

`width`, `padding`, and `playback` are no longer fields on the input object. They are arguments to `set.render(i, width, padding, playback)`. The reason: rendering at a different width or moving the playback cursor is cheap, but re-running pathfinding is not. Separating the two calls makes the cheap path cheap.

```ts
const set = generateArrangements({ /* ... */ });

const narrow = set.render(0, 40, 1, null);
const wide   = set.render(0, 100, 1, null);   // no re-pathfinding
const withCursor = set.render(0, 40, 1, 3);   // no re-pathfinding
```

See [ADR-0001](docs/adr/0001-arrangement-set-opaque-handle.md).

## Lifecycle (JS only)

`ArrangementSet` is a `#[wasm_bindgen]` opaque handle, not a serde-cloned value. JS owns the underlying allocation. Release it with `set.free()` when done:

```ts
const set = generateArrangements(input);
try {
  for (let i = 0; i < set.len; i++) {
    console.log(set.render(i, 30, 1, null));
  }
} finally {
  set.free();
}
```

On TS 5.2+ runtimes that wire `[Symbol.dispose]` to `free()`:

```ts
using set = generateArrangements(input);
// `set.free()` runs automatically at scope exit
```

Relying on `FinalizationRegistry` alone leaks the handle on runtimes whose GC is not prompt. See [ADR-0001](docs/adr/0001-arrangement-set-opaque-handle.md).

## Error handling

Errors are typed. `generateArrangements` and the `ArrangementSet` methods throw a `TabError` whose `kind` field discriminates the variant.

Before (1.x):

```ts
try {
  const compositions = wasm_create_guitar_compositions(input);
} catch (err) {
  console.error(err.message);   // free-form string
}
```

After (2.0):

```ts
try {
  const set = generateArrangements(input);
} catch (err) {
  switch ((err as TabError).kind) {
    case "parse":
      // err.errors: { line: number, text: string }[]
      for (const { line, text } of err.errors) {
        showLineMarker(line, text);
      }
      break;
    case "numFretsTooHigh":
    case "capoTooHigh":
    case "capoExceedsFrets":
    case "stringNumberOutOfRange":
    case "openPitchOutOfRange":
    case "fretRangeExceedsPitchRange":
    case "noArrangementsFound":
    case "numArrangementsOutOfRange":
    case "tuningNameUnknown":
    case "indexOutOfBounds":
      // Each variant has its own structured payload; see the "Flat TabError variants"
      // section below for the field-level switch.
      showRawError(err);
      break;
    case "unplayablePitches":
      // err.pitches: { value: string, line: number }[]
      for (const { value, line } of err.pitches) {
        showLineMarker(line, value);
      }
      break;
    default:
      // Defensive: a future non-breaking 2.x release may add a variant.
      showRawError(err);
  }
}
```

`Parse` carries structured `errors: ParseError[]` so an editor UI can highlight failing lines without re-parsing. See [ADR-0002](docs/adr/0002-tab-error-discriminated-union.md).

## Normalized input

`set.normalizedInput` returns `NormalizedBeat[]`. Each beat is a tagged variant.

Before (1.x):

```ts
for (const beat of composition.normalized_input) {
  if (beat.length === 1 && beat[0] === "REST") {
    /* ... */
  } else if (beat.length === 1 && beat[0] === "MEASURE_BREAK") {
    /* ... */
  } else {
    /* beat is string[] of pitches */
  }
}
```

After (2.0):

```ts
for (const beat of set.normalizedInput) {
  switch (beat.kind) {
    case "rest":
      /* ... */
      break;
    case "measureBreak":
      /* ... */
      break;
    case "playable":
      // beat.pitches: string[]
      break;
  }
}
```

## Tuning names

`getTuningNames()` returns a typed `TuningName[]`. The preset names are camelCase on the wire (`"openG"`, `"dropD"`, etc.).

```ts
const names: TuningName[] = getTuningNames();
// ["openG", "openD", "c6", "dsus4", "dropD", "dropC", "openC", "dropB", "openE"]
```

`tuningName` accepts the case-insensitive literal `"standard"` as standard tuning (all-zero offsets), or any variant returned by `getTuningNames()` (case-insensitive). `"standard"` is intentionally not in the typed `TuningName` union since it carries no semitone offsets; TS-strict consumers passing `"standard"` should widen the field type to `string | TuningName` or use the literal directly.

### Behavior change

In 1.x, an unrecognized or empty `tuningName` silently fell back to standard tuning. In 2.0.0 it throws `TabError::TuningNameUnknown { value }` (and the empty string is no longer accepted as a synonym for `"standard"`). Code that previously relied on the silent fallback (typos, dynamic strings, empty input) must explicitly pass `"standard"` or validate against `getTuningNames()` before calling `generateArrangements`.

The Rust-side parser (`parse_tuning`) remains case-insensitive, so 1.x calls that passed `"DropD"` or `"OpenG"` still resolve. New code should use the camelCase typed values. See [ADR-0004](docs/adr/0004-tuning-name-camelcase-wire.md).

## Direct Rust callers

A handful of changes affect Rust callers that consume the crate directly (rather than through the WASM bindings):

### `TabInput` is `#[non_exhaustive]`; use the constructor

`TabInput` can no longer be built with a struct literal from outside the crate.
Build it with `TabInput::new`, which defaults `max_fret_span_filter` to `None`,
and set the optional filter with `with_max_fret_span_filter`:

```rust
// Before
let input = TabInput {
    input: "E2\nA2".to_owned(),
    tuning_name: "standard".to_owned(),
    guitar_num_frets: 18,
    guitar_capo: 0,
    num_arrangements: 1,
    max_fret_span_filter: None,
};

// After
let input = TabInput::new("E2\nA2", "standard", 18, 0, 1);

// With the optional filter
let input = TabInput::new("E2\nA2", "standard", 18, 0, 1).with_max_fret_span_filter(7);
```

Reading fields is unchanged. JS callers are unaffected; the deserialized wire
shape is identical.

### `Arrangement::lines` is now a getter

```rust
// Before:
let lines = &arrangement.lines;

// After:
let lines = arrangement.lines();
```

### `create_arrangements` takes `NumArrangements`, not `u8`

```rust
// Before:
create_arrangements(guitar, lines, 3, None)?;

// After:
let n = NumArrangements::try_new(3)?;
create_arrangements(guitar, lines, n, None)?;
```

See [ADR-0005](docs/adr/0005-num-arrangements-newtype.md).

### `build_arrangement_set` is renamed to `generate_arrangements`

The Rust function name now matches the JS function name. The signature and behaviour are unchanged.

```rust
// Before:
use guitar_tab_generator::build_arrangement_set;
let set = build_arrangement_set(TabInput { /* ... */ })?;

// After:
use guitar_tab_generator::generate_arrangements;
let set = generate_arrangements(TabInput::new(/* ... */))?;
```

### Memoize escape hatches moved

`memoized_original_create_arrangements` and `memoized_original_parse_lines` are now under `__bench_internals`. They are not part of the stable 2.x API and may be removed without a major version bump.

```rust
// Before:
use guitar_tab_generator::memoized_original_create_arrangements;

// After (bench code only):
use guitar_tab_generator::__bench_internals::memoized_original_create_arrangements;
```

### Tuning offset helpers are now private

`parse_tuning`, `create_string_tuning_offset`, and `STD_6_STRING_TUNING_OPEN_PITCHES` are no longer re-exported from the crate root. They were the `[i8; 6]` offset machinery the table-driven preset path uses internally. Callers that need a non-preset tuning continue to build a `BTreeMap<StringNumber, Pitch>` directly and pass it to `Guitar::new`; the public `create_string_tuning(&[Pitch; N])` helper covers the common shape.

`parse_tuning` and `create_string_tuning_offset` remain reachable from criterion benches via `__bench_internals`; this namespace is `#[doc(hidden)]` and not part of the stable 2.x API.

## 2.0.0 final-pass error and validation changes

The 2.0.0 release that shipped from the `v2.0.0` branch carries an additional pass of breaking changes on top of the WASM surface redesign above:

### Flat TabError variants

The umbrella variants `Guitar`, `Arrangement`, and `InvalidInput` are removed. Each concrete failure mode is now its own variant. JS callers extend their `switch (err.kind)` blocks:

```ts
// Before:
switch (err.kind) {
  case "parse": ...; break;
  case "guitar": showMessage(err.message); break;
  case "arrangement": showMessage(err.message); break;
  case "invalidInput": showField(err.field, err.message); break;
}

// After:
switch (err.kind) {
  case "parse": ...; break;
  case "numFretsTooHigh": showFretLimit(err.numFrets, err.max); break;
  case "capoTooHigh": showCapoLimit(err.capo, err.max); break;
  case "capoExceedsFrets": showCapoVsFrets(err.capo, err.numFrets); break;
  case "stringNumberOutOfRange": ...; break;
  case "openPitchOutOfRange": ...; break;
  case "fretRangeExceedsPitchRange": ...; break;
  case "unplayablePitches": showPitches(err.pitches); break;
  case "noArrangementsFound": showNoArrangements(); break;
  case "numArrangementsOutOfRange": showRangeError(err.value, err.max); break;
  case "tuningNameUnknown": showUnknownTuning(err.value); break;
  case "indexOutOfBounds": showRangeError(err.index, err.len); break;
}
```

`TabError` remains `#[non_exhaustive]`; future 2.x releases may add variants. Defensive default arms remain a good idea.

### `UnplayablePitch` is now a public type

`TabError::UnplayablePitches { pitches }` carries `Vec<UnplayablePitch>` with `{ value: string, line: number }` per pitch. Replaces the prose "Pitch X on line N cannot be played..." string the umbrella `Arrangement` variant used to carry.

As of the additional 2.0.0 changes it is also re-exported from the crate root, so it can be named directly (`guitar_tab_generator::UnplayablePitch`) in downstream signatures, not just reached as a `TabError::UnplayablePitches` field value.

### Anyhow removed from public Rust signatures

`StringNumber::new`, `Guitar::new`, and `create_string_tuning` now return `Result<_, TabError>` instead of `anyhow::Result`. Direct Rust callers replace `.context(...)` with pattern-matching on `TabError`.

`Pitch::plus_offset` returns `Option<Pitch>` instead of `anyhow::Result<Pitch>`. Callers replace `?` with `.ok_or_else(...)` and construct a typed error themselves.

### Capo cannot exceed `num_frets`

`Guitar::new(tuning, num_frets, capo)` with `capo > num_frets` now returns `TabError::CapoExceedsFrets { capo, num_frets }`. Previously this combination underflowed `let playable_frets = num_frets - capo;` and either panicked in debug or wrapped around to a large `playable_frets` in release. Callers that supplied a capo position above the fret count must clamp before calling.

### Empty string no longer means standard tuning

`tuningName: ""` previously fell back to standard tuning. It now returns `TabError::TuningNameUnknown { value: "" }`. Callers wanting standard tuning must pass `"standard"` (case-insensitive) explicitly. The case-insensitive `"standard"` literal continues to work.

## See also

- [`CHANGELOG.md`](CHANGELOG.md) -- flat list of every breaking change.
- [`CONTEXT.md`](CONTEXT.md) -- domain glossary for 2.x terminology.
- [`docs/adr/`](docs/adr/) -- architectural decision records.
- [`types.md`](types.md) -- typed surface reference.
- [`examples/wasm.html`](examples/wasm.html) -- single-file reference demo exercising the full 2.0.0 WASM surface.
