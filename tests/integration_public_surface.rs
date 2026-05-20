//! Integration tests exercising only the public Rust surface of `guitar_tab_generator`.
//!
//! Lives outside the crate (under `tests/`) so the compiler enforces that every used
//! symbol is genuinely `pub` from the crate root. Catches regressions where an
//! internal refactor accidentally tightens visibility.
//!
//! Every re-export listed in `src/lib.rs` should be touched by at least one test in
//! this file. Behaviour coverage lives in unit tests and `examples/advanced.rs`; the
//! tests here are deliberately shallow so a visibility tightening fails to compile
//! rather than fails an assertion.

use guitar_tab_generator::{
    create_arrangements, create_string_tuning, generate_arrangements, get_tuning_names, parse_lines,
    render_tab, Arrangement, ArrangementSet, BeatVec, Guitar, Line, NormalizedBeat,
    NumArrangements, ParseError, Pitch, PitchFingering, StringNumber, TabError, TabInput,
    TuningName,
};

fn fixture(num: u8) -> TabInput {
    TabInput {
        input: "E2\nA2\nD3".to_owned(),
        tuning_name: "standard".to_owned(),
        guitar_num_frets: 18,
        guitar_capo: 0,
        num_arrangements: num,
        max_fret_span_filter: None,
    }
}

#[test]
fn generate_arrangements_happy_path() {
    let set = generate_arrangements(fixture(1)).expect("valid input must produce a set");
    assert_eq!(set.len(), 1);
    assert_eq!(set.max_fret_span(0).unwrap(), 0);
    assert!(set.difficulty(0).is_ok());
}

#[test]
fn render_produces_non_empty_string_with_fret_markers() {
    let set = generate_arrangements(fixture(1)).unwrap();
    let rendered = set.render(0, 30, 2, None).unwrap();
    assert!(!rendered.is_empty(), "rendered tab must not be empty");
    // The fixture uses all open strings; each beat column contains at least one '0'.
    assert!(rendered.contains('0'), "rendered tab must include open-string fret marker");
    assert!(rendered.contains('-'), "rendered tab must include dash fill characters");
}

#[test]
fn tab_input_round_trips_from_camel_case_json() {
    let json = r#"{
        "input": "E2\nA2",
        "tuningName": "openG",
        "guitarNumFrets": 22,
        "guitarCapo": 3,
        "numArrangements": 4,
        "maxFretSpanFilter": 7
    }"#;
    let parsed: TabInput = serde_json::from_str(json).expect("camelCase deserialization");
    assert_eq!(parsed.input, "E2\nA2");
    assert_eq!(parsed.tuning_name, "openG");
    assert_eq!(parsed.guitar_num_frets, 22);
    assert_eq!(parsed.guitar_capo, 3);
    assert_eq!(parsed.num_arrangements, 4);
    assert_eq!(parsed.max_fret_span_filter, Some(7));

    // Null on the wire deserializes to None for the Option<u8> filter field.
    let json_null = r#"{
        "input": "E2",
        "tuningName": "standard",
        "guitarNumFrets": 18,
        "guitarCapo": 0,
        "numArrangements": 1,
        "maxFretSpanFilter": null
    }"#;
    let parsed_null: TabInput =
        serde_json::from_str(json_null).expect("null filter deserialization");
    assert_eq!(parsed_null.max_fret_span_filter, None);
}

#[test]
fn get_tuning_names_returns_non_empty() {
    let names: Vec<TuningName> = get_tuning_names();
    assert!(!names.is_empty(), "tuning name list must include at least one preset");
}

