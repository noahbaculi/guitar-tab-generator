extern crate guitar_tab_generator;

use guitar_tab_generator::{build_arrangement_set, TabInput};

/// Basic usage example using `build_arrangement_set` and `render`.
fn main() {
    let tab_input = TabInput {
        input: "E4
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
        A3"
        .to_owned(),
        tuning_name: "standard".to_owned(),
        guitar_num_frets: 18,
        guitar_capo: 0,
        num_arrangements: 1,
        max_fret_span_filter: None,
    };

    let set = build_arrangement_set(tab_input).unwrap();
    let tab = set.render(0, 55, 2, Some(12)).unwrap();
    println!("Tab:\n{tab}");
}
