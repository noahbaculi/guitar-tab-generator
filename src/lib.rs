use anyhow::Result;
use guitar::Guitar;
use itertools::Itertools;
use pitch::Pitch;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

pub mod arrangement;
pub mod guitar;
pub mod parser;
pub mod pitch;
pub mod renderer;
pub mod string_number;

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

#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    pub tab: String,
    pub max_fret_span: u8,
}

#[wasm_bindgen]
#[cfg(not(tarpaulin_include))]
pub fn wasm_create_guitar_compositions(input: JsValue) -> Result<JsValue, JsError> {
    let composition_input: CompositionInput = serde_wasm_bindgen::from_value(input)?;

    let compositions = match wrapper_create_arrangements(composition_input) {
        Ok(compositions) => compositions,
        Err(e) => return Err(JsError::new(&e.to_string())),
    };

    Ok(serde_wasm_bindgen::to_value(&compositions)?)
}

pub fn wrapper_create_arrangements(
    composition_input: CompositionInput,
) -> Result<Vec<Composition>> {
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

    let input_lines: Vec<arrangement::Line<Vec<Pitch>>> = parser::parse_lines(input_pitches)?;

    let tuning = parser::create_string_tuning_offset(parser::parse_tuning(&tuning_name));

    let guitar = Guitar::new(tuning, guitar_num_frets, guitar_capo)?;

    let arrangements =
        arrangement::create_arrangements(guitar.clone(), input_lines, num_arrangements)?;

    let compositions = arrangements
        .iter()
        .map(|arrangement| Composition {
            tab: renderer::render_tab(&arrangement.lines, &guitar, width, padding, playback_index),
            max_fret_span: arrangement.max_fret_span(),
        })
        .collect_vec();

    Ok(compositions)
}
#[cfg(test)]
mod test_wrapper_create_arrangements {
    use super::*;

    #[test]
    fn test_create_guitar_compositions_valid_input() {
        let composition_input = CompositionInput {
            pitches: "E2\nA2\nD3\nG3\nB3\nE4".to_owned(),
            tuning_name: "standard".to_string(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 1,
            width: 24,
            padding: 2,
            playback_index: Some(3),
        };

        let compositions = wrapper_create_arrangements(composition_input).unwrap();
        let expected_composition = Composition {
            tab: "           ▼\n-----------------0------\n--------------0---------\n-----------0------------\n--------0---------------\n-----0------------------\n--0---------------------\n           ▲\n".to_owned(),
            max_fret_span: 0,
        };

        assert_eq!(compositions[0], expected_composition);
    }
    #[test]
    fn test_error() {
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
