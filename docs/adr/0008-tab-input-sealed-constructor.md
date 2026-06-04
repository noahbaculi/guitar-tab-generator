# 0008: Sealed TabInput with a constructor

Status: accepted
Date: 2026-06-01

## Context

`TabInput` is the configuration bundle callers hand to `generate_arrangements`.
It crosses the WASM boundary via tsify (deserialized from a camelCase JS
object) and is also built directly by Rust callers, the examples, the crate
doctest, and the public-surface canary test.

Custom tuning is a planned 2.x feature. It lands as a new optional field on
`TabInput`: a custom open-string set alongside the existing `tuning_name`
preset selector. Adding a field to an open struct is additive on the JS wire
(the key is optional), but it breaks every Rust `TabInput { ... }` literal,
which under SemVer forces a major version bump. The 2.0.0 release is the last
window to prevent that.

## Decision

`TabInput` is `#[non_exhaustive]`. External crates can no longer build it with
a struct literal; they call `TabInput::new(input, tuning_name,
guitar_num_frets, guitar_capo, num_arrangements)`, which defaults
`max_fret_span_filter` to `None`, and set optional fields through `with_*`
setters (`with_max_fret_span_filter`). Public fields stay readable, so the
deserialize-then-read pattern is unchanged, and the JS wire shape is untouched
(`#[non_exhaustive]` is a Rust-visibility concept that serde and tsify ignore,
since their generated code lives in the defining crate).

When custom tuning lands, `new` gains a line defaulting the field to `None`, a
`with_custom_tuning(...)` setter is added, and existing `new(...)` callers keep
compiling. The field addition ships in a 2.x minor.

A full builder type was considered and rejected as heavier than the field
count warrants; `new` plus `with_*` setters covers the one optional field
today and scales to future ones.

The output types (`NormalizedBeat`, `Line`, `ParseError`, `UnplayablePitch`)
were deliberately left open, because no roadmap item adds variants or fields
to them. The cost of sealing differs by shape: on the enums (`NormalizedBeat`,
`Line`) `#[non_exhaustive]` removes downstream exhaustiveness checking, forcing
a catch-all arm over a set of cases that is in fact closed; on the structs
(`ParseError`, `UnplayablePitch`) it blocks struct-literal construction and
forces `..` in destructuring, for types a consumer only ever reads back. Absent
a concrete field to add, neither cost buys anything. `TabError` and `TuningName`
keep their seals because they genuinely grow (new error conditions, new tuning
presets); the output types do not.

## Consequences

- Rust callers replace `TabInput { ... }` literals with `TabInput::new(...)`
  plus `with_*` setters. The crate doctest, the example, and the public-surface
  canary are updated; in-crate tests keep their literals, since same-crate
  construction is unaffected by `#[non_exhaustive]`.
- JS callers see no change; the tsify-generated TypeScript interface is
  identical.
- Future optional fields on `TabInput` (custom tuning first) are additive: a
  new `with_*` setter plus a default in `new`, shipped in a minor release.
