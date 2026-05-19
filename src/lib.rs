#![deny(clippy::correctness)]

//! Generate fingerstyle guitar tabs from a sequence of pitches.
//!
//! Given a string of newline-separated pitches (e.g. `"E2\nA2\nD3"`), a tuning,
//! and guitar parameters, this crate picks playable fingerings and renders an
//! ASCII tab. Arrangements are ranked by difficulty and returned in ascending
//! order; the first arrangement is the easiest to play.
//!
//! # Quick start
//!
//! ```no_run
//! use guitar_tab_generator::{
//!     create_arrangements, create_string_tuning, parse_lines, render_tab, Guitar,
//!     STD_6_STRING_TUNING_OPEN_PITCHES,
//! };
//!
//! let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES).unwrap();
//! let guitar = Guitar::new(tuning, 18, 0).unwrap();
//! let input_lines = parse_lines("E2\nA2\nD3".to_owned()).unwrap();
//! let arrangements = create_arrangements(guitar.clone(), input_lines, 1).unwrap();
//! let tab = render_tab(&arrangements[0].lines, &guitar, 30, 2, None);
//! println!("{tab}");
//! ```

use anyhow::{anyhow, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

pub(crate) mod arrangement;
pub(crate) mod error;
pub(crate) mod guitar;
pub(crate) mod parser;
pub(crate) mod pitch;
pub(crate) mod renderer;
pub(crate) mod string_number;

pub use arrangement::{
    create_arrangements, memoized_original_create_arrangements, Arrangement, BeatVec, Line,
};
pub use error::{ParseError, TabError};
pub use guitar::{
    create_string_tuning, Guitar, PitchFingering, STD_6_STRING_TUNING_OPEN_PITCHES,
};
pub use parser::{
    create_string_tuning_offset, memoized_original_parse_lines, parse_lines, parse_tuning,
};
pub use pitch::Pitch;
pub use renderer::render_tab;
pub use string_number::StringNumber;

/// The fully-specified input for generating one set of compositions from a pitch string.
///
/// Values map directly to the WASM boundary via serde; `pitches` is the raw newline-
/// delimited input text, `tuning_name` is one of the `TuningName` variants or `"standard"`,
/// and `num_arrangements` must be in `1..=20`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionInput {
    pub pitches: String,
    pub tuning_name: String,
    pub guitar_num_frets: u8,
    pub guitar_capo: u8,
    pub num_arrangements: u8,
    pub width: u16,
    pub padding: u8,
    pub playback_index: Option<u16>,
}

/// A single rendered arrangement returned from `wrapper_create_arrangements`.
///
/// `tab` is the rendered ASCII tab, `normalized_input` is the per-beat input echoed back
/// (pitch text for playable beats, the sentinels `"REST"` and `"MEASURE_BREAK"` otherwise;
/// shared across the result set via `Rc`), and `max_fret_span` reports the widest
/// non-zero fret span across any beat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderedTab {
    pub tab: String,
    pub normalized_input: Rc<Vec<BeatVec<String>>>,
    pub max_fret_span: u8,
}

#[wasm_bindgen]
#[cfg(not(tarpaulin_include))]
pub fn wasm_create_guitar_compositions(input: JsValue) -> Result<JsValue, JsError> {
    let composition_input: CompositionInput = serde_wasm_bindgen::from_value(input)?;

    let rendered_tabs = match wrapper_create_arrangements(composition_input) {
        Ok(rendered_tabs) => rendered_tabs,
        Err(e) => return Err(JsError::new(&e.to_string())),
    };

    Ok(serde_wasm_bindgen::to_value(&rendered_tabs)?)
}

