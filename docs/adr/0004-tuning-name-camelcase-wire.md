# TuningName serializes as camelCase on the wire

Status: accepted
Date: 2026-05-20

In 2.0.0, the `TuningName` enum (`OpenG`, `DropD`, `C6`, etc.) crosses the WASM boundary as a camelCase string: `"openG"`, `"dropD"`, `"c6"`. `getTuningNames()` returns `TuningName[]` typed as a union of those camelCase literals. The Rust-side parser (`parse_tuning`) remains case-insensitive (`"DropD"`, `"dropd"`, `"DROPD"` all resolve) because it lowercases first.

## Considered Options

- **Serde-default PascalCase (`"OpenG"`, `"DropD"`).** Matches the Rust enum variant names verbatim. Rejected: PascalCase string literals are out-of-step with the JS/TS ecosystem, where camelCase is the idiom for member-like identifiers.
- **Raw lowercase strings without type safety.** The 1.x shape; `get_tuning_names()` returned `string[]`. Rejected: nothing in the type system stops a JS caller from passing `"openg"` or `"OpenG"` and getting a silent parse fallback to standard.
- **camelCase typed enum (`"openG"`, `"dropD"`).** Picked. `tsify-next` emits the union type; `parse_tuning`'s case-insensitivity preserves 1.x call sites.

## Consequences

- TypeScript callers see idiomatic enum values: `tuningName: "dropD"`.
- 1.x calls that passed `"DropD"` or `"standard"` still work because the parser lowercases.
- A typo (`"dropd!"`) is caught by `tsc` against the generated `TuningName` union, not silently at runtime.
- The conversion uses serde's `#[serde(rename_all = "camelCase")]`; no manual mapping table.
