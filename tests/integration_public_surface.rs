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
//!
//! Host-only: these use the default libtest harness. The WASM boundary lane lives in
//! `tests/wasm_boundary.rs`; this file is empty on `wasm32` so `wasm-pack test` skips it.
#![cfg(not(target_arch = "wasm32"))]

use guitar_tab_generator::{
    Arrangement, ArrangementSet, BeatVec, Guitar, Line, NormalizedBeat, NumArrangements,
    ParseError, Pitch, PitchFingering, StringNumber, TabError, TabInput, TuningName,
    UnplayablePitch, create_arrangements, create_string_tuning, generate_arrangements,
    get_tuning_names, parse_lines, render_tab,
};

fn fixture(num: u8) -> TabInput {
    TabInput::new("E2\nA2\nD3", "standard", 18, 0, num)
}

#[test]
fn generate_arrangements_happy_path() {
    let set = generate_arrangements(fixture(1)).expect("valid input must produce a set");
    assert_eq!(set.len(), 1);
    assert_eq!(set.max_fret_span(0).unwrap(), 0);
    // All-open fixture: optimal difficulty is 0. Pin the value, not just Ok-ness.
    assert_eq!(set.difficulty(0).unwrap(), 0);
}

#[test]
fn render_produces_non_empty_string_with_fret_markers() {
    let set = generate_arrangements(fixture(1)).unwrap();
    let rendered = set.render(0, 30, 2, None).unwrap();
    assert!(!rendered.is_empty(), "rendered tab must not be empty");
    // The fixture uses all open strings; each beat column contains at least one '0'.
    assert!(
        rendered.contains('0'),
        "rendered tab must include open-string fret marker"
    );
    assert!(
        rendered.contains('-'),
        "rendered tab must include dash fill characters"
    );
}

#[test]
fn render_is_stateless_across_repeated_calls() {
    let set = generate_arrangements(fixture(1)).unwrap();
    let first = set.render(0, 30, 2, None).unwrap();
    let second = set.render(0, 30, 2, None).unwrap();
    assert_eq!(
        first, second,
        "repeated render with identical parameters must return identical output"
    );
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
    assert!(
        !names.is_empty(),
        "tuning name list must include at least one preset"
    );
}

