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
#[derive(Debug, Serialize, Deserialize)]
pub struct Composition {
    pub tab: String,
    pub max_fret_span: u8,
}

#[wasm_bindgen]
pub fn wasm_create_guitar_compositions(input: JsValue) -> Result<JsValue, JsError> {
    let composition_input: CompositionInput = serde_wasm_bindgen::from_value(input)?;

    let compositions = match create_guitar_compositions(composition_input) {
        Ok(comps) => comps,
        Err(e) => return Err(JsError::new(&e.to_string())),
    };

    Ok(serde_wasm_bindgen::to_value(&compositions)?)
}

pub fn create_guitar_compositions(composition_input: CompositionInput) -> Result<Vec<Composition>> {
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
