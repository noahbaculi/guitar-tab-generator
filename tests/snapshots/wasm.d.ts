/* tslint:disable */
/* eslint-disable */
/**
 * A pitch that could not be played on the configured guitar, with its 1-indexed line number.
 *
 * Public payload of [`TabError::UnplayablePitches`]; the structured `{ value, line }` record replaced the free-form prose string used for unplayable pitches before 2.0.0.
 */
export interface UnplayablePitch {
    value: string;
    line: number;
}

/**
 * Configuration bundle for one tab-generation request.
 *
 * Crosses the WASM boundary via `tsify_next`; JS sees a camelCase interface generated
 * alongside the `.wasm`. `num_arrangements` must be in `1..=NumArrangements::MAX`; the value is validated
 * at the boundary and a [`TabError::NumArrangementsOutOfRange`] is thrown when out of range.
 */
export interface TabInput {
    input: string;
    /**
     * Name of the tuning preset. Accepts the case-insensitive literal `\"standard\"` for
     * standard tuning, or any variant of `TuningName` (case-insensitive, camelCase on the
     * wire: `\"openG\"`, `\"dropD\"`, etc.). Other strings (including the empty string) are
     * rejected with [`TabError::TuningNameUnknown`].
     */
    tuningName: string;
    guitarNumFrets: number;
    guitarCapo: number;
    numArrangements: number;
    /**
     * Upper bound on per-beat fret span. An aggressive value can drop the set to zero
     * arrangements; callers receive `Ok(set)` with `set.len == 0`, not `Err`.
     */
    maxFretSpanFilter?: number;
}

/**
 * Named tuning presets. Parsed case-insensitively from strings.
 *
 * Additional variants may be added in a non-breaking release; the `#[non_exhaustive]`
 * attribute requires external matches to include a wildcard arm.
 */
export type TuningName = "openG" | "openD" | "c6" | "dsus4" | "dropD" | "dropC" | "openC" | "dropB" | "openE";

/**
 * One beat in the normalized input echoed back from `ArrangementSet::normalized_input`.
 *
 * Serialized as a discriminated union tagged by `kind`, so JS code can `switch (b.kind)`
 * instead of comparing strings.
 */
export type NormalizedBeat = { kind: "playable"; pitches: string[] } | { kind: "rest" } | { kind: "measureBreak" };

/**
 * One unparseable substring in the input, with its 1-indexed line number.
 */
export interface ParseError {
    line: number;
    text: string;
}

/**
 * Top-level error variant for the WASM boundary.
 */
export type TabError = { kind: "parse"; errors: ParseError[] } | { kind: "numFretsTooHigh"; numFrets: number; max: number } | { kind: "capoTooHigh"; capo: number; max: number } | { kind: "capoExceedsFrets"; capo: number; numFrets: number } | { kind: "stringNumberOutOfRange"; value: number; max: number } | { kind: "openPitchOutOfRange"; string: number; semitones: number } | { kind: "fretRangeExceedsPitchRange"; openPitch: string; playableFrets: number } | { kind: "unplayablePitches"; pitches: UnplayablePitch[] } | { kind: "noArrangementsFound" } | { kind: "numArrangementsOutOfRange"; value: number; max: number } | { kind: "tuningNameUnknown"; value: string } | { kind: "indexOutOfBounds"; index: number; len: number } | { kind: "renderWidthTooSmall"; width: number; min: number };


/**
 * Opaque handle holding the result of one `generate_arrangements` call.
 *
 * Owns the arrangements, the guitar configuration, and the normalized input shared across
 * arrangements. Per-arrangement metadata (`difficulty`, `max_fret_span`) and the rendered
 * tab string are reached by index through methods on the handle.
 */
export class ArrangementSet {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Difficulty score for the arrangement at `index`. Lower is easier.
     */
    difficulty(index: number): number;
    /**
     * Largest non-zero fret span across any beat in the arrangement at `index`.
     */
    maxFretSpan(index: number): number;
    /**
     * Renders the arrangement at `index` at the supplied `width`, `padding`, and optional
     * `playback` beat indicator. Cheap to call repeatedly with different render parameters
     * -- pathfinding does not re-run.
     */
    render(index: number, width: number, padding: number, playback?: number | null): string;
    /**
     * Returns true when `len == 0`.
     */
    readonly isEmpty: boolean;
    /**
     * Number of arrangements in the set. Equal to the requested `num_arrangements`, possibly
     * reduced by `max_fret_span_filter` when filtering would otherwise drop below the count.
     */
    readonly len: number;
    /**
     * The per-beat input echoed back as a sequence of tagged `NormalizedBeat` variants.
     * Shared across all arrangements; lives once on the set.
     *
     * Returns a fresh `Vec` on each call; cache on the JS side if reading repeatedly.
     * `examples/wasm.html` caches the result on `state.normalizedInput` and reads from that
     * cache in the rerender path; that pattern is the intended consumer shape.
     */
    readonly normalizedInput: NormalizedBeat[];
}

/**
 * Generates an `ArrangementSet` from a `TabInput`. Single entry point for both Rust callers
 * and the WASM boundary; JS sees this as `generateArrangements`.
 *
 * # Errors
 *
 * Returns the typed [`TabError`] variant for each failure mode reachable from this entry point:
 *
 * - Input-shape validation: [`TabError::NumArrangementsOutOfRange`], [`TabError::TuningNameUnknown`],
 *   [`TabError::NumFretsTooHigh`], [`TabError::CapoTooHigh`], [`TabError::CapoExceedsFrets`].
 * - Parser: [`TabError::Parse`] (carries `Vec<ParseError>` with line/text per unparseable substring).
 * - Pathfinding: [`TabError::UnplayablePitches`] (one or more pitches reach no string),
 *   [`TabError::NoArrangementsFound`] (every pitch reaches the guitar but no valid combination exists,
 *   for example duplicate pitches in a single beat that the no-duplicate-strings constraint filters away).
 *
 * [`TabError::OpenPitchOutOfRange`], [`TabError::StringNumberOutOfRange`], and
 * [`TabError::FretRangeExceedsPitchRange`] are members of the enum but are not reachable through this
 * entry point: the preset tunings and fixed 1..=6 string numbering keep every open-string pitch and
 * fret range well inside the supported `Pitch` range. They surface only on the lower-level Rust API
 * ([`Guitar::new`], [`create_string_tuning`]).
 *
 * # Validation order
 *
 * Input-shape errors (currently `numArrangements` range) are reported before `parse_lines` runs.
 * The ordering is deliberate: shape checks are O(1) and unambiguous, while parse errors depend on
 * the full input. When both are present the shape error wins because the parser's output would be
 * discarded anyway.
 *
 * # Performance
 *
 * `tab_input.input` is cloned once per call because `parse_lines` is `#[memoize]`d on owned
 * `String`. Memoization makes a repeat call with the same input cheap, but the clone runs
 * on every call (including cache hits). Hot loops over `generate_arrangements` should expect
 * one `String::clone` per invocation in addition to the boundary deserialization cost.
 */
export function generateArrangements(tab_input: TabInput): ArrangementSet;

/**
 * Returns the supported `TuningName` variants, typed for JS consumption via tsify.
 */
export function getTuningNames(): TuningName[];

