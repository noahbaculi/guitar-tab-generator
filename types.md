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
NormalizedBeat                         <- ArrangementSet.normalized_input element
    kind: "rest" | "measureBreak" | "playable"
    pitches?: string[]                 (present when kind == "playable")

TabError                               <- thrown by build_arrangement_set / generateArrangements
    kind: "parse"        + errors:  ParseError[]
    kind: "guitar"       + message: string
    kind: "arrangement"  + message: string
    kind: "invalidInput" + field:   string, message: string

ParseError
    line: u32
    text: String

TuningName                             <- enum returned by getTuningNames()
    "openG" | "openD" | "c6" | "dsus4" | "dropD"
  | "dropC" | "openC" | "dropB" | "openE"
```

The entry points are:

- `generate_arrangements(input: TabInput) -> Result<ArrangementSet, TabError>` -- WASM entry point (JS name: `generateArrangements`). Validates, builds the guitar, runs the pathfinder, returns the opaque handle.
- `build_arrangement_set(tab_input: TabInput) -> Result<ArrangementSet, TabError>` -- pure-Rust constructor. Same work; used by tests and direct Rust callers.
- `getTuningNames() -> TuningName[]` -- enumerates the supported tuning presets, typed for JS via tsify.

## Lifecycle (JS only)

`ArrangementSet` is a `#[wasm_bindgen]` opaque handle, not a serde-cloned value, so JS callers own the underlying allocation. Release it with `set.free()` when done, or use `using set = generateArrangements(input)` on TS 5.2+ runtimes that wire `[Symbol.dispose]` to `free()`. Relying on `FinalizationRegistry` alone leaks the handle on runtimes whose GC isn't prompt.
