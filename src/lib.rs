use anyhow::Result;
use parser::{parse_lines, parse_tuning};
use pitch::Pitch;
use wasm_bindgen::prelude::*;

pub mod arrangement;
pub mod guitar;
pub mod parser;
pub mod pitch;
pub mod string_number;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}
#[wasm_bindgen(getter_with_clone)]
pub struct WebArrangement {
    pub composition: String,
    pub max_fret_span: u32,
}

#[wasm_bindgen]
#[allow(unused_variables)]
pub fn create_guitar_compositions(
    input: String,
    tuning_input: &str,
    guitar_capo: u8,
    playback_beat_num: u16,
) -> Result<WebArrangement, JsError> {
    let _input_lines: Result<Vec<arrangement::Line<Vec<Pitch>>>> = match parse_lines(input) {
        Ok(lines) => Ok(lines),
        Err(e) => return Err(JsError::new(&e.to_string())),
    };

    let tuning = parse_tuning(tuning_input);

    Ok(WebArrangement {
        composition: "Hi".to_owned(),
        max_fret_span: 2,
    })
}

#[wasm_bindgen]
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}
