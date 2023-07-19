use wasm_bindgen::prelude::*;

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
pub fn test_return_struct(composition_str: &str) -> WebArrangement {
    WebArrangement {
        composition: composition_str.to_owned(),
        max_fret_span: 2,
    }
}

#[wasm_bindgen]
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}
