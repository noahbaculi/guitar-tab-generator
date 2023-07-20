fn main() {
    let tab = guitar_tab_generator::create_guitar_compositions(
        "E4
        Eb4
        E4
        Eb4
        E4
        B3
        D4
        C4"
        .to_owned(),
        "standard",
        18,
        0,
        1,
        None,
    )
    .map_err(wasm_bindgen::JsValue::from)
    .unwrap();
    dbg!(&tab);
}
