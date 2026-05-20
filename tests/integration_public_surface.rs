//! Integration tests exercising only the public Rust surface of `guitar_tab_generator`.
//!
//! Lives outside the crate (under `tests/`) so the compiler enforces that every used
//! symbol is genuinely `pub` from the crate root. Catches regressions where an
//! internal refactor accidentally tightens visibility.

use guitar_tab_generator::{
    build_arrangement_set, get_tuning_names, ParseError, TabError, TabInput, TuningName,
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
fn build_arrangement_set_happy_path() {
    let set = build_arrangement_set(fixture(1)).expect("valid input must produce a set");
    assert_eq!(set.len(), 1);
    assert_eq!(set.max_fret_span(0).unwrap(), 0);
    assert!(set.difficulty(0).is_ok());
}

#[test]
fn render_produces_non_empty_string_with_fret_markers() {
    let set = build_arrangement_set(fixture(1)).unwrap();
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
    let set = build_arrangement_set(input).expect("empty filter result is not an error");
    assert!(set.is_empty(), "set must be empty when no candidate survives the filter");
    assert_eq!(set.len(), 0);
}

#[test]
fn invalid_input_errors_are_equal_for_equal_inputs() {
    let err_a = build_arrangement_set(fixture(0)).expect_err("0 must be rejected");
    let err_b = build_arrangement_set(fixture(0)).expect_err("0 must be rejected");
    assert_eq!(err_a, err_b, "TabError::InvalidInput must derive structural equality");
    let err_high = build_arrangement_set(fixture(99)).expect_err("99 must be rejected");
    assert_ne!(err_a, err_high, "different messages must not compare equal");
}
