# Move the TypeScript surface from tsify-next back to tsify

Status: accepted
Date: 2026-06-09

[ADR-0003](0003-tsify-next-bindings.md) chose `tsify-next` to derive the TypeScript surface because the original `tsify` crate was stale when 2.0.0 shipped. That has reversed. `tsify` carries the features `tsify-next` had added, and the `tsify-next` maintainer published a 0.5.6 release whose only content is a deprecation notice: "Tsify now has all the features of `tsify-next`; `tsify-next` has served its purpose and will no longer receive updates. Use `tsify` instead." RUSTSEC-2025-0048 records the same status as an unmaintained advisory.

This crate now derives `tsify::Tsify`. Both crates sit at version 0.5.6 with the same `js` feature and the same `#[derive(Tsify)]` and `#[tsify(...)]` attributes, so the move is the crate name in `Cargo.toml` plus the three `use` statements in `lib.rs`, `parser.rs`, and `error.rs`.

## Consequences

- The RUSTSEC-2025-0048 advisory clears.
- The generated `.d.ts` public surface is unchanged except for one doc comment that named the old crate. The snapshot diff at `tests/snapshots/wasm.d.ts` is the gate, and it confirmed the rest is byte-for-byte identical.
- `serde-wasm-bindgen` stays transitive, now pulled in by `tsify` rather than `tsify-next`. The macros sub-crate in the build graph changes name from `tsify-next-macros` to `tsify-macros`, with no source or consumer-visible effect.
- The rest of ADR-0003 still holds: types tagged with the derive emit the matching TypeScript at `wasm-pack build` time, and the CI snapshot gate catches drift.
