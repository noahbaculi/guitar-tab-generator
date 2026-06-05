# Types and Data Flow

Input: `TabInput`. Output: `ArrangementSet` (opaque handle).

## Pipeline

```
                        TabInput
                        ────────
  input: String │ tuning_name: String │ guitar_num_frets, guitar_capo: u8
          │                  │                        │
          ▼                  ▼                        │
    parse_lines         parse_tuning                  │
          │                  │                        │
          │                  ▼                        │
          │             [i8; 6]                       │
          │                  │                        │
          │                  ▼                        │
          │      create_string_tuning_offset          │
          │                  │                        │
          │                  ▼                        │
          │      BTreeMap<StringNumber, Pitch>        │
          │                  │                        │
          │                  └────────┬───────────────┘
          │                           ▼
          │                      Guitar::new
          │                           │
          │                           ▼
          │                        Guitar
          │                           │
          ▼                           │              num_arrangements: NumArrangements (u8 on JSON wire)
Vec<Line<BeatVec<Pitch>>>             │              max_fret_span_filter: Option<u8>
          │                           │                        │
          └──────────────┬────────────┴────────────────────────┘
                         ▼
                 create_arrangements
                         │
                         ▼
                Vec<Arrangement>
                ─────────────────────────────────────────────────
                 lines        : Vec<Line<BeatVec<PitchFingering>>>
                 difficulty   : i32
                 max_fret_span: u8
                         │
                         ▼
               ArrangementSet (handle)
               ────────────────────────
                arrangements      : Vec<Arrangement>
                guitar            : Guitar
                normalized_input  : Vec<NormalizedBeat>

  per-arrangement reach: set.render(i, width, padding, playback) -> String
                         set.max_fret_span(i) -> u8
                         set.difficulty(i) -> i32
```

> `parse_lines`, `parse_tuning`, and `create_string_tuning_offset` are crate-internal stages,
> not part of the stable public API. They are surfaced only through the `#[doc(hidden)]`
> `__bench_internals` module, for benchmarks.

## Types Up Close

```
Vec<Line<BeatVec<Pitch>>>              <- parser output
    |       |        +- Pitch          (C0..B9, one semitone each)
    |       +- BeatVec<T> = Vec<T>     (one beat's worth)
    +- Line<T> = Playable(T) | Rest | MeasureBreak

Vec<Line<BeatVec<PitchFingering>>>     <- arrangement output
    |       |        +- PitchFingering { pitch, string_number, fret }
    |       +- BeatVec<T> = Vec<T>
    +- Line<T> = Playable(T) | Rest | MeasureBreak
```

> `Line<T>` has the same shape in both stages. Only the leaf inside `Playable`
> changes: `Pitch` after parsing, `PitchFingering` after arranging.

```
NormalizedBeat                         <- ArrangementSet.normalizedInput element
    { kind: "playable", pitches: string[] }
  | { kind: "rest" }
  | { kind: "measureBreak" }

TabError                                  <- thrown by generate_arrangements (JS: generateArrangements)
    kind: "parse"                      + errors: ParseError[]
    kind: "numFretsTooHigh"            + numFrets: number, max: number
    kind: "capoTooHigh"                + capo: number, max: number
    kind: "capoExceedsFrets"           + capo: number, numFrets: number
    kind: "stringNumberOutOfRange"     + value: number, max: number          (lower-level Rust API only)
    kind: "openPitchOutOfRange"        + string: number, semitones: number   (lower-level Rust API only)
    kind: "fretRangeExceedsPitchRange" + openPitch: string, playableFrets: number  (lower-level Rust API only)
    kind: "unplayablePitches"          + pitches: UnplayablePitch[]
    kind: "noArrangementsFound"
    kind: "numArrangementsOutOfRange"  + value: number, max: number
    kind: "tuningNameUnknown"          + value: string
    kind: "indexOutOfBounds"           + index: number, len: number
    kind: "renderWidthTooSmall"        + width: number, min: number

ParseError
    line: number
    text: string

TuningName                             <- enum returned by getTuningNames()
    "openG" | "openD" | "c6" | "dsus4" | "dropD"
  | "dropC" | "openC" | "dropB" | "openE"
```

The entry points are:

- `generate_arrangements(tab_input: TabInput) -> Result<ArrangementSet, TabError>` -- entry point for both Rust and WASM callers (JS name: `generateArrangements`). Validates, builds the guitar, runs the pathfinder, returns the opaque handle.
- `getTuningNames() -> TuningName[]` -- enumerates the supported tuning presets, typed for JS via tsify.

## Lifecycle (JS only)

`ArrangementSet` is a `#[wasm_bindgen]` opaque handle, not a serde-cloned value, so JS callers own the underlying allocation. Release it with `set.free()` when done, or use `using set = generateArrangements(input)` on TS 5.2+ runtimes that wire `[Symbol.dispose]` to `free()`. Relying on `FinalizationRegistry` alone leaks the handle on runtimes whose GC isn't prompt.