#[test]
fn parse_variant_serializes_with_kind_tag() {
    let err = TabError::Parse {
        errors: vec![ParseError { line: 1, text: "bad".to_owned() }],
    };
    let json = serde_json::to_string(&err).unwrap();
    assert!(json.contains(r#""kind":"parse""#), "missing kind tag in {json}");
    assert!(json.contains(r#""line":1"#), "missing line field in {json}");
}

#[test]
fn arrangement_set_is_empty_when_filter_drops_every_candidate() {
    // C3E3 is an all-fretted chord in standard tuning; max_fret_span_filter=Some(0)
    // drops every candidate, yielding a non-error empty set. This is the path the
    // JS demo surfaces via `set.isEmpty` and the "No arrangements match" message.
    let input = TabInput {
        input: "C3E3".to_owned(),
        tuning_name: "standard".to_owned(),
        guitar_num_frets: 20,
        guitar_capo: 0,
        num_arrangements: 5,
        max_fret_span_filter: Some(0),
    };
    let set = generate_arrangements(input).expect("empty filter result is not an error");
    assert!(set.is_empty(), "set must be empty when no candidate survives the filter");
    assert_eq!(set.len(), 0);
}

#[test]
fn invalid_input_errors_are_equal_for_equal_inputs() {
    let err_a = generate_arrangements(fixture(0)).expect_err("0 must be rejected");
    let err_b = generate_arrangements(fixture(0)).expect_err("0 must be rejected");
    assert_eq!(err_a, err_b, "TabError::InvalidInput must derive structural equality");
    let err_high = generate_arrangements(fixture(99)).expect_err("99 must be rejected");
    assert_ne!(err_a, err_high, "different messages must not compare equal");
}

#[test]
fn arrangement_set_handle_is_publicly_named() {
    // `ArrangementSet` is bound to a name so direct Rust callers can mention it in
    // signatures and trait bounds, even though the canonical access path is method
    // calls on the value returned from `generate_arrangements`.
    let set: ArrangementSet = generate_arrangements(fixture(1)).unwrap();
    assert_eq!(set.len(), 1);
    assert!(!set.is_empty());
}

#[test]
fn normalized_input_variants_are_publicly_constructible() {
    // Matching on `NormalizedBeat` is the documented JS-side discrimination shape;
    // the same enum is the Rust-side return value of `ArrangementSet::normalized_input`.
    let set = generate_arrangements(fixture(1)).unwrap();
    let beats: Vec<NormalizedBeat> = set.normalized_input();
    assert!(beats.iter().any(|b| matches!(b, NormalizedBeat::Playable { .. })));

    // Construct each variant directly to pin its public shape.
    let _playable = NormalizedBeat::Playable {
        pitches: vec!["E2".to_owned()],
    };
    let _rest = NormalizedBeat::Rest;
    let _measure_break = NormalizedBeat::MeasureBreak;
}

#[test]
fn num_arrangements_newtype_round_trips() {
    let valid = NumArrangements::try_new(3).expect("3 is within 1..=MAX");
    assert_eq!(valid.get(), 3);

    let too_high = NumArrangements::try_new(NumArrangements::MAX + 1)
        .expect_err("values above MAX must be rejected");
    assert!(matches!(
        too_high,
        TabError::InvalidInput { ref field, .. } if field == "numArrangements"
    ));

    let zero = NumArrangements::try_new(0).expect_err("0 must be rejected");
    assert!(matches!(
        zero,
        TabError::InvalidInput { ref field, .. } if field == "numArrangements"
    ));
}

#[test]
fn lower_level_pipeline_is_publicly_callable() {
    // Exercises the direct-Rust pipeline that the WASM boundary builds on top of:
    // `parse_lines` -> `Guitar::new` -> `create_arrangements` -> `render_tab`,
    // plus the `Pitch` / `StringNumber` / `PitchFingering` / `BeatVec` / `Line`
    // types they pass between them. Catches accidental visibility tightening on
    // any node of the pipeline.

    let lines: Vec<Line<BeatVec<Pitch>>> =
        parse_lines("E2\nA2\nD3".to_owned()).expect("clean input parses");

    let open_pitches: [Pitch; 6] = [Pitch::E2, Pitch::A2, Pitch::D3, Pitch::G3, Pitch::B3, Pitch::E4];
    let tuning = create_string_tuning(&open_pitches).expect("six valid pitches");
    let guitar = Guitar::new(tuning, 18, 0).expect("standard configuration");

    let n = NumArrangements::try_new(1).unwrap();
    let arrangements: Vec<Arrangement> =
        create_arrangements(guitar.clone(), lines, n, None).expect("playable input");
    assert_eq!(arrangements.len(), 1);

    let arrangement = arrangements.first().expect("at least one arrangement");
    // `lines()` is the public getter (replaces the 1.x `pub` field on `Arrangement`).
    let arrangement_lines: &[Line<BeatVec<PitchFingering>>] = arrangement.lines();
    assert!(!arrangement_lines.is_empty());

    let rendered = render_tab(arrangement_lines, &guitar, 30, 1, None);
    assert!(!rendered.is_empty(), "rendered tab is non-empty for valid input");
    assert!(rendered.contains('0'), "all-open-string fixture renders open-fret markers");
}

#[test]
fn string_number_constructor_is_publicly_callable() {
    // `StringNumber` is part of the boundary because `PitchFingering` carries one;
    // direct Rust callers building custom tunings hit `StringNumber::new`.
    let valid = StringNumber::new(1).expect("string number 1 is valid");
    assert_eq!(valid.get(), 1);
    StringNumber::new(0).expect_err("string number 0 must be rejected");
}

#[test]
fn pitch_fingering_is_publicly_named() {
    // `PitchFingering` is `pub` because it leaks via `Arrangement::lines()`. Its fields
    // are crate-internal by design; consumers read via `Debug`. This test pins the
    // type name and the `Debug` impl as reachable from outside the crate.
    let open_pitches: [Pitch; 6] = [Pitch::E2, Pitch::A2, Pitch::D3, Pitch::G3, Pitch::B3, Pitch::E4];
    let guitar = Guitar::new(create_string_tuning(&open_pitches).unwrap(), 18, 0).unwrap();
    let lines = parse_lines("E2".to_owned()).unwrap();
    let n = NumArrangements::try_new(1).unwrap();
    let arrangements = create_arrangements(guitar, lines, n, None).unwrap();
    let first_beat = arrangements[0]
        .lines()
        .iter()
        .find_map(|line| match line {
            Line::Playable(beat) => beat.first().cloned(),
            _ => None,
        })
        .expect("E2 in standard tuning has at least one fingering");
    let typecheck: PitchFingering = first_beat;
    assert!(!format!("{typecheck:?}").is_empty(), "Debug impl produces output");
}

#[test]
fn tuning_name_variant_is_publicly_constructible() {
    // `TuningName` is `#[non_exhaustive]`, but constructing an existing variant
    // directly must still work for callers that build select-list UIs or test
    // fixtures. The type doesn't derive `PartialEq`; match on it instead.
    let _preset: TuningName = TuningName::OpenG;
    let names = get_tuning_names();
    assert!(names.iter().any(|n| matches!(n, TuningName::OpenG)));
}
