use crate::{
    arrangement::{BeatVec, Line},
    guitar::{create_string_tuning, STD_6_STRING_TUNING_OPEN_PITCHES},
    pitch::Pitch,
    string_number::StringNumber,
};
use itertools::Itertools;
use memoize::memoize;
use regex::{Regex, RegexBuilder};
use serde::Serialize;
use std::{collections::BTreeMap, result::Result::Ok, sync::Arc};
use std::{collections::HashSet, str::FromStr};
use strum::VariantNames;
use strum_macros::{EnumString, VariantNames};
use tsify_next::Tsify;
use wasm_bindgen::prelude::*;

const PITCH_PATTERN: &str =
    r"(?P<three_char_pitch>[A-G][#|♯|b|♭][0-9])|(?P<two_char_pitch>[A-G][0-9])";

#[cfg(test)]
fn test_pitch_regex() -> Regex {
    RegexBuilder::new(PITCH_PATTERN)
        .case_insensitive(true)
        .build()
        .expect("Regex pattern should be valid")
}

/// Named tuning presets. Parsed case-insensitively from strings.
///
/// Additional variants may be added in a non-breaking release; the `#[non_exhaustive]`
/// attribute requires external matches to include a wildcard arm.
#[derive(Debug, EnumString, VariantNames, Serialize, Tsify)]
#[strum(ascii_case_insensitive)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum TuningName {
    OpenG,
    OpenD,
    C6,
    #[strum(serialize = "dsus4", serialize = "dadgad")]
    Dsus4,
    DropD,
    DropC,
    OpenC,
    DropB,
    OpenE,
}

/// Returns the supported `TuningName` variants, typed for JS consumption via tsify.
#[wasm_bindgen(js_name = "getTuningNames")]
pub fn get_tuning_names() -> Vec<TuningName> {
    TuningName::VARIANTS
        .iter()
        .map(|&v| TuningName::from_str(v).expect("BUG: VARIANTS yields parseable strings"))
        .collect()
}

/// Returns the 6-element semitone offsets for a named tuning, relative to standard 6-string
/// tuning.
///
/// Accepts the empty string and the case-insensitive literal `"standard"` as standard tuning
/// (all-zero offsets). Returns `TabError::InvalidInput { field: "tuningName", ... }` for any
/// other string that does not match a `TuningName` variant.
pub fn parse_tuning(tuning_name: &str) -> Result<[i8; 6], crate::error::TabError> {
    match TuningName::from_str(tuning_name) {
        Ok(TuningName::OpenG) => Ok([-2, 0, 0, 0, -2, -2]),
        Ok(TuningName::OpenD) => Ok([-2, 0, 0, -1, -2, -2]),
        Ok(TuningName::C6) => Ok([-4, 0, -2, 0, 1, 0]),
        Ok(TuningName::Dsus4) => Ok([-2, 0, 0, 0, -2, -2]),
        Ok(TuningName::DropD) => Ok([-2, 0, 0, 0, 0, 0]),
        Ok(TuningName::DropC) => Ok([-4, -2, -2, -2, -2, -2]),
        Ok(TuningName::OpenC) => Ok([-4, -2, -2, 0, 1, 0]),
        Ok(TuningName::DropB) => Ok([-5, -3, -3, -3, -3, -3]),
        Ok(TuningName::OpenE) => Ok([0, -2, -2, -2, 0, 0]),
        Err(_) if tuning_name.is_empty() || tuning_name.eq_ignore_ascii_case("standard") => {
            Ok([0; 6])
        }
        Err(_) => Err(crate::error::TabError::InvalidInput {
            field: "tuningName".to_owned(),
            message: format!(
                "must be \"standard\" or one of the supported TuningName variants, got {tuning_name:?}"
            ),
        }),
    }
}
#[cfg(test)]
mod test_parse_tuning {
    use super::*;
    use crate::error::TabError;

