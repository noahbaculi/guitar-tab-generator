# Changelog

## Unreleased

### Internal

- Index `yen()` successors by node group instead of re-scanning the full node list. `calc_next_nodes` now looks up the one contiguous beat group through `path_node_groups` rather than filtering a flattened `Vec<Node>`, which drops the adjacency term from roughly O(k * N^2) toward O(group size) on multi-arrangement and long inputs. Arrangement output is byte-for-byte unchanged. In CI the `fur_elise_3_arrangements` and `fur_elise_5_arrangements` benches fell about 22% (roughly 21.6 ms to 16.7 ms) while the untouched `parse_lines` control held flat. The single-arrangement long-input bench moved only 1 to 3%, matching the O(k * N^2) shape where the win scales with the arrangement count k. (Indicative cross-run CI figures.)
- Dropped the `average` dependency and compute the non-zero-fret mean inline. This removes eight crates from the non-dev dependency graph (`average`, `rayon`, `rayon-core`, the three `crossbeam-*` crates, `easy-cast`, `float-ord`), so Rayon no longer compiles into the production or WASM build. That restores [ADR-0009](docs/adr/0009-pipeline-stays-single-threaded.md)'s no-Rayon state and slightly shrinks the size-optimized WASM artifact. The mean is arithmetically identical, pinned by the existing `calc_avg_non_zero_fret` tests.
- Compile the pitch regex once into a process-lifetime `LazyLock<Regex>` static instead of rebuilding it on every cache-missing `parse_lines` call. Behavior is unchanged. The `parse_lines` bench, which reparses each iteration, fell from roughly 285 µs to 77 µs in CI (about a 3.7x speedup), since regex compilation dominated the per-parse cost and now runs once for the process. (Indicative cross-run CI figure.)
- Pad a rendered row's trailing dashes in place with `push` instead of building and copying a throwaway `"-".repeat(..)` `String`. The row buffer already reserves the full width, so the bytes have a home. Render output is unchanged, pinned by the render snapshot tests. The `render_tab` bench fell roughly 4 to 6% against the prior commit in CI, while the untouched `parse_lines` and `fur_elise_1_arrangement` benches held flat, so the gain tracks this change rather than runner variance. (Indicative cross-run CI figure, not a persisted baseline.)
- Measure each fret's display width in `calc_fret_width_max` with a digit-count check instead of allocating a `String` per fingering to read its length. The width is identical for any fret and reuses the same digit rule `render_fret` already applies. The `render_tab` bench fell a further 3.5 to 4.5% in CI, roughly 8 to 9% cumulative across the two render cleanups.

## 2.0.0 -- 2026-06-05

### Breaking changes

