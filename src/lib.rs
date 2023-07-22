use anyhow::Result;
use arrangement::create_arrangements;
use guitar::Guitar;
use parser::{create_string_tuning_offset, parse_lines, parse_tuning};
use pitch::Pitch;
use renderer::render_tab;
use wasm_bindgen::prelude::*;

pub mod arrangement;
pub mod guitar;
pub mod parser;
pub mod pitch;
pub mod renderer;
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
#[derive(Debug)]
pub struct WebArrangement {
    pub composition: String,
    pub max_fret_span: u32,
}

#[wasm_bindgen]
#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn create_guitar_compositions(
    input: String,
    tuning_name: &str,
    guitar_num_frets: u8,
    guitar_capo: u8,
    num_arrangements: u8,
    width: u16,
    padding: u8,
    playback_beat_num: Option<u16>,
) -> Result<WebArrangement, JsError> {
    let input_lines: Vec<arrangement::Line<Vec<Pitch>>> = match parse_lines(input) {
        Ok(lines) => lines,
        Err(e) => return Err(JsError::new(&e.to_string())),
    };

    let tuning = create_string_tuning_offset(parse_tuning(tuning_name));

    let guitar = match Guitar::new(tuning, guitar_num_frets, guitar_capo) {
        Ok(guitar) => guitar,
        Err(e) => return Err(JsError::new(&e.to_string())),
    };

    let arrangements = match create_arrangements(guitar.clone(), input_lines, num_arrangements) {
        Ok(arrangements) => arrangements,
        Err(e) => return Err(JsError::new(&e.to_string())),
    };

    let tab = render_tab(
        arrangements[0].clone(),
        guitar,
        width,
        padding,
        playback_beat_num,
    );
    println!("{}", &tab);

    Ok(WebArrangement {
        composition: tab,
        max_fret_span: 2,
    })
}

#[wasm_bindgen]
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}
