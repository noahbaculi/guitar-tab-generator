# Types and Data Flow

Input: `RenderedTabInput`. Output: `Vec<RenderedTab>`.

## Pipeline

```
                        RenderedTabInput
                        ────────────────
  pitches: String │ tuning_name: String │ guitar_num_frets, guitar_capo: u8
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
          │                        ─────────────────────────────────────────
          │                         tuning       : BTreeMap<StringNumber, Pitch>
          │                         num_frets    : u8
          │                         range        : BTreeSet<Pitch>
          │                         string_ranges: BTreeMap<StringNumber, Box<[Pitch]>>
          │                           │
          ▼                           │
Vec<Line<BeatVec<Pitch>>>             │              num_arrangements: u8
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
                 (for each arrangement)
                         ▼
                    render_tab
                 args: &[Line<BeatVec<PitchFingering>>], &Guitar,
                       width: u16, padding: u8, playback: Option<u16>
                         │
                         ▼
                      String  (ASCII tab)
                         │
                         ▼
             RenderedTab { tab, normalized_input: Rc<Vec<BeatVec<String>>>,
                           max_fret_span: u8 }
                         │
                         ▼
                  Vec<RenderedTab>
```

## Types Up Close

```
Vec<Line<BeatVec<Pitch>>>              ← parser output
    │       │        └─ Pitch          (C0..B9, one semitone each)
    │       └─ BeatVec<T> = Vec<T>     (one beat's worth)
    └─ Line<T> = Playable(T) | Rest | MeasureBreak

Vec<Line<BeatVec<PitchFingering>>>     ← arrangement output
    │       │        └─ PitchFingering { pitch, string_number, fret }
    │       └─ BeatVec<T> = Vec<T>
    └─ Line<T> = Playable(T) | Rest | MeasureBreak
```

> `Line<T>` has the same shape in both stages. Only the leaf inside `Playable`
> changes: `Pitch` after parsing, `PitchFingering` after arranging.