/// Parses, arranges, and renders a full set of `RenderedTab`s from a `CompositionInput`.
///
/// # Errors
///
/// Returns an error if any of the underlying steps fails: parsing (unparseable lines),
/// guitar construction (invalid tuning, capo, or fret count), or arrangement (no valid
/// fingering for a pitch).
pub fn wrapper_create_arrangements(
    composition_input: CompositionInput,
) -> Result<Vec<RenderedTab>> {
    let CompositionInput {
        pitches: input_pitches,
        tuning_name,
        guitar_num_frets,
        guitar_capo,
        num_arrangements,
        width,
        padding,
        playback_index,
    } = composition_input;

    let input_lines: Vec<arrangement::Line<Vec<Pitch>>> = parser::parse_lines(input_pitches)
        .map_err(|errs| {
            let joined = errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
            anyhow!("{joined}")
        })?;

    let first_playable_index = input_lines
        .iter()
        .position(|line| matches!(line, arrangement::Line::Playable(_)))
        .unwrap_or(0);

    let normalized_input: Rc<Vec<BeatVec<String>>> = Rc::new(
        input_lines
            .iter()
            .skip(first_playable_index)
            .map(|line| match line {
                arrangement::Line::Playable(pitches) => {
                    pitches.iter().map(|p| p.plain_text().to_owned()).collect()
                }
                arrangement::Line::Rest => vec!["REST".to_owned()],
                arrangement::Line::MeasureBreak => vec!["MEASURE_BREAK".to_owned()],
            })
            .collect_vec(),
    );

    let tuning = parser::create_string_tuning_offset(parser::parse_tuning(&tuning_name));

    let guitar = Guitar::new(tuning, guitar_num_frets, guitar_capo)?;

    let arrangements =
        arrangement::create_arrangements(guitar.clone(), input_lines, num_arrangements)
            .map_err(|e| anyhow!("{e}"))?;

    let rendered_tabs = arrangements
        .iter()
        .map(|arrangement| RenderedTab {
            tab: renderer::render_tab(&arrangement.lines, &guitar, width, padding, playback_index),
            normalized_input: Rc::clone(&normalized_input),
            max_fret_span: arrangement.max_fret_span(),
        })
        .collect_vec();

    Ok(rendered_tabs)
}
#[cfg(test)]
mod test_wrapper_create_arrangements {
    use super::*;

    #[test]
    fn valid_input() {
        let composition_input = CompositionInput {
            pitches: "E2\nA2\nD3\n\nG3\nB3\n---\nE4".to_owned(),
            tuning_name: "standard".to_string(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 1,
            width: 30,
            padding: 2,
            playback_index: Some(3),
        };

        let rendered_tabs = wrapper_create_arrangements(composition_input).unwrap();
        let expected = RenderedTab {
            tab: "           ▼\n--------------------|--0------\n-----------------0--|---------\n--------------0-----|---------\n--------0-----------|---------\n-----0--------------|---------\n--0-----------------|---------\n           ▲\n".to_owned(),
            normalized_input: Rc::new(vec![
                vec!["E2".to_owned()],
                vec!["A2".to_owned()],
                vec!["D3".to_owned()],
                vec!["REST".to_owned()],
                vec!["G3".to_owned()],
                vec!["B3".to_owned()],
                vec!["MEASURE_BREAK".to_owned()],
                vec!["E4".to_owned()]
                ]),
            max_fret_span: 0,
        };

        assert_eq!(rendered_tabs[0], expected);
    }
    #[test]
    fn empty_input() {
        let composition_input = CompositionInput {
            pitches: "\n\n\n---\n \n".to_owned(),
            tuning_name: "standard".to_string(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 2,
            width: 30,
            padding: 2,
            playback_index: Some(3),
        };

        let rendered_tabs = wrapper_create_arrangements(composition_input).unwrap();
        let expected = vec![
            RenderedTab {
                tab: "".to_owned(),
                normalized_input: Rc::new(vec![
                    vec!["REST".to_owned()],
                    vec!["REST".to_owned()],
                    vec!["REST".to_owned()],
                    vec!["MEASURE_BREAK".to_owned()],
                    vec!["REST".to_owned()]
                ]),
                max_fret_span: 0,
            };
            2
        ];

        assert_eq!(rendered_tabs, expected);
    }
    #[test]
    fn invalid_input() {
        let composition_input = CompositionInput {
            pitches: "E2\nA2\nD3\n???\nG3\nB3\nE4".to_owned(),
            tuning_name: "standard".to_string(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 1,
            width: 20,
            padding: 2,
            playback_index: Some(3),
        };
        assert!(wrapper_create_arrangements(composition_input).is_err());
    }
}