    #[test]
    fn standard_tuning_returns_zero_offsets() {
        assert_eq!(parse_tuning("standard").unwrap(), [0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn standard_is_case_insensitive() {
        assert_eq!(parse_tuning("STANDARD").unwrap(), [0, 0, 0, 0, 0, 0]);
        assert_eq!(parse_tuning("Standard").unwrap(), [0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn empty_string_returns_standard() {
        assert_eq!(parse_tuning("").unwrap(), [0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn non_standard_tunings() {
        assert_eq!(parse_tuning("openg").unwrap(), [-2, 0, 0, 0, -2, -2]);
        assert_eq!(parse_tuning("opend").unwrap(), [-2, 0, 0, -1, -2, -2]);
        assert_eq!(parse_tuning("c6").unwrap(), [-4, 0, -2, 0, 1, 0]);
        assert_eq!(parse_tuning("dadgad").unwrap(), [-2, 0, 0, 0, -2, -2]);
        assert_eq!(parse_tuning("dsus4").unwrap(), [-2, 0, 0, 0, -2, -2]);
        assert_eq!(parse_tuning("dropd").unwrap(), [-2, 0, 0, 0, 0, 0]);
        assert_eq!(parse_tuning("dropc").unwrap(), [-4, -2, -2, -2, -2, -2]);
        assert_eq!(parse_tuning("openc").unwrap(), [-4, -2, -2, 0, 1, 0]);
        assert_eq!(parse_tuning("dropb").unwrap(), [-5, -3, -3, -3, -3, -3]);
        assert_eq!(parse_tuning("opene").unwrap(), [0, -2, -2, -2, 0, 0]);
    }

    #[test]
    fn unrecognized_name_returns_invalid_input_error() {
        let err = parse_tuning("opan G").unwrap_err();
        match err {
            TabError::InvalidInput { field, message } => {
                assert_eq!(field, "tuningName");
                assert!(
                    message.contains("opan G"),
                    "message should echo the bad value, got: {message}"
                );
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }
}

/// Generates a tuning map of open string pitches from an array of pitch offsets
/// relative to the standard 6-string tuning open pitches.
///
/// # Examples
///
/// `create_string_tuning_offset([0, 0, 0, 0, 0, 0])` creates the standard tuning.
///
/// # Panics
///
/// Panics only if an internal invariant is violated: an offset that pushes a standard
/// open-string pitch out of the `Pitch` range, or a 6-element tuning that fails
/// `create_string_tuning`. Both are BUG conditions given the fixed 6-string std tuning.
#[must_use]
pub fn create_string_tuning_offset(offsets: [i8; 6]) -> BTreeMap<StringNumber, Pitch> {
    let offset_tuning_open_pitches: Vec<Pitch> = STD_6_STRING_TUNING_OPEN_PITCHES
        .iter()
        .zip(offsets)
        .map(|(std_tuning_pitch, offset)| {
            std_tuning_pitch
                .plus_offset(offset as i16)
                .expect("BUG: Tuning pitch offset should be valid")
        })
        .collect();

    create_string_tuning(&offset_tuning_open_pitches)
        .expect("BUG: standard tuning offsets produce valid pitches")
}
#[cfg(test)]
mod test_create_string_tuning_offset {
    use super::*;

    #[test]
    fn no_offset() {
        assert_eq!(
            create_string_tuning_offset([0, 0, 0, 0, 0, 0]),
            create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES).unwrap()
        );
    }
    #[test]
    fn single_offset() {
        assert_eq!(
            create_string_tuning_offset([-2, 0, 0, 0, 0, 0]),
            create_string_tuning(&[
                Pitch::D4,
                Pitch::B3,
                Pitch::G3,
                Pitch::D3,
                Pitch::A2,
                Pitch::E2,
            ])
            .unwrap()
        );
    }
    #[test]
    fn random_offsets() {
        // Test case with random offsets
        assert_eq!(
            create_string_tuning_offset([2, -1, 3, 0, -2, 1]),
            create_string_tuning(&[
                Pitch::FSharpGFlat4,
                Pitch::ASharpBFlat3,
                Pitch::ASharpBFlat3,
                Pitch::D3,
                Pitch::G2,
                Pitch::F2,
            ])
            .unwrap()
        );
    }
}

/// Parses a newline-delimited input string into a sequence of `Line` values.
///
/// Each input line is classified as `Playable` (one or more pitches, e.g. `"A3"` or
/// `"G4Bb2"`), `Rest` (empty or comment-only), or `MeasureBreak` (a line of dash
/// characters: `-`, `–`, or `—`). Call results are cached for the 10 most recent inputs.
///
/// # Errors
///
/// Returns an error listing every unparseable substring with its 1-indexed line number.
#[memoize(Capacity: 10)]
pub fn parse_lines(
    input: String,
) -> Result<Vec<Line<BeatVec<Pitch>>>, Arc<Vec<crate::error::ParseError>>> {
    let pitch_regex = RegexBuilder::new(PITCH_PATTERN)
        .case_insensitive(true)
        .build()
        .expect("BUG: Regex pattern should be valid");

    let (parsed_lines, errors): (Vec<Line<BeatVec<Pitch>>>, Vec<Vec<crate::error::ParseError>>) =
        input
            .lines()
            .enumerate()
            .map(|(input_index, input_line)| parse_line(&pitch_regex, input_index, input_line))
            .partition_map(|result| match result {
                Ok(line) => itertools::Either::Left(line),
                Err(errs) => itertools::Either::Right(errs),
            });

    let flat_errors: Vec<crate::error::ParseError> = errors.into_iter().flatten().collect();
    if !flat_errors.is_empty() {
        return Err(Arc::new(flat_errors));
    }

    Ok(parsed_lines)
}
#[cfg(test)]
mod test_parse_lines {
    use super::*;

    #[test]
    fn parses_mixed_pitches_rests_and_measure_breaks() {
        let input = "A3\nE2// Comment\n\nG4BB2G4\n-\nE4".to_owned();
        let expected = vec![
            Line::Playable(vec![Pitch::A3]),
            Line::Playable(vec![Pitch::E2]),
            Line::Rest,
            Line::Playable(vec![Pitch::G4, Pitch::ASharpBFlat2, Pitch::G4]),
            Line::MeasureBreak,
            Line::Playable(vec![Pitch::E4]),
        ];
        assert_eq!(parse_lines(input).unwrap(), expected);
    }
    #[test]
    fn reports_line_and_content_for_unparseable_input() {
        let input = "A3xyz\nE2\n\nG4BB.2\n-\nE4".to_owned();

        let errors = parse_lines(input).unwrap_err();
        assert_eq!(
            *errors,
            vec![
                crate::error::ParseError { line: 1, text: "xyz".to_owned() },
                crate::error::ParseError { line: 4, text: "BB.2".to_owned() },
            ],
        );
    }
}

fn parse_line(
    regex: &Regex,
    input_index: usize,
    mut input_line: &str,
) -> Result<Line<Vec<Pitch>>, Vec<crate::error::ParseError>> {
    input_line = remove_comments(input_line);
    let line_content: String = remove_whitespace(input_line);

    if let Some(rest) = parse_rest(&line_content) {
        return Ok(rest);
    }
    if let Some(measure_break) = parse_measure_break(&line_content) {
        return Ok(measure_break);
    }
    parse_pitch(regex, input_index, &line_content)
}
#[cfg(test)]
mod test_parse_line {
    use super::*;

    #[test]
    fn empty() {
        assert_eq!(parse_line(&test_pitch_regex(), 0, "").unwrap(), Line::Rest);
    }
    #[test]
    fn only_comment() {
        assert_eq!(
            parse_line(&test_pitch_regex(), 0, "  // Long comment.... ").unwrap(),
            Line::Rest
        );
    }
    #[test]
    fn measure_break() {
        assert_eq!(
            parse_line(&test_pitch_regex(), 0, "    --    ").unwrap(),
            Line::MeasureBreak
        );
        assert_eq!(
            parse_line(&test_pitch_regex(), 0, "- //comment").unwrap(),
            Line::MeasureBreak
        );
    }
    #[test]
    fn parses_line_with_pitches_whitespace_and_comments() {
        let expected = Line::Playable(vec![Pitch::GSharpAFlat2, Pitch::A4, Pitch::E3, Pitch::G2]);
        assert_eq!(
            parse_line(&test_pitch_regex(), 123, "    G#2A4  E3 G2 ").unwrap(),
            expected
        );
        assert_eq!(
            parse_line(&test_pitch_regex(), 123, "G#2A4E3 G2// Comment").unwrap(),
            expected
        );
    }
    #[test]
    fn reports_error_for_unparseable_text() {
        let errors = parse_line(&test_pitch_regex(), 4, "  Invalid Text  ").unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 5);
        assert_eq!(errors[0].text, "InvalidText");
    }
}

fn remove_comments(input_line: &str) -> &str {
    input_line.split("//").next().unwrap_or(input_line)
}
#[cfg(test)]
mod test_remove_comments {
    use super::*;

    #[test]
    fn no_comment() {
        assert_eq!(remove_comments("Hello, World!"), "Hello, World!");
        assert_eq!(remove_comments("B3C1"), "B3C1");
    }
    #[test]
    fn single_comment() {
        assert_eq!(
            remove_comments("Hello, World! // This is a comment"),
            "Hello, World! "
        );
    }
    #[test]
    fn multiple_comments() {
        assert_eq!(
            remove_comments("Hello, // Comment 1\nWorld! // Comment 2"),
            "Hello, "
        );
    }
    #[test]
    fn comment_at_start() {
        assert_eq!(remove_comments("// Comment at the start"), "");
    }
}

fn remove_whitespace(input: &str) -> String {
    input.chars().filter(|c| !c.is_whitespace()).collect()
}

fn parse_rest(input_line: &str) -> Option<Line<Vec<Pitch>>> {
    if input_line.is_empty() {
        return Some(Line::Rest);
    }
    None
}
#[cfg(test)]
mod test_parse_rest {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(parse_rest(""), Some(Line::Rest));
    }
    #[test]
    fn pitch_input() {
        assert_eq!(parse_rest("G7"), None);
    }
}

fn parse_measure_break(input_line: &str) -> Option<Line<Vec<Pitch>>> {
    let unique_chars: HashSet<char> = input_line.chars().collect();
    if unique_chars == HashSet::<char>::from(['-'])
        || unique_chars == HashSet::<char>::from(['–'])
        || unique_chars == HashSet::<char>::from(['—'])
    {
        return Some(Line::MeasureBreak);
    }
    None
}
#[cfg(test)]
mod test_parse_measure_break {
    use super::*;

    #[test]
    fn measure_break_dash() {
        assert_eq!(parse_measure_break("-"), Some(Line::MeasureBreak));
    }
    #[test]
    fn measure_break_en_dash() {
        assert_eq!(parse_measure_break("–"), Some(Line::MeasureBreak));
    }
    #[test]
    fn measure_break_em_dash() {
        assert_eq!(parse_measure_break("—"), Some(Line::MeasureBreak));
    }
    #[test]
    fn empty_input() {
        assert_eq!(parse_measure_break(""), None);
    }
    #[test]
    fn no_measure_break() {
        assert_eq!(parse_measure_break("ABCDEF"), None);
    }
    #[test]
    fn whitespace_input() {
        assert_eq!(parse_measure_break(" "), None);
    }
    #[test]
    fn multiple_dashes() {
        assert_eq!(parse_measure_break("---"), Some(Line::MeasureBreak));
    }
    #[test]
    fn multiple_en_dashes() {
        assert_eq!(parse_measure_break("–––"), Some(Line::MeasureBreak));
    }
    #[test]
    fn mixed_dashes() {
        assert_eq!(parse_measure_break("-–—"), None);
    }
}

/// Parses input line to extract valid musical pitches, returning structured errors for any
/// substring that cannot be parsed.
fn parse_pitch(
    regex: &Regex,
    input_index: usize,
    input_line: &str,
) -> Result<Line<Vec<Pitch>>, Vec<crate::error::ParseError>> {
    let mut matched_mask = vec![false; input_line.len()];
    let mut matched_pitches: Vec<Pitch> = Vec::new();

    for regex_match in regex.find_iter(input_line) {
        if let Ok(pitch) = Pitch::from_str(regex_match.as_str()) {
            matched_pitches.push(pitch);
            for slot in matched_mask
                .iter_mut()
                .take(regex_match.end())
                .skip(regex_match.start())
            {
                *slot = true;
            }
        }
    }

    let unmatched_indices: Vec<usize> = matched_mask
        .iter()
        .enumerate()
        .filter_map(|(idx, matched)| if *matched { None } else { Some(idx) })
        .collect();

    if !unmatched_indices.is_empty() {
        let line_number = (input_index + 1) as u32;
        let consecutive_indices = consecutive_slices(&unmatched_indices);
        let errors: Vec<crate::error::ParseError> = consecutive_indices
            .into_iter()
            .map(|unmatched_input_indices| {
                let first_idx = *unmatched_input_indices.first().unwrap();
                let last_idx = *unmatched_input_indices.last().unwrap();
                let unmatched_input = &input_line[first_idx..=last_idx];
                crate::error::ParseError {
                    line: line_number,
                    text: unmatched_input.to_owned(),
                }
            })
            .collect();
        return Err(errors);
    }

    Ok(Line::Playable(matched_pitches))
}
#[cfg(test)]
mod test_parse_pitch {
    use super::*;

    #[test]
    fn single_natural_pitch() {
        assert_eq!(
            parse_pitch(&test_pitch_regex(), 0, "A0").unwrap(),
            Line::Playable(vec![Pitch::A0])
        );
        assert_eq!(
            parse_pitch(&test_pitch_regex(), 0, "E6").unwrap(),
            Line::Playable(vec![Pitch::E6])
        );
    }
    #[test]
    fn single_sharp_pitch() {
        assert_eq!(
            parse_pitch(&test_pitch_regex(), 0, "D#2").unwrap(),
            Line::Playable(vec![Pitch::DSharpEFlat2])
        );
    }
    #[test]
    fn single_flat_pitch() {
        assert_eq!(
            parse_pitch(&test_pitch_regex(), 0, "Db2").unwrap(),
            Line::Playable(vec![Pitch::CSharpDFlat2])
        );
        assert_eq!(
            parse_pitch(&test_pitch_regex(), 0, "Bb2").unwrap(),
            Line::Playable(vec![Pitch::ASharpBFlat2])
        );
    }
    #[test]
    fn case_insensitivity() {
        assert_eq!(
            parse_pitch(&test_pitch_regex(), 0, "A3").unwrap(),
            Line::Playable(vec![Pitch::A3])
        );
        assert_eq!(
            parse_pitch(&test_pitch_regex(), 0, "a3").unwrap(),
            Line::Playable(vec![Pitch::A3])
        );
        assert_eq!(
            parse_pitch(&test_pitch_regex(), 0, "Bb2").unwrap(),
            Line::Playable(vec![Pitch::ASharpBFlat2])
        );
        assert_eq!(
            parse_pitch(&test_pitch_regex(), 0, "bB2").unwrap(),
            Line::Playable(vec![Pitch::ASharpBFlat2])
        );
        assert_eq!(
            parse_pitch(&test_pitch_regex(), 0, "bb2").unwrap(),
            Line::Playable(vec![Pitch::ASharpBFlat2])
        );
    }
    #[test]
    fn multiple_pitches() {
        assert_eq!(
            parse_pitch(&test_pitch_regex(), 0, "C3G2A#1F8").unwrap(),
            Line::Playable(vec![Pitch::C3, Pitch::G2, Pitch::ASharpBFlat1, Pitch::F8])
        );
    }
    #[test]
    fn invalid_typo() {
        let errors = parse_pitch(&test_pitch_regex(), 12, "ZA2G#444B3").unwrap_err();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].line, 13);
        assert_eq!(errors[0].text, "Z");
        assert_eq!(errors[1].line, 13);
        assert_eq!(errors[1].text, "44");
    }
    #[test]
    fn invalid_pitch() {
        let errors = parse_pitch(&test_pitch_regex(), 28, "Fb3").unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 29);
        assert_eq!(errors[0].text, "Fb3");
    }
    #[test]
    fn invalid_random() {
        let errors = parse_pitch(&test_pitch_regex(), 0, "baS3Q-hNr").unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 1);
        assert_eq!(errors[0].text, "baS3Q-hNr");
    }
}

/// Returns a vector of consecutive slices of the input numbers.
///
/// This function does not sort the input vector and the consecutive slices are grouped together based
/// on the order of the input numbers as received.
/// Each returned slice is a reference to a subarray of `usize` elements from the original data array.
fn consecutive_slices(numbers: &[usize]) -> Vec<&[usize]> {
    let mut slice_start = 0;
    let mut result = Vec::with_capacity(numbers.len());
    for i in 1..numbers.len() {
        if numbers[i - 1] + 1 != numbers[i] {
            result.push(&numbers[slice_start..i]);
            slice_start = i;
        }
    }
    if !numbers.is_empty() {
        result.push(&numbers[slice_start..]);
    }
    result
}
#[cfg(test)]
mod test_consecutive_slices {
    use super::*;

    #[test]
    fn simple() {
        let flat_nums = vec![1, 2, 3, 4];
        let consecutive_nums = vec![vec![1, 2, 3, 4]];

        assert_eq!(consecutive_slices(&flat_nums), consecutive_nums);
    }
    #[test]
    fn complex() {
        let flat_nums = vec![1, 2, 3, 4, 113, 115, 116, 6, 7, 8];
        let consecutive_nums = vec![vec![1, 2, 3, 4], vec![113], vec![115, 116], vec![6, 7, 8]];

        assert_eq!(consecutive_slices(&flat_nums), consecutive_nums);
    }
    #[test]
    fn no_consecutive() {
        let flat_nums = vec![95, 65, 74, 96, 68, 29, 34, 32];
        let consecutive_nums = vec![
            vec![95],
            vec![65],
            vec![74],
            vec![96],
            vec![68],
            vec![29],
            vec![34],
            vec![32],
        ];

        assert_eq!(consecutive_slices(&flat_nums), consecutive_nums);
    }
}

#[cfg(test)]
mod test_get_tuning_names {
    use super::*;

    #[test]
    fn returns_non_empty_set() {
        assert!(!get_tuning_names().is_empty());
    }
}
