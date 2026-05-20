/* tslint:disable */
/* eslint-disable */
/**
 * Configuration bundle for one tab-generation request.
 *
 * Crosses the WASM boundary via `tsify_next`; JS sees a camelCase interface generated
 * alongside the `.wasm`. `num_arrangements` must be in `1..=NumArrangements::MAX`; the value is validated
 * at the boundary and a `TabError::InvalidInput` is thrown when out of range.
 */
export interface TabInput {
    input: string;
    /**
     * Name of the tuning preset. Accepts the empty string and the case-insensitive literal
     * `\"standard\"` for standard tuning, or any variant of `TuningName` (case-insensitive,
     * camelCase on the wire: `\"openG\"`, `\"dropD\"`, etc.). Other strings are rejected with
     * `TabError::InvalidInput { field: \"tuningName\", ... }`.
     */
    tuningName: string;
    guitarNumFrets: number;
    guitarCapo: number;
    numArrangements: number;
    /**
     * Upper bound on per-beat fret span. An aggressive value can drop the set to zero
     * arrangements; callers receive `Ok(set)` with `set.len == 0`, not `Err`.
     */
    maxFretSpanFilter: number | undefined;
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
export type TabError = { kind: "parse"; errors: ParseError[] } | { kind: "guitar"; message: string } | { kind: "arrangement"; message: string } | { kind: "invalidInput"; field: string; message: string };


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
 * Returns `TabError::InvalidInput` when `tab_input.num_arrangements` is outside `1..=NumArrangements::MAX`,
 * `TabError::Parse` when the input contains unparseable substrings, `TabError::Guitar` on invalid tuning
 * or capo or fret count, and `TabError::Arrangement` when no valid fingering exists for a pitch.
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

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_arrangementset_free: (a: number, b: number) => void;
    readonly arrangementset_difficulty: (a: number, b: number, c: number) => void;
    readonly arrangementset_isEmpty: (a: number) => number;
    readonly arrangementset_len: (a: number) => number;
    readonly arrangementset_maxFretSpan: (a: number, b: number, c: number) => void;
    readonly arrangementset_normalizedInput: (a: number, b: number) => void;
    readonly arrangementset_render: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly generateArrangements: (a: number, b: number) => void;
    readonly getTuningNames: (a: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
