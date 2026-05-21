# 2.0.0 Final Pass: Structured Errors and Validation Fixes

Status: ready-for-agent
Date: 2026-05-20

## Goal

Land the breaking changes that should ride with 2.0.0 so they do not force a future major. Scope is limited to the typed error surface, removing `anyhow` from public Rust signatures, fixing a real capo validation bug, and tightening the `tuningName` contract. Performance work and the Rust public-surface trim are deliberately deferred.

## Motivation

The 2.0.0 branch shipped the typed WASM boundary, but `TabError::Guitar`, `TabError::Arrangement`, and `TabError::InvalidInput` still carry `message: String` as the only payload. `src/error.rs:41` calls that out as "UI strings, not a stable wire field." The JS demo cannot branch on subkind without string-matching the message. With `#[non_exhaustive]` we can add variants in 2.x, but we cannot remove the umbrella variants without a major bump. The window is now.

In parallel, three smaller issues should ride with the same release:

1. `src/guitar.rs:123` does `let playable_frets = num_frets - capo;` after only bounds-checking each input independently. `num_frets=2, capo=8` underflows. The capo cap is 8 and the fret cap is 30, so any caller with `num_frets in 1..=7` plus `capo in num_frets+1..=8` hits it. This is a real bug, not a hypothetical.
2. `parse_tuning` accepts the empty string as a synonym for `"standard"`. Every other unknown tuning name returns `TabError::InvalidInput`. The asymmetry is a 1.x holdover and contradicts 2.0.0's stricter validation posture.
3. `StringNumber::new`, `Guitar::new`, and `create_string_tuning` return `anyhow::Result`. Anyhow strings leak through the Rust public API. The boundary already wraps them with `e.to_string()` into `TabError::Guitar { message }`, so removing anyhow makes the Rust signatures match what JS already sees.

## Scope

### In scope

- Restructure `TabError` as a flat tagged union with one variant per concrete error condition. The umbrella `Guitar`, `Arrangement`, and `InvalidInput` variants are removed.
- Promote `UnplayablePitch` from a private `arrangement.rs` struct to a public, Tsify-typed wire struct used in `TabError::UnplayablePitches`.
- Replace `anyhow::Error` on `StringNumber::new`, `Guitar::new`, and `create_string_tuning` with `Result<_, TabError>`. Change `Pitch::plus_offset` to return `Option<Pitch>` (it has no contextual info to populate a `TabError`; the caller wraps `None` into a variant that does). Ripple through internal helpers (`check_fret_number`, `check_capo_number`, `create_string_range`, `create_arrangements`).
- Add `TabError::CapoExceedsFrets { capo, num_frets }` and the cross-check in `Guitar::new` that prevents the underflow.
- Drop the `tuning_name.is_empty()` branch from `parse_tuning`. Empty string now returns `TabError::TuningNameUnknown { value: "" }`.
- Update `examples/advanced.rs` and `tests/integration_public_surface.rs` for the changed error types. They keep their existing structure; only the `Err` branches change.
- Add ADR-0007 for the flat `TabError` shape. Update `MIGRATION.md`, `CHANGELOG.md`, and `CONTEXT.md`.
- Drop `anyhow` from `Cargo.toml` if no internal use remains after the cascade. Verified with `cargo tree`.

### Out of scope

- Public-surface trim. `parse_lines`, `render_tab`, `create_arrangements`, `create_string_tuning`, `Line`, `BeatVec`, `PitchVec` stay re-exported from the crate root. The advanced Rust path is preserved.
- Performance work. Covered by `.scratch/post-2.0.0-internals/PRD.md`.
- Custom tuning over the WASM boundary. Additive, deferred to 2.x.
- Algorithm changes (difficulty function, fret-span weighting).

## Type surface

### `TabError` after the change

```rust
// src/error.rs

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct UnplayablePitch {
    pub value: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[non_exhaustive]
pub enum TabError {
    Parse { errors: Vec<ParseError> },
    NumFretsTooHigh { num_frets: u8, max: u8 },
    CapoTooHigh { capo: u8, max: u8 },
    CapoExceedsFrets { capo: u8, num_frets: u8 },
    StringNumberOutOfRange { value: u8, max: u8 },
    OpenPitchOutOfRange { string: u8, semitones: i16 },
    FretRangeExceedsPitchRange { open_pitch: String, playable_frets: u8 },
    UnplayablePitches { pitches: Vec<UnplayablePitch> },
    NumArrangementsOutOfRange { value: u8, max: u8 },
    TuningNameUnknown { value: String },
    IndexOutOfBounds { index: usize, len: usize },
}
```

