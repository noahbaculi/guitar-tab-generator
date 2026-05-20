# Guitar Tab Generator

A Rust library (with WASM bindings) that turns a newline-delimited list of pitches into one or more rendered ASCII guitar tabs, ranked by playing difficulty.

## Language

**TabInput**:
The bundle of configuration a caller hands across the WASM boundary to generate tabs: the raw `input` text, tuning, guitar parameters, render parameters, and `num_arrangements`. Drives one [[ArrangementSet]] per call.
_Avoid_: CompositionInput (former name, renamed in 2.0.0), TabRequest, GenerateTabInput

**ArrangementSet**:
The WASM-facing handle returned from a generate call. Holds all [[Arrangement]]s for one [[TabInput]] together with the [[Normalized input]] they share. Per-arrangement metadata (`difficulty`, `max_fret_span`) and the [[RenderedTab]] for any chosen render parameters are reached by index through methods on the handle.
_Avoid_: TabSet, RenderedTabBundle, GenerationResult

**Arrangement**:
One concrete choice of fingerings for the input pitches, ranked against alternatives by `difficulty`. Held inside an [[ArrangementSet]] and reached by index across the WASM boundary; its `difficulty` and `max_fret_span` are exposed as getter methods on the set, and its [[RenderedTab]] is produced on demand by `set.render(i, ...)`.
_Avoid_: Solution, transcription

**RenderedTab**:
The rendered ASCII tab string produced for one [[Arrangement]] at a chosen `(width, padding, playback)`. Returned from `set.render(i, ...)`; not a struct, just the tab text.
_Avoid_: Composition (former name, renamed in 2.0.0 to avoid clash with the musical sense), Tab (ambiguous between the rendered string and the larger artifact), Output

**Beat**:
A single rhythmic position in the input — one moment in time, either sounding or silent. Beats are the unit the pathfinder, difficulty calculation, and playback indicator reason about. Every beat is also a [[Line]], but not every line is a beat (specifically: `MeasureBreak` is not).
_Avoid_: Step, position, moment, sonorous (a stale synonym; "sonorous beat" wrongly implies non-rest)

**Line**:
One row of the user's input text and (after rendering) one logical row of the output tab. The render-side concept; includes structural variants ([[Beat]]-bearing rows and `MeasureBreak`) that flow through parsing and rendering even though some are filtered out for pathfinding.
_Avoid_: Row, entry

**MeasureBreak**:
A non-[[Beat]] [[Line]] — a bar line drawn in the rendered tab. Filtered out before pathfinding and re-injected for rendering. Carries no rhythmic or musical content; it is a structural divider only.
_Avoid_: Bar, measure (there is no real measure / time-signature concept in this project)

