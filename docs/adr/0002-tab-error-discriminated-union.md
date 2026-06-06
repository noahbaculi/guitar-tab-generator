# TabError is a discriminated union with a `kind` tag

Status: Superseded by ADR-0007
Date: 2026-05-20

> Superseded by [ADR-0007](0007-flat-taberror-variants.md): the three umbrella variants
> described below (`guitar`, `arrangement`, `invalidInput`) were flattened into specific
> variants, while `parse` was retained. See [ADR-0007](0007-flat-taberror-variants.md) for the
> current variant set. This record is kept for historical context.

In 2.0.0, errors crossing the WASM boundary are typed: `generate_arrangements` and the `ArrangementSet` methods throw a `TabError` whose `kind` field discriminates between four variants (`parse`, `guitar`, `arrangement`, `invalidInput`). The enum is `#[non_exhaustive]` and serialized with `#[serde(tag = "kind", rename_all = "camelCase")]`, so JS callers `switch (err.kind)` instead of parsing message strings.

## Considered Options

- **Flat `TabError(String)`.** One error type with a human-readable message. Rejected: programmatic recovery requires string-matching the message, which is fragile and breaks the moment a message is reworded.
- **Tagged union with `kind`.** A Rust enum with explicit variants, tagged on the wire by `kind`. Each variant carries its own structured fields; `Parse` carries `errors: ParseError[]` with `{line, text}` so editor highlights can locate the failing line without re-parsing. Picked.
- **One TypeScript exception class per variant.** Rejected: wasm-bindgen does not natively throw distinct error classes, and inflating the surface to N exceptions for what is structurally one tagged value adds nothing the discriminant doesn't.

## Consequences

- `TabError` is `#[non_exhaustive]`, so adding a fifth variant in 2.x is non-breaking for Rust callers (they already handle a default arm) and JS callers (they should already have a default branch).
- JS code patterns are `switch (err.kind) { case "parse": ...; case "guitar": ...; default: ... }`. The default branch is required defensively because a future 2.x release may add a variant.
- `Parse` carries `errors: ParseError[]`. The structured payload exists so an editor UI can render `{line, text}` markers next to the textarea without re-running the parser.
- `TabError` derives `PartialEq, Eq` so tests can `assert_eq!` against an expected variant.
- The non-breaking envelope under `#[non_exhaustive]` is: (a) adding a new variant is non-breaking; (b) adding a new field to an existing variant requires that variant struct to also be `#[non_exhaustive]` to remain non-breaking; (c) renaming, removing, or reshaping an existing variant requires a major version bump. Today only the top-level `TabError` enum carries the attribute, so any field addition inside `Parse`, `Guitar`, `Arrangement`, or `InvalidInput` would be breaking until those variants are individually marked.