`Display` and `Error` are hand-rolled. The output strings reproduce the pre-2.0.0 messages exactly where they have user-facing equivalents (the existing `Display` tests in `error.rs` pin the wire format; new tests pin one Display string per new variant).

### TypeScript shape JS sees

```ts
type TabError =
  | { kind: "parse"; errors: ParseError[] }
  | { kind: "numFretsTooHigh"; numFrets: number; max: number }
  | { kind: "capoTooHigh"; capo: number; max: number }
  | { kind: "capoExceedsFrets"; capo: number; numFrets: number }
  | { kind: "stringNumberOutOfRange"; value: number; max: number }
  | { kind: "openPitchOutOfRange"; string: number; semitones: number }
  | { kind: "fretRangeExceedsPitchRange"; openPitch: string; playableFrets: number }
  | { kind: "unplayablePitches"; pitches: UnplayablePitch[] }
  | { kind: "numArrangementsOutOfRange"; value: number; max: number }
  | { kind: "tuningNameUnknown"; value: string }
  | { kind: "indexOutOfBounds"; index: number; len: number };
```

The demo's existing `switch (err.kind)` block expands from 4 cases to 11. Cases that need only a message can fall through to a generic handler; specific cases (`parse`, `unplayablePitches`, `tuningNameUnknown`) carry structured payloads the UI can use.

### Variant-to-callsite mapping

| Variant | Replaces |
|---|---|
| `Parse` | unchanged |
| `NumFretsTooHigh` | `check_fret_number` anyhow string |
| `CapoTooHigh` | `check_capo_number` anyhow string |
| `CapoExceedsFrets` | new; no previous code path produced this safely (it underflowed) |
| `StringNumberOutOfRange` | `StringNumber::new` anyhow strings (both the zero and too-high arms collapse to one variant with an explicit value) |
| `OpenPitchOutOfRange` | `Guitar::new`'s capo loop catching `Pitch::plus_offset` returning `None`. The variant carries the string number and semitone offset; `plus_offset` itself has no view of either |
| `FretRangeExceedsPitchRange` | `create_string_range` anyhow string |
| `UnplayablePitches` | `create_arrangements`'s "The following pitch(es) cannot be played" anyhow string. Carries the existing `UnplayablePitch` records instead of formatting them into prose |
| `NumArrangementsOutOfRange` | `NumArrangements::try_new`'s previous `TabError::InvalidInput` with `field: "numArrangements"` |
| `TuningNameUnknown` | `parse_tuning`'s previous `TabError::InvalidInput` with `field: "tuningName"` |
| `IndexOutOfBounds` | `out_of_bounds_error` in `lib.rs`, previously `TabError::InvalidInput` with `field: "index"` |

`StringNumber::new` collapses its two error arms (zero vs. above max) into one variant because the caller can disambiguate from the `value` field. The Display impl reproduces the two distinct messages by branching on `value == 0`.

## Implementation details

### Error wiring through memoize

`create_arrangements` currently returns `Result<Vec<Arrangement>, Arc<anyhow::Error>>`. The `Arc` is there because `memoize` requires the `Err` to be `Clone`. The new signature is `Result<Vec<Arrangement>, Arc<TabError>>` (preserving the `Arc` for cache-friendly cloning, matching the existing `parse_lines: Result<_, Arc<Vec<ParseError>>>` pattern). At the boundary, `lib.rs::generate_arrangements` unwraps it the same way as parse errors:

```rust
arrangement::create_arrangements(...)
    .map_err(|arc| Arc::try_unwrap(arc).unwrap_or_else(|a| (*a).clone()))?;
```

### Capo cross-check placement

```rust
// src/guitar.rs::Guitar::new
check_fret_number(num_frets)?;        // -> TabError::NumFretsTooHigh
check_capo_number(capo)?;             // -> TabError::CapoTooHigh
if capo > num_frets {                 // new
    return Err(TabError::CapoExceedsFrets { capo, num_frets });
}
let playable_frets = num_frets - capo;
```

