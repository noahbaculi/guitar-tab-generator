extern crate guitar_tab_generator;

use guitar_tab_generator::{TabInput, generate_arrangements};

/// Basic usage example using `generate_arrangements` and `render`.
fn main() {
    let tab_input = TabInput::new(
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
        E3
        A3
        C3
        E3
        A3",
        "standard",
        18,
        0,
        1,
    );

    let set = generate_arrangements(tab_input).unwrap();
    let tab = set.render(0, 55, 2, Some(12)).unwrap();
    println!("Tab:\n{tab}");
}