#[test]
fn parse_variant_serializes_with_kind_tag() {
    let err = TabError::Parse {
        errors: vec![ParseError {
            line: 1,
            text: "bad".to_owned(),
        }],
    };
    let json = serde_json::to_string(&err).unwrap();
    assert!(
        json.contains(r#""kind":"parse""#),
        "missing kind tag in {json}"
    );
    assert!(json.contains(r#""line":1"#), "missing line field in {json}");
}

#[test]
fn arrangement_set_is_empty_when_filter_drops_every_candidate() {
    // C3E3 is an all-fretted chord in standard tuning; max_fret_span_filter=Some(0)
    // drops every candidate, yielding a non-error empty set. This is the path the
    // JS demo surfaces via `set.isEmpty` and the "No arrangements match" message.
    let input = TabInput::new("C3E3", "standard", 20, 0, 5).with_max_fret_span_filter(0);
    let set = generate_arrangements(input).expect("empty filter result is not an error");
    assert!(
        set.is_empty(),
        "set must be empty when no candidate survives the filter"
    );
    assert_eq!(set.len(), 0);
}

#[test]
fn invalid_input_errors_are_equal_for_equal_inputs() {
    let err_a = generate_arrangements(fixture(0)).expect_err("0 must be rejected");
    let err_b = generate_arrangements(fixture(0)).expect_err("0 must be rejected");
    assert_eq!(err_a, err_b, "TabError must derive structural equality");
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
    assert!(
        beats
            .iter()
            .any(|b| matches!(b, NormalizedBeat::Playable { .. }))
    );

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
        TabError::NumArrangementsOutOfRange { value: 21, max: 20 }
    ));

    let zero = NumArrangements::try_new(0).expect_err("0 must be rejected");
    assert!(matches!(
        zero,
        TabError::NumArrangementsOutOfRange { value: 0, max: 20 }
    ));
}

#[test]
fn public_max_consts_are_reachable() {
    // Pin the `pub const` bounds from outside the crate so a refactor that tightens their
    // visibility fails to compile here instead of silently dropping them from the surface.
    assert_eq!(Guitar::MAX_NUM_FRETS, 30);
    assert_eq!(Guitar::MAX_CAPO, 8);
    assert_eq!(StringNumber::MAX, 12);
    assert_eq!(NumArrangements::MAX, 20);
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

    let open_pitches: [Pitch; 6] = [
        Pitch::E2,
        Pitch::A2,
        Pitch::D3,
        Pitch::G3,
        Pitch::B3,
        Pitch::E4,
    ];
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
    assert!(
        !rendered.is_empty(),
        "rendered tab is non-empty for valid input"
    );
    assert!(
        rendered.contains('0'),
        "all-open-string fixture renders open-fret markers"
    );
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
    // are crate-internal, but the three getters (`string_number`, `fret`, `pitch`) give
    // structured read access for downstream callers building per-arrangement fingering
    // inspectors without re-running pathfinding.
    // String 1 is the highest pitch (E4); string 6 is the lowest (E2). See CONTEXT.md.
    let open_pitches: [Pitch; 6] = [
        Pitch::E4,
        Pitch::B3,
        Pitch::G3,
        Pitch::D3,
        Pitch::A2,
        Pitch::E2,
    ];
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

    // E2 in standard tuning is the open low-E string (string 6, fret 0).
    let s: StringNumber = typecheck.string_number();
    assert_eq!(s.get(), 6, "E2 in standard tuning sits on string 6");
    assert_eq!(
        typecheck.fret(),
        0,
        "E2 in standard tuning is the open string"
    );
    let p: Pitch = typecheck.pitch();
    assert_eq!(p, Pitch::E2);

    // Debug impl remains reachable for ad-hoc inspection.
    assert!(
        !format!("{typecheck:?}").is_empty(),
        "Debug impl produces output"
    );
}

#[test]
fn unplayable_pitch_is_nameable_from_crate_root() {
    // `UnplayablePitch` is the element type of `TabError::UnplayablePitches`; it must be
    // nameable in a downstream signature, not just readable as a field value.
    let err = generate_arrangements(TabInput::new("A1", "standard", 18, 0, 1)).unwrap_err();
    let pitches = match err {
        TabError::UnplayablePitches { pitches } => pitches,
        other => panic!("expected UnplayablePitches, got {other:?}"),
    };
    let first: &UnplayablePitch = &pitches[0];
    assert_eq!(first.value, "A1");
    assert_eq!(first.line, 1);
}

#[test]
fn tuning_name_variant_is_publicly_constructible() {
    // `TuningName` is `#[non_exhaustive]`, but constructing an existing variant
    // directly must still work for callers that build select-list UIs or test
    // fixtures.
    let _preset: TuningName = TuningName::OpenG;
    let names = get_tuning_names();
    assert!(names.contains(&TuningName::OpenG));
}

#[cfg(test)]
mod boundary_variant_smoke {
    use guitar_tab_generator::{TabError, TabInput, generate_arrangements};

    fn input(num_frets: u8, capo: u8, num_arrangements: u8, tuning: &str, input: &str) -> TabInput {
        TabInput::new(input, tuning, num_frets, capo, num_arrangements)
    }

    #[test]
    fn num_frets_too_high() {
        let err = generate_arrangements(input(31, 0, 1, "standard", "E2")).unwrap_err();
        assert!(
            matches!(
                err,
                TabError::NumFretsTooHigh {
                    num_frets: 31,
                    max: 30
                }
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn capo_too_high() {
        let err = generate_arrangements(input(18, 9, 1, "standard", "E2")).unwrap_err();
        assert!(
            matches!(err, TabError::CapoTooHigh { capo: 9, max: 8 }),
            "got {err:?}"
        );
    }

    #[test]
    fn capo_exceeds_frets() {
        let err = generate_arrangements(input(2, 4, 1, "standard", "E2")).unwrap_err();
        assert!(
            matches!(
                err,
                TabError::CapoExceedsFrets {
                    capo: 4,
                    num_frets: 2
                }
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn num_arrangements_out_of_range() {
        let err = generate_arrangements(input(18, 0, 0, "standard", "E2")).unwrap_err();
        assert!(
            matches!(
                err,
                TabError::NumArrangementsOutOfRange { value: 0, max: 20 }
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn tuning_name_unknown_empty_string() {
        let err = generate_arrangements(input(18, 0, 1, "", "E2")).unwrap_err();
        match err {
            TabError::TuningNameUnknown { value } => assert_eq!(value, ""),
            other => panic!("expected TuningNameUnknown, got {other:?}"),
        }
    }

    #[test]
    fn tuning_name_unknown_garbage() {
        let err = generate_arrangements(input(18, 0, 1, "openZ", "E2")).unwrap_err();
        match err {
            TabError::TuningNameUnknown { value } => assert_eq!(value, "openZ"),
            other => panic!("expected TuningNameUnknown, got {other:?}"),
        }
    }

    #[test]
    fn bad_tuning_on_valid_multibeat_input_reports_tuning_name_unknown() {
        // Guards the generate_arrangements ordering: the guitar config is validated before
        // the normalized-input vector is built, but parse_lines still runs first. A valid
        // multi-beat pitch list with an unknown tuning must still surface TuningNameUnknown.
        let err = generate_arrangements(input(18, 0, 1, "openZ", "E2\nA2\nD3")).unwrap_err();
        match err {
            TabError::TuningNameUnknown { value } => assert_eq!(value, "openZ"),
            other => panic!("expected TuningNameUnknown, got {other:?}"),
        }
    }

    #[test]
    fn parse_error() {
        let err = generate_arrangements(input(18, 0, 1, "standard", "E2\n???")).unwrap_err();
        match err {
            TabError::Parse { errors } => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].line, 2);
                assert_eq!(errors[0].text, "???");
            }
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn unplayable_pitches() {
        let err = generate_arrangements(input(18, 0, 1, "standard", "A1")).unwrap_err();
        match err {
            TabError::UnplayablePitches { pitches } => {
                assert_eq!(pitches.len(), 1);
                assert_eq!(pitches[0].value, "A1");
                assert_eq!(pitches[0].line, 1);
            }
            other => panic!("expected UnplayablePitches, got {other:?}"),
        }
    }

    #[test]
    fn index_out_of_bounds() {
        let set = generate_arrangements(input(18, 0, 1, "standard", "E2")).unwrap();
        let err = set.render(99, 30, 1, None).unwrap_err();
        assert!(
            matches!(err, TabError::IndexOutOfBounds { index: 99, len: 1 }),
            "got {err:?}"
        );
        // All three indexed handle methods share the bounds guard; pin each at the boundary.
        assert!(
            matches!(
                set.max_fret_span(99).unwrap_err(),
                TabError::IndexOutOfBounds { index: 99, len: 1 }
            ),
            "max_fret_span out-of-bounds must report IndexOutOfBounds"
        );
        assert!(
            matches!(
                set.difficulty(99).unwrap_err(),
                TabError::IndexOutOfBounds { index: 99, len: 1 }
            ),
            "difficulty out-of-bounds must report IndexOutOfBounds"
        );
    }

    #[test]
    fn render_width_too_small() {
        let set = generate_arrangements(input(18, 0, 1, "standard", "E2")).unwrap();
        // padding 1 -> min width = 2*1 + 2 + 1 = 5; width 3 is below it.
        let err = set.render(0, 3, 1, None).unwrap_err();
        assert!(
            matches!(err, TabError::RenderWidthTooSmall { width: 3, min: 5 }),
            "got {err:?}"
        );
    }

    /// Duplicate pitches in a single beat are individually playable but produce no
    /// valid arrangement because the `no_duplicate_strings` constraint filters every
    /// candidate fingering combination. This is the failure mode the proptest seeds
    /// in `proptest-regressions/arrangement.txt` shrink to (e.g. `Playable([E2, E2])`).
    ///
    /// Pinning the variant at the public boundary keeps the ADR-0007 justification
    /// for `NoArrangementsFound` honest: the variant is reachable from valid public
    /// input, so it earns its place in the flat `TabError` union.
    #[test]
    fn no_arrangements_found() {
        let err = generate_arrangements(input(18, 0, 1, "standard", "E2E2")).unwrap_err();
        assert!(matches!(err, TabError::NoArrangementsFound), "got {err:?}");
    }

    #[test]
    fn input_beyond_max_lines_returns_input_too_many_lines() {
        // One line past the u16 beat-index limit (65,535) fails fast as InputTooManyLines at
        // the public boundary instead of overflowing the index and corrupting the result.
        let huge = "A2\n".repeat(u16::MAX as usize + 1);
        let err = generate_arrangements(input(18, 0, 1, "standard", &huge)).unwrap_err();
        match err {
            TabError::InputTooManyLines { max } => {
                assert_eq!(max, u16::MAX as u32);
            }
            other => panic!("expected InputTooManyLines, got {other:?}"),
        }
    }
}