The order matters: the individual bounds run first so a caller with `num_frets=100, capo=100` sees both upper bounds rather than a single "capo exceeds frets" message about values that were themselves invalid.

### Empty-string tuning

```rust
// src/parser.rs::parse_tuning
// Before:
Err(_) if tuning_name.is_empty() || tuning_name.eq_ignore_ascii_case("standard") => Ok([0; 6]),
Err(_) => Err(TabError::InvalidInput { ... }),

// After:
Err(_) if tuning_name.eq_ignore_ascii_case("standard") => Ok([0; 6]),
Err(_) => Err(TabError::TuningNameUnknown { value: tuning_name.to_owned() }),
```

The case-insensitive `"standard"` literal stays. Empty string now produces `TuningNameUnknown { value: "" }`.

### `anyhow` removal scope

| File | Change |
|---|---|
| `src/string_number.rs` | `StringNumber::new` returns `Result<Self, TabError>`. Drop `use anyhow`. |
| `src/guitar.rs` | `Guitar::new`, `create_string_tuning`, `check_fret_number`, `check_capo_number`, `create_string_range` all return `Result<_, TabError>`. Drop `use anyhow`. |
| `src/pitch.rs` | `Pitch::plus_offset` returns `Option<Pitch>` (the new-index math has no contextual data to populate a typed error). Drop `use anyhow`. |
| `src/arrangement.rs` | `create_arrangements` returns `Result<_, Arc<TabError>>`. The internal helpers that produced anyhow now build `TabError` directly. Drop `use anyhow`. |
| `src/parser.rs` | Already uses `TabError`; no change. |
| `Cargo.toml` | Remove `anyhow` if `cargo tree` confirms no remaining use. |

Verification: `cargo tree -p anyhow` after the cascade. If any transitive consumer pulls it back in, leave it as a transitive dep but remove the direct dependency.

### `examples/advanced.rs` and `tests/integration_public_surface.rs`

Both files use `parse_lines`, `render_tab`, `create_arrangements`, `create_string_tuning`. They stay; only the `match err` arms change to pattern-match on the new flat `TabError` variants instead of unwrapping anyhow strings. No structural rewrite.

The canary test at `tests/integration_public_surface.rs` gains explicit assertions for the new error variants firing on the right inputs.

## Tests

### New unit tests

- `error::test_tab_error_display::display_for_<variant>` for each new variant. Pins the Display string.
- `error::test_tab_error_display::string_number_out_of_range_disambiguates_on_value` confirms `value == 0` and `value > max` produce different Display strings from one variant.
- `guitar::test_create_guitar::capo_exceeds_frets_returns_typed_error` for `num_frets=2, capo=4`. Asserts `TabError::CapoExceedsFrets { capo: 4, num_frets: 2 }`. Also asserts no panic in debug mode (the current code underflows).
- `parser::test_parse_tuning::empty_string_returns_tuning_name_unknown`.
- `parser::test_parse_tuning::standard_is_still_accepted` (regression guard).

### New integration tests (in `lib.rs::test_boundary_types` and `tests/integration_public_surface.rs`)

- One test per variant exercises the boundary path. Existing `invalid_input_returns_parse_error`, `invalid_guitar_config_returns_guitar_error`, and `unreachable_pitch_returns_arrangement_error` all rewrite to assert on the new variants.

### Preserved

- All `arrangement.rs` proptests. They do not assert on error shape.
- All existing parser tests beyond the two listed above.
- `parse_error_display::reproduces_legacy_message_format`.

### Removed

- `tab_error_display::parse_variant_joins_errors_with_newlines` keeps. The `invalid_input_includes_field_name` test rewrites to test `IndexOutOfBounds` or another new variant since `InvalidInput` is gone.

## Sequencing

Each step is a green-tests commit. The order keeps the build compiling at every step.

