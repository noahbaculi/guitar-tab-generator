extern crate guitar_tab_generator;

fn main() {
    let web_input = guitar_tab_generator::WebInput {
        input_pitches: "E4
        Eb4

        E4
        Eb4
        E4
        B3
        D4
        C4
        -
        A2A3
        E3E3E3
        A3
        C3
        E3
        A3"
        .to_owned(),

        tuning_name: "standard".to_owned(),
        guitar_num_frets: 18,
        guitar_capo: 0,
        num_arrangements: 1,
        width: 40,
        padding: 2,
        playback_index: Some(12),
    };

    let tab = guitar_tab_generator::create_guitar_compositions(
        serde_wasm_bindgen::to_value(&web_input).unwrap(),
    )
    .map_err(wasm_bindgen::JsValue::from)
    .unwrap();
    dbg!(&tab);
}
