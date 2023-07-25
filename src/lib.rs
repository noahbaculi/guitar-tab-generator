use anyhow::Result;
use arrangement::create_arrangements;
use guitar::Guitar;
use itertools::Itertools;
use parser::{create_string_tuning_offset, parse_lines, parse_tuning};
use pitch::Pitch;
use renderer::render_tab;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

pub mod arrangement;
pub mod guitar;
pub mod parser;
pub mod pitch;
pub mod renderer;
pub mod string_number;

#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, Serialize, Deserialize)]
pub struct WebInput {
    pub input_pitches: String,
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
pub struct WebArrangement {
    pub tab: String,
    pub max_fret_span: u32,
}

#[wasm_bindgen]
pub fn create_guitar_compositions(input: JsValue) -> Result<JsValue, JsError> {
    let WebInput {
        input_pitches,
        tuning_name,
        guitar_num_frets,
        guitar_capo,
        num_arrangements,
        width,
        padding,
        playback_index,
    }: WebInput = serde_wasm_bindgen::from_value(input)?;

    let input_lines: Vec<arrangement::Line<Vec<Pitch>>> = match parse_lines(input_pitches) {
        Ok(lines) => lines,
        Err(e) => return Err(JsError::new(&e.to_string())),
    };

    let tuning = create_string_tuning_offset(parse_tuning(&tuning_name));

    let guitar = match Guitar::new(tuning, guitar_num_frets, guitar_capo) {
        Ok(guitar) => guitar,
        Err(e) => return Err(JsError::new(&e.to_string())),
    };

    let arrangements = match create_arrangements(guitar.clone(), input_lines, num_arrangements) {
        Ok(arrangements) => arrangements,
        Err(e) => return Err(JsError::new(&e.to_string())),
    };

    let web_arrangements = arrangements
        .iter()
        .map(|arrangement| WebArrangement {
            tab: render_tab(&arrangement.lines, &guitar, width, padding, playback_index),
            max_fret_span: 2,
        })
        .collect_vec();

    Ok(serde_wasm_bindgen::to_value(&web_arrangements)?)
}
