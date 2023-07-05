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
    match Guitar::new(tuning, 18) {
        // Ok(_) => {}
        Ok(g) => {
            dbg!(g);
        }
        Err(e) => println!("There is an error: {}", e),
    };
    // dbg!(x);
}