- `wasm_create_guitar_compositions(input)` replaced by `generateArrangements(input)` (JS) / `generate_arrangements(input)` (Rust).
- JS export `get_tuning_names()` renamed to `getTuningNames()` and now returns the typed `TuningName[]` union instead of `string[]`. The Rust crate-root re-export keeps the name `get_tuning_names`; its return type is now `Vec<TuningName>`.
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
- `parse_tuning`, `create_string_tuning_offset`, and `STD_6_STRING_TUNING_OPEN_PITCHES` are no longer re-exported from the crate root. They were leaked from a 1.x composition pattern that `generate_arrangements` and the `tuning_name` field on `TabInput` make redundant; non-preset tunings continue to flow through `create_string_tuning(&[Pitch])` plus `Guitar::new`. The two helpers remain reachable from criterion benches via `__bench_internals` and may be removed without a major version bump.
- `wrapper_create_arrangements` is renamed to `generate_arrangements`. The Rust function and the JS function (`generateArrangements`) now share a single implementation and parallel names; the `#[wasm_bindgen]` wrapper is gone. Direct Rust callers update their imports.
- `TabError` flattened: the umbrella `Guitar { message }`, `Arrangement { message }`, and `InvalidInput { field, message }` variants are removed. Each failure mode now has its own variant with structured payload. See [MIGRATION.md](MIGRATION.md#flat-taberror-variants) for the full mapping and [ADR-0007](docs/adr/0007-flat-taberror-variants.md) for the rationale.
- `TabError::NoArrangementsFound` (payloadless) reports the case where every input pitch reaches the guitar but no valid arrangement exists. Reachable from inputs like duplicate pitches in a single beat (e.g. `"E2E2"`), which the `no_duplicate_strings` constraint filters to zero candidate fingerings. JS callers handling an exhaustive `switch (err.kind)` must add a `"noArrangementsFound"` arm.
- `UnplayablePitch` is now a public type carried by `TabError::UnplayablePitches`. Replaces the prose error string with structured `{ value, line }` records.
- `StringNumber::new`, `Guitar::new`, and `create_string_tuning` return `Result<_, TabError>` instead of `anyhow::Result`. Direct Rust callers must update error handling.
- `Pitch::plus_offset` returns `Option<Pitch>` instead of `anyhow::Result<Pitch>`. Callers replace `?` with `.ok_or_else(...)`.
- `Guitar::new` validates that `capo <= num_frets` before computing `playable_frets`. The previous code underflowed; the new behavior is `TabError::CapoExceedsFrets`.
- `tuningName: ""` no longer means standard tuning. Pass `"standard"` (case-insensitive) explicitly.
- `Guitar::MAX_NUM_FRETS` and `Guitar::MAX_CAPO` are now `pub const` on `Guitar`, alongside the existing `NumArrangements::MAX`. (Additive.)
- `StringNumber::MAX` is now `pub const` on `StringNumber`. (Additive.)
- `TabInput` is now `#[non_exhaustive]`. Construct it with `TabInput::new(input, tuningName, guitarNumFrets, guitarCapo, numArrangements)` and set the optional filter via `.with_max_fret_span_filter(n)`. JS callers are unaffected; the deserialized wire shape is unchanged. See [ADR-0008](docs/adr/0008-tab-input-sealed-constructor.md).
- Crate edition upgraded to 2024; minimum supported Rust version is now 1.86, declared via `rust-version` in `Cargo.toml`.

### Added

- `TabInput.maxFretSpanFilter: Option<u8>` filters arrangements by maximum non-zero fret span. Emitted as `maxFretSpanFilter?: number` in the TypeScript surface, so TS-strict callers may omit the key.
- `PitchFingering::string_number()`, `::fret()`, and `::pitch()` getters give Rust callers structured read access to the fingerings returned by `Arrangement::lines()`, replacing the previous `Debug`-only path. The fields stay `pub(crate)`; the getters are the stable surface.
- `UnplayablePitch` is re-exported from the crate root, so direct Rust callers can name the type in signatures. It was previously reachable only as a `TabError::UnplayablePitches` field value.
- `TabError::RenderWidthTooSmall { width, min }` is returned by `ArrangementSet::render` when `width` is below the minimum needed to lay out one beat at the given `padding` (`min_render_width(padding)`). A too-small width previously underflowed the renderer's column math (debug panic, release allocation blow-up) or stalled its wrap loop. JS callers with an exhaustive `switch (err.kind)` may add a `"renderWidthTooSmall"` arm; the existing default arm already covers it.
- Input longer than 65,535 lines returns `TabError::InputTooManyLines { max }` (where `max` is the inclusive line limit) rather than overflowing the internal `u16` beat index. JS callers with an exhaustive `switch (err.kind)` may add an `"inputTooManyLines"` arm; the existing default arm already covers it. A real transcription is far below this bound, so this only rejects pathological input.
- `TuningName` now derives `Clone, Copy, PartialEq, Eq, Hash` and `NormalizedBeat` derives `PartialEq, Eq`, so Rust callers can compare the values returned by `get_tuning_names()` and `ArrangementSet::normalized_input()` with `==` rather than `matches!`.

### Internal

- Adopted `tsify-next` for typed TypeScript bindings. `serde-wasm-bindgen` is now a transitive dependency only; no consumer-visible change beyond `cargo tree` ordering.
- Parser returns structured `Vec<ParseError>` internally; the wire format reuses the same struct via `crate::error::ParseError`.
- Released `CONTEXT.md` (domain glossary) and the architecture decision records in `docs/adr/` (ADR-0001 through ADR-0008), beginning with the opaque-handle pattern in [ADR-0001](docs/adr/0001-arrangement-set-opaque-handle.md).
- `TabError` now derives `PartialEq, Eq` and carries `#[non_exhaustive]` so future structured-error variants land non-breakingly.
- Dropped the unused `serde` `"rc"` feature.