1. **Define the new `TabError` shape alongside the old one.** Add the new variants and `UnplayablePitch` as a public struct. Keep `Guitar { message }`, `Arrangement { message }`, `InvalidInput { field, message }` for the moment so the build stays green. New variant set is `#[non_exhaustive]`-safe; tests do not depend on enum exhaustiveness.
2. **Migrate `StringNumber::new` to `TabError` and `Pitch::plus_offset` to `Option<Pitch>`.** Smallest surface, lets the next steps cascade.
3. **Migrate `Guitar::new`, `create_string_tuning`, and the helpers.** Including the new `CapoExceedsFrets` check.
4. **Migrate `create_arrangements` to `Result<_, Arc<TabError>>`.** Replace the "unplayable pitches" anyhow with `TabError::UnplayablePitches { pitches }`. The `UnplayablePitch` struct moves from `arrangement.rs` private to `error.rs` public.
5. **Update `parse_tuning` for empty-string and the typed `TuningNameUnknown`.**
6. **Update `lib.rs::generate_arrangements`** to propagate typed errors without `.to_string()` wrapping. `out_of_bounds_error` returns `TabError::IndexOutOfBounds`. `NumArrangements::try_new` returns `TabError::NumArrangementsOutOfRange`.
7. **Delete the old umbrella variants** (`Guitar`, `Arrangement`, `InvalidInput`) from `TabError`. Update the few remaining call sites the cascade missed (build will fail if any are left).
8. **Update `examples/advanced.rs` and `tests/integration_public_surface.rs`** for the new `Err` arms.
9. **Add tests for each new variant** (unit + boundary integration).
10. **Drop `anyhow`** from `Cargo.toml` if `cargo tree` shows no remaining direct use.
11. **Docs:** ADR-0007 (flat TabError), `MIGRATION.md` new section, `CHANGELOG.md` bullets, `CONTEXT.md` entry for `UnplayablePitch`.

## Migration notes

The CHANGELOG bullets:

- `TabError` is now a flat tagged union; `Guitar`, `Arrangement`, and `InvalidInput` umbrellas are removed in favor of specific variants. JS callers must extend their `switch (err.kind)` blocks. The Tsify wire shape is the tagged object only; there is no longer a free-form `message` field on the catch-all. UIs that previously rendered `err.message` build a per-kind string from the structured fields, or fall through to a default handler in the `switch`.
- `UnplayablePitch` is now a public type carried by `TabError::UnplayablePitches`. Replaces the prose "The following pitch(es) cannot be played" string with structured `{ value, line }` records.
- `StringNumber::new`, `Guitar::new`, and `create_string_tuning` return `Result<_, TabError>` instead of `anyhow::Result`. Direct Rust callers replace `.context(...)` with pattern-match on `TabError`.
- `Pitch::plus_offset` returns `Option<Pitch>` instead of `anyhow::Result<Pitch>`. Direct Rust callers replace `?` with `.ok_or_else(...)` or `.context(...)`-equivalent error construction at the call site.
- `tuningName: ""` no longer means standard tuning. Pass `"standard"` (case-insensitive) explicitly.
- `Guitar::new(tuning, num_frets, capo)` with `capo > num_frets` returns `TabError::CapoExceedsFrets` instead of underflowing. Direct Rust callers that previously relied on this path (none expected) will now see a typed error.

## Open questions

None at spec time. The implementation plan owns the remaining choices: exact Display strings per variant, whether `Arc<TabError>` or bare `TabError` for `create_arrangements`' Err (defaulting to `Arc` to match the `parse_lines` pattern), and whether `anyhow` survives as a transitive dep.

A judgement call worth flagging: `Pitch::plus_offset` shrinks from `Result<_, anyhow::Error>` to `Option<_>` rather than to `Result<_, TabError>`. The variant `OpenPitchOutOfRange` needs the string number and semitone offset, neither of which `plus_offset` knows; making it return `TabError` forces a synthetic variant that the only caller (`Guitar::new`) would have to throw away anyway. `Option` keeps `plus_offset` honest about what it actually computes.

## What 2.0.0 still defers

- Public-surface trim. `parse_lines`, `render_tab`, `create_arrangements`, `create_string_tuning` remain re-exported. If a future major version chooses to trim, the constraint will be that `Arrangement::lines()` returns `Line<BeatVec<PitchFingering>>`, which keeps `Line` and `BeatVec` in the SemVer contract for now.
- Performance. The post-2.0.0 PRD owns the pathfinding and allocation work.
- Custom tuning over the WASM boundary. Additive.