**Difficulty**:
The score being minimized. The canonical word at every layer: the per-[[Beat]] features fed to scoring (the difficulty features), the score on each pathfinding edge (transition difficulty, the cost of moving from one [[Beat]]'s fingering to the next), and the sum along the chosen path (`Arrangement.difficulty`). `pathfinding::yen` internally calls its edge values "weight" — that's a library detail, not domain vocabulary.
_Avoid_: Cost, weight, score

**Difficulty features**:
The per-[[Beat]] stats fed to difficulty scoring, currently `avg_non_zero_fret` and `non_zero_fret_span`. Properties of one beat's chosen fingering, not of a transition.
_Avoid_: Difficulty inputs, stats, metrics

**Transition difficulty**:
The [[Difficulty]] of moving from one [[Beat]]'s fingering to the next adjacent [[Beat]]'s fingering. The value on a pathfinding edge.
_Avoid_: Edge cost, edge weight, step cost

**Pitch fingering**:
The placement of one pitch on one specific (string, fret). The atomic unit of fingering choice.
_Avoid_: Note position, finger placement

**Pitch fingering candidates**:
All the valid [[Pitch fingering]]s for a single pitch on a given guitar — one per string the pitch is reachable on. The set the arranger picks from.
_Avoid_: Pitch fingering options, pitch fingering group

**Beat fingering**:
The chosen [[Pitch fingering]]s for one [[Beat]] — one per pitch in the beat, with no two pitches landing on the same string. The cartesian product of [[Pitch fingering candidates]] across the beat's pitches, filtered for string collisions. When decorated with [[Difficulty features]], it is held as `BeatFingeringCombo`.
_Avoid_: Beat fingering combo (the type name is current shorthand; "combo" suggests "one of many" but the chosen one is just *the* beat fingering), fingering combination

**Tuning**:
The assignment of open-string pitches to a guitar's strings — a map from `StringNumber` to `Pitch`. There is one canonical form (the map); the public API also accepts a **tuning preset** (the `TuningName` enum + the string `"standard"`) which resolves through a fixed table of semitone offsets relative to standard 6-string tuning. The offset array (`[i8; 6]`) is a parsing waypoint, not a separate domain concept.
_Avoid_: Tuning offsets / tuning array as standalone terms (they're encodings of a tuning, not tunings in their own right)

**Fret count**:
The number of physical frets on the instrument — what the caller supplies (`TabInput.guitar_num_frets`). A property of the guitar hardware, independent of capo placement.
_Avoid_: num_frets as a freestanding term (the bare name is currently overloaded with [[Playable fret count]])

**Playable fret count**:
The number of frets above the capo — what `Guitar.playable_frets` holds after construction. Equal to [[Fret count]] minus capo position. The number that bounds fingering search.
_Avoid_: Effective frets, available frets

**Normalized input**:
The input echoed back per-[[Beat]] for the WASM consumer, held once on the [[ArrangementSet]] (not per-[[Arrangement]]) since every arrangement of one [[TabInput]] shares it. A sequence of [[NormalizedBeat]]s. Exists so a UI can align playback or highlights against the rendered tab without re-parsing.
_Avoid_: Pitches (former field name was `pitches` — misleading because the sequence also carries rests and measure breaks; renamed in 2.0.0), input pitches

**NormalizedBeat**:
One entry in the [[Normalized input]]. A tagged variant: `{ kind: "playable", pitches: [...] }`, `{ kind: "rest" }`, or `{ kind: "measureBreak" }`. Replaces the legacy `["REST"]` / `["MEASURE_BREAK"]` string sentinels with a discriminated union so JS consumers can switch on `.kind` instead of string equality.
_Avoid_: Beat entry, NormalizedLine (it carries the [[Line]] structural shape but the canonical user-facing word at this layer is "beat")

**StringNumber**:
A guitar string's index, where **string 1 is the highest-pitched string** (thinnest, e.g. high E on standard tuning) and the largest string number is the lowest-pitched string (thickest, e.g. low E on standard tuning). Standard guitar convention; opposite of programmer-intuitive "index 0 = bass." Tabs render string 1 on top, largest string number on the bottom. The `BTreeMap<StringNumber, Pitch>` iteration order in [[Tuning]] follows the same direction.
_Avoid_: String index (ambiguous about direction)

**Playback cursor**:
A 0-indexed [[Beat]] position passed in by a UI player so the rendered tab can draw `▼`/`▲` indicators above and below the corresponding beat column. Counts beats (Playable and Rest); skips `MeasureBreak`s. Carried as the `playback` parameter on `ArrangementSet::render` and `render_tab`.
_Avoid_: Playhead, current position

## Flagged ambiguities

_None currently._

## Example dialogue

> **Dev:** "We're returning ten rendered tabs for that input — is that the right cap?"
>
> **Domain expert:** "Ten *arrangements*, held in one [[ArrangementSet]]. The set carries the shared normalized input; each arrangement has its own difficulty and max fret span. A rendered tab is just the ASCII string for one arrangement at a chosen width and padding — the demo can call `set.render(i, width, padding, playback)` as many times as it wants without re-pathfinding."
>
> **Dev:** "Right — the cap is on arrangements, not on renders. Width changes are cheap."
