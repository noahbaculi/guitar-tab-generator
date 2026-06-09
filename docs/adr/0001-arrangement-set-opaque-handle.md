# ArrangementSet crosses the WASM boundary as an opaque handle

Status: accepted
Date: 2026-05-19

In 2.0.0, generating tabs splits into two calls: `generate_arrangements(TabInput) -> Result<ArrangementSet, TabError>` (exported to JS as `generateArrangements`), then `set.render(i, width, padding, playback) -> String` (the rendered ASCII tab, the [[RenderedTab]] in CONTEXT.md). `ArrangementSet` is a `#[wasm_bindgen]` struct, not a serde-serialized value, so the arrangements themselves stay Rust-side and only the rendered ASCII tabs cross the wire.

## Considered Options

- **Pattern A (serde wire).** Every `Arrangement` (including its `Vec<Line<Vec<PitchFingering>>>`) serializes to JS. Rejected: the demo never inspects fingerings, and for `num_arrangements = 20` the wire payload runs ~100KB of structured data that gets thrown away.
- **Pattern B (opaque handle).** `ArrangementSet` exposes getter and render methods. Demo holds the handle across width / playback / padding changes and re-renders cheaply without re-pathfinding. Picked.
- **Hybrid.** Opaque handle plus a separate serde-friendly view when the demo wants to introspect one arrangement. Rejected as speculative; no consumer needs it today.

## Consequences

- The demo on noahbaculi.com must manage the `ArrangementSet` lifecycle. Call `set.free()` (or `using` in runtimes with explicit resource management) when done; `FinalizationRegistry` reclaims it otherwise, but not promptly, so an explicit `free()` is the recommended path.
- Adding a new piece of per-arrangement information (e.g. a fingering inspector) means adding a getter method on `ArrangementSet`, not a field on a serialized struct. This is the deliberate trade-off: cheaper hot path, slightly more API surface to grow.
- The codebase otherwise uses `tsify` (with `serde-wasm-bindgen` pulled in transitively) for boundary crossings. `ArrangementSet` is the documented exception; `TabInput`, `NormalizedBeat`, and `TabError` keep the serde-via-tsify path. The rendered tab crosses as a bare `String`, not a serde-serialized type.
