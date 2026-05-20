# Migration: 1.x to 2.0

Guitar Tab Generator 2.0.0 reworks the WASM surface for typed boundaries, structured errors, and cheap re-renders. This guide walks 1.x callers through every breaking change with before / after snippets.

For a flat list of changes, see [`CHANGELOG.md`](CHANGELOG.md). For the architectural decisions behind each change, see [`docs/adr/`](docs/adr/).

## Quick reference

### Renames

| 1.x | 2.0 |
|---|---|
| `wasm_create_guitar_compositions(input)` | `generateArrangements(input)` (JS) / `generate_arrangements(input)` (Rust) |
| `get_tuning_names()` | `getTuningNames()` |
| `CompositionInput` (Rust) | `TabInput` |
| `Composition[]` (output) | `ArrangementSet` handle; `set.render(i, ...)` returns the rendered string |
| `pitches` (input field) | `input` |
| `num_arrangements`, `tuning_name`, `guitar_num_frets`, `guitar_capo` | unchanged in Rust; JS sees camelCase (`numArrangements`, `tuningName`, `guitarNumFrets`, `guitarCapo`) |
| `width`, `padding`, `playback` (on the input bundle) | moved to `ArrangementSet.render(i, width, padding, playback)` |
| `"REST"` / `"MEASURE_BREAK"` (normalized_input sentinels) | `{ kind: "rest" }` / `{ kind: "measureBreak" }` |

### What you have to do

1. Replace the single `wasm_create_guitar_compositions` call with `generateArrangements` plus a loop over `set.render(i, ...)`.
2. Move `width`, `padding`, `playback` out of the input object and into the `render` call.
3. Switch error handling from string-matching to `switch (err.kind)`.
4. Call `set.free()` (or `using` on TS 5.2+) when you are done with the handle.
5. If you read `normalized_input`, switch from `["REST"]` / `["MEASURE_BREAK"]` string checks to `.kind`-based discrimination.

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
use guitar_tab_generator::create_guitar_compositions;
let compositions = create_guitar_compositions(CompositionInput { /* ... */ })?;
```

After (2.0):

```rust
use guitar_tab_generator::{generate_arrangements, TabInput};

let set = generate_arrangements(TabInput {
    input: "E2\nA2\nD3".to_owned(),
    tuning_name: "standard".to_owned(),
    guitar_num_frets: 18,
    guitar_capo: 0,
    num_arrangements: 1,
    max_fret_span_filter: None,
})?;
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
    case "guitar":
      showGuitarError(err.message);
      break;
    case "arrangement":
      showArrangementError(err.message);
      break;
    case "invalidInput":
      // err.field: string, err.message: string
      showFieldError(err.field, err.message);
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

`getTuningNames()` returns a typed `TuningName[]`. The preset names are camelCase on the wire (`"openG"`, `"dropD"`, etc.) plus the special `"standard"` value.

```ts
const names: TuningName[] = getTuningNames();
// ["openG", "openD", "c6", "dsus4", "dropD", "dropC", "openC", "dropB", "openE"]
```

The Rust-side parser (`parse_tuning`) remains case-insensitive, so 1.x calls that passed `"DropD"` or `"OpenG"` still resolve. New code should use the camelCase typed values. See [ADR-0004](docs/adr/0004-tuning-name-camelcase-wire.md).

## Direct Rust callers

A handful of changes affect Rust callers that consume the crate directly (rather than through the WASM bindings):

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
let set = generate_arrangements(TabInput { /* ... */ })?;
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

## See also

- [`CHANGELOG.md`](CHANGELOG.md) -- flat list of every breaking change.
- [`CONTEXT.md`](CONTEXT.md) -- domain glossary for 2.x terminology.
- [`docs/adr/`](docs/adr/) -- architectural decision records.
- [`types.md`](types.md) -- typed surface reference.
- [`examples/wasm.html`](examples/wasm.html) -- single-file reference demo exercising the full 2.0.0 WASM surface.
