# 0007: Flat TabError variants

Status: accepted
Date: 2026-05-20

## Context

The 2.0.0 preview surface kept three umbrella `TabError` variants from the
original 1.x shape: `Guitar { message }`, `Arrangement { message }`, and
`InvalidInput { field, message }`. The `message` field was prose, not a
typed wire field, which forced JS callers to fall back to string
inspection for anything more granular than the umbrella kind.
`error.rs:41` documented this explicitly: "treat it like UI strings, not
like a stable wire field."

With 2.0.0 about to ship, the umbrella shape would have to be removed
behind a major bump if structured payloads were added later. The window
to flatten was now.

## Decision

`TabError` is a flat tagged union. Each concrete failure mode is its own
variant with a structured payload. The umbrella variants are removed.
The initial variant set (twelve kinds including the unchanged `Parse`)
captures every error path currently reachable from `generate_arrangements`
and the public Rust API:

- `Parse { errors: Vec<ParseError> }`
- `NumFretsTooHigh { num_frets, max }`
- `CapoTooHigh { capo, max }`
- `CapoExceedsFrets { capo, num_frets }`
- `StringNumberOutOfRange { value, max }`
- `OpenPitchOutOfRange { string, semitones }`
- `FretRangeExceedsPitchRange { open_pitch, playable_frets }`
- `UnplayablePitches { pitches: Vec<UnplayablePitch> }`
- `NoArrangementsFound`
- `NumArrangementsOutOfRange { value, max }`
- `TuningNameUnknown { value }`
- `IndexOutOfBounds { index, len }`

The variant count grew by one (`NoArrangementsFound`) during implementation.
The original plan called for a `panic!` on the empty-`path_results` path in
`arrangement::create_arrangements`. Internal proptests (`proptest-regressions/arrangement.txt`)
shrink to duplicate-pitch beats such as `Playable([E2, E2])`, which the
`no_duplicate_strings` constraint filters to zero candidate fingerings even
though every individual pitch is playable. That is valid public input, not
a BUG, so the path returns a structured error instead. The boundary test
`integration_public_surface::boundary_variant_smoke::no_arrangements_found`
pins this guarantee.

The enum stays `#[non_exhaustive]`, so new variants can be added in 2.x
without a major bump. The grouped alternative (Guitar/Arrangement
sub-enums) was rejected because it preserved the umbrella indirection
that this decision exists to remove; the flat shape matches the existing
flat `Parse` variant and the JS-side `switch (err.kind)` pattern the
demo already uses.

`NoArrangementsFound` carries no payload because the failure mode does
not have call-site context the variant could surface: it fires when the
pathfinding graph through `multi_cartesian_product` plus
`no_duplicate_strings` produces no valid sequence for an input whose
individual pitches all reach the guitar. Internal proptests reach this
state with valid-looking random input, so it is not a panic-worthy BUG.

## Consequences

- JS callers extend their `switch (err.kind)` blocks. The Tsify wire
  shape is the tagged object only; there is no free-form `message`
  field on the catch-all. UIs that previously rendered `err.message`
  build a per-kind string from the structured fields, or fall through
  to a default handler.
- `UnplayablePitch` becomes a public type. Its prior home as a private
  struct in `arrangement.rs` is gone.
- Removing the umbrella variants required removing `anyhow` from public
  Rust signatures so the typed errors do not get re-wrapped. See ADR-0007's
  companion changes in the 2.0.0 final-pass commits.
- `Pitch::plus_offset` returns `Option<Pitch>` rather than `Result<_, TabError>`
  because the math has no context to populate `OpenPitchOutOfRange`. The
  caller has the string number and offset, the function does not.
