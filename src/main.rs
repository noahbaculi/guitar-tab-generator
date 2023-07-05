use guitar_tab_generator::*;
fn main() {
    let tuning = StringCollection {
        e: Pitch::E4,
        B: Pitch::B3,
        G: Pitch::G3,
        D: Pitch::D3,
        A: Pitch::A2,
        E: Pitch::E2,
    };
    let _g = Guitar::new(tuning, 16).unwrap();
    // dbg!(_g);

    let input_pitches = vec![vec![Pitch::G3], vec![Pitch::B3], vec![Pitch::D4, Pitch::G4]];

    let _arr = Arrangement::new(_g, input_pitches);
    // dbg!(_arr);
}
