fn main() {
    let tab = guitar_tab_generator::create_guitar_compositions(
        "E4
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
        "standard",
        18,
        0,
        1,
        40,
        2,
        Some(13),
    )
    .map_err(wasm_bindgen::JsValue::from)
    .unwrap();
    dbg!(&tab);
}
