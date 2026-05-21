# Changelog

## 2.0.0 -- 2026-05-19

### Breaking changes

- `wasm_create_guitar_compositions(input)` replaced by `generateArrangements(input)` (JS) / `generate_arrangements(input)` (Rust).
- `get_tuning_names()` replaced by `getTuningNames()`; returns typed `TuningName[]` instead of `string[]`.
- Output shape: `Composition[]` replaced by a single `ArrangementSet` handle. Access pattern: `for (let i = 0; i < set.len; i++) set.render(i, width, padding, playback)`.
- Input field `pitches` renamed to `input`.
- Input field renames flow to the JS side as `numArrangements`, `tuningName`, `guitarNumFrets`, `guitarCapo`, `maxFretSpanFilter`.
- Render parameters (`width`, `padding`, `playback`) moved from input bundle to `ArrangementSet.render(...)`.
- `normalized_input` sentinels (`"REST"`, `"MEASURE_BREAK"`) replaced by tagged variants (`{ kind: "rest" }`, `{ kind: "measureBreak" }`).
- Errors are typed: throws `TabError` with `kind` discriminator. `Parse` carries `errors[].line` and `errors[].text` for inline editor highlights.
- `TuningName` wire serialization switched from PascalCase (`"OpenG"`) to camelCase (`"openG"`). Input parsing remains case-insensitive.
- Renamed `CompositionInput` to `TabInput` to align with the domain glossary; the old name was a 1.x holdover.
- `create_arrangements` takes `NumArrangements` instead of `u8` for `num_arrangements`; construct via `NumArrangements::try_new(n)?`. Direct Rust callers no longer validate the range themselves.
- `memoized_original_create_arrangements` and `memoized_original_parse_lines` moved from the crate root to the `__bench_internals` namespace. The namespace is `#[doc(hidden)]`, not part of the stable 2.x API, and may be removed without a major version bump.
- `Arrangement::lines` is now a getter returning `&[Line<BeatVec<PitchFingering>>]` instead of a `pub` field. Direct Rust consumers call `arrangement.lines()` instead of `&arrangement.lines`. `difficulty` and `max_fret_span` were already getters.
- `parse_tuning`, `create_string_tuning_offset`, and `STD_6_STRING_TUNING_OPEN_PITCHES` are no longer re-exported from the crate root. They were leaked from a 1.x composition pattern that `generate_arrangements` and the `tuning_name` field on `TabInput` make redundant; non-preset tunings continue to flow through `create_string_tuning(&[Pitch; N])` plus `Guitar::new`. The two helpers remain reachable from criterion benches via `__bench_internals` and may be removed without a major version bump.
- `build_arrangement_set` is renamed to `generate_arrangements`. The Rust function and the JS function (`generateArrangements`) now share a single implementation and parallel names; the `#[wasm_bindgen]` wrapper is gone. Direct Rust callers update their imports.
- `TabError` flattened: the umbrella `Guitar { message }`, `Arrangement { message }`, and `InvalidInput { field, message }` variants are removed. Each failure mode now has its own variant with structured payload. See [MIGRATION.md](MIGRATION.md#flat-taberror-variants) for the full mapping and [ADR-0007](docs/adr/0007-flat-taberror-variants.md) for the rationale.
- `UnplayablePitch` is now a public type carried by `TabError::UnplayablePitches`. Replaces the prose error string with structured `{ value, line }` records.
- `StringNumber::new`, `Guitar::new`, and `create_string_tuning` return `Result<_, TabError>` instead of `anyhow::Result`. Direct Rust callers must update error handling.
- `Pitch::plus_offset` returns `Option<Pitch>` instead of `anyhow::Result<Pitch>`. Callers replace `?` with `.ok_or_else(...)`.
- `Guitar::new` validates that `capo <= num_frets` before computing `playable_frets`. The previous code underflowed; the new behavior is `TabError::CapoExceedsFrets`.
- `tuningName: ""` no longer means standard tuning. Pass `"standard"` (case-insensitive) explicitly.
- `Guitar::MAX_NUM_FRETS` and `Guitar::MAX_CAPO` are now `pub const` on `Guitar`, alongside the existing `NumArrangements::MAX`. (Additive.)
- `StringNumber::MAX` is now `pub const` on `StringNumber`. (Additive.)

### Added

- `TabInput.maxFretSpanFilter: Option<u8>` filters arrangements by maximum non-zero fret span. Emitted as `maxFretSpanFilter?: number` in the TypeScript surface, so TS-strict callers may omit the key.
- `PitchFingering::string_number()`, `::fret()`, and `::pitch()` getters give Rust callers structured read access to the fingerings returned by `Arrangement::lines()`, replacing the previous `Debug`-only path. The fields stay `pub(crate)`; the getters are the stable surface.

### Internal

- Adopted `tsify-next` for typed TypeScript bindings. `serde-wasm-bindgen` is now a transitive dependency only; no consumer-visible change beyond `cargo tree` ordering.
- Parser returns structured `Vec<ParseError>` internally; the wire format reuses the same struct via `crate::error::ParseError`.
- Released `CONTEXT.md` (domain glossary) and `docs/adr/0001-arrangement-set-opaque-handle.md` (ADR for the opaque-handle pattern).
- `TabError` now derives `PartialEq, Eq` and carries `#[non_exhaustive]` so future structured-error variants land non-breakingly.
- Dropped the unused `serde` `"rc"` feature.
