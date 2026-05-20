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
        "input": "E2",
        "tuningName": "standard",
        "guitarNumFrets": 18,
        "guitarCapo": 0,
        "numArrangements": 1,
        "maxFretSpanFilter": null
    }"#;
    let parsed: TabInput = serde_json::from_str(json).expect("camelCase deserialization");
    assert_eq!(parsed.num_arrangements, 1);
    assert_eq!(parsed.guitar_num_frets, 18);
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
