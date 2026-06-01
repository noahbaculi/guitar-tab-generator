# Use tsify-next to generate the TypeScript surface from Rust types

Status: accepted
Date: 2026-05-20

In 2.0.0, types crossing the WASM boundary derive `tsify_next::Tsify`. The generated `.d.ts` lives next to the `.wasm` after `wasm-pack build` and is the canonical TypeScript surface for downstream consumers. A snapshot of the generated `.d.ts` is committed at `tests/snapshots/wasm.d.ts` and a CI step fails the build when the generated file drifts from the snapshot.

## Considered Options

- **Hand-written `.d.ts`.** Author the TypeScript interface alongside the Rust types. Rejected: silent drift between the Rust types and the hand-written declarations is too easy. Every Rust-type change requires a manual sync step that an unaware contributor would skip.
- **Plain `serde-wasm-bindgen` with separately declared TS interfaces.** Same drift risk as hand-written; rejected for the same reason.
- **`tsify-next` derive macro.** Rust types tagged with `#[derive(Tsify)]` plus `#[tsify(from_wasm_abi)]` or `#[tsify(into_wasm_abi)]` emit matching TypeScript declarations at `wasm-pack build` time. Picked.

## Consequences

- A change to any boundary-crossing Rust type forces a corresponding `.d.ts` update; the snapshot diff in CI is the gate.
- `serde-wasm-bindgen` becomes a transitive dependency only (pulled in by `tsify-next`), not a direct one.
- One additional proc-macro dep in the build graph and a generated-file diff lands in any PR that touches a boundary type. Trade-off accepted because the alternative is silent drift.
- `ArrangementSet` is the documented exception: it is a `#[wasm_bindgen]` opaque handle, not a serde-serialized value, so it sidesteps the tsify path (see [ADR-0001](0001-arrangement-set-opaque-handle.md)).
