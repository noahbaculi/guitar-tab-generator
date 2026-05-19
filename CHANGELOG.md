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

### Added

- `TabInput.maxFretSpanFilter: Option<u8>` filters arrangements by maximum non-zero fret span.

### Internal

- Adopted `tsify-next` for typed TypeScript bindings.
- Parser returns structured `Vec<ParseError>` internally; the wire format reuses the same struct via `crate::error::ParseError`.
- Released `CONTEXT.md` (domain glossary) and `docs/adr/0001-arrangement-set-opaque-handle.md` (ADR for the opaque-handle pattern).
