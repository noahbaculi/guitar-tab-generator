# DifficultyWeights newtype exposes the scoring coefficients, lifted at the boundary

Status: accepted
Date: 2026-06-11

The three difficulty-scoring coefficients (movement, span, position) were baked into `calculate_node_difficulty` as the literals `100.0`, `10.0`, and `1.0`. They are now a `DifficultyWeights` newtype that callers can override per `generate_arrangements` call. `TabInput` carries an optional `DifficultyWeightsInput` (plain `f64` fields) on the wire, and `generate_arrangements` validates and lifts it into `DifficultyWeights` at the boundary, the same shape [ADR-0005](0005-num-arrangements-newtype.md) uses for `num_arrangements`. Omitting the field reproduces the previous `100 / 10 / 1` ranking.

## Considered Options

- **Pass three raw `f64`s through the pipeline.** Rejected for the same reason as the raw-`u8` count in ADR-0005: every function in the chain would accept any `f64`, including negatives and `NaN`, and the validation would have no single home.
- **A `DifficultyWeights` newtype validated in `try_new`.** The type guarantees each coefficient is finite and non-negative. `create_arrangements` cannot run with an invalid weight without going through the constructor. Picked.

## Decision details

- **`f64` on the public surface.** A JavaScript `number` is an IEEE-754 double, so `f64` crosses the `tsify` boundary with no conversion and an honest TypeScript `number`. The difficulty math is already `f64` (`avg_non_zero_fret` is an `OrderedFloat<f64>`), so the weights drop straight into the scoring expression with no cast. `f32` would silently narrow, and an integer type would reject a fractional weight like `100.5` with an opaque serde error rather than a structured `TabError`.
- **`OrderedFloat<f64>` for the internal fields.** `create_arrangements` is memoized, so every argument must implement `Hash` and `Eq`, which `f64` does not. The fields are stored as `OrderedFloat<f64>` (the crate's existing idiom for hashable floats, already used by `ScoredBeatFingering`) so the type derives `Hash`/`Eq` and joins the memoize key. The public surface stays plain `f64`: `try_new` takes `f64`, the accessors return `f64`, and `try_new` rejects non-finite inputs so the derives never observe a `NaN`.
- **The error variant omits the offending value.** `TabError::DifficultyWeightOutOfRange` carries only `field`, not the value. `TabError` derives `Eq` (pinned by `tests/integration_public_surface.rs`), which an `f64` field would break, and `OrderedFloat` would break the `Tsify` derive instead. The caller already holds the value it passed, so the field name is the new information worth surfacing.
- **No upper bound on the coefficients.** `try_new` validates only finite and non-negative. Only the ratio of the coefficients affects ranking, so a large absolute value is harmless. (Originally bounded by `MAX = 10_000.0` to keep the `i32` difficulty cast from overflowing; see the amendment below.)

## Consequences

- The weight-range invariant lives in `DifficultyWeights::try_new` and nowhere else. `try_new` returns `TabError::DifficultyWeightOutOfRange { field }`, shared by direct Rust callers and WASM callers.
- `TabInput.difficulty_weights: Option<DifficultyWeightsInput>` stays a plain optional record on the wire, lifted to `DifficultyWeights` inside `generate_arrangements`. `None` means `DifficultyWeights::standard()`.
- Direct Rust callers pass `DifficultyWeights` to `create_arrangements`, constructing it via `try_new` or `standard`.

## Amendment 2026-06-12: float difficulty cost

The internal pathfinding cost changed from `i32` to `OrderedFloat<f64>`, completing this ADR's "the difficulty math is already `f64`" reasoning by removing the one remaining `as i32` cast in `calculate_node_difficulty`. The cast truncated each per-beat score, which made the absolute magnitude of the weights affect ranking even though, mathematically, only their ratios should. With the cast gone, magnitude no longer matters.

`Arrangement` stores the score as `OrderedFloat<f64>` (it derives `Eq`, which `f64` does not satisfy) and exposes plain `f64` through `Arrangement::difficulty` and `ArrangementSet::difficulty`, the same store-as-`OrderedFloat`, expose-as-`f64` shape these weights use. Across the WASM boundary the accessor stays `number`, so the `.d.ts` surface is unchanged.

The `MAX = 10_000.0` bound existed only to keep the `i32` cast from overflowing, so it was removed along with the cast. `try_new` now validates finite and non-negative.
