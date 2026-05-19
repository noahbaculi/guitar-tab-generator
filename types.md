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
          ▼                           │              num_arrangements: u8
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

TabError                               <- thrown by generate_arrangements / build_arrangement_set
    kind: "parse" | "noArrangements"
    errors?: ParseError[]              (present when kind == "parse")

ParseError
    line: u32
    text: String
```

The entry points are:

- `generate_arrangements(input: TabInput) -> ArrangementSet` -- validates, builds the guitar, runs the pathfinder, and returns the opaque handle.
- `build_arrangement_set(lines, guitar, num_arrangements, max_fret_span_filter) -> ArrangementSet` -- lower-level constructor used internally and in tests.
