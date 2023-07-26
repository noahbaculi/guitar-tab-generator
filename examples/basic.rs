extern crate guitar_tab_generator;

/// Basic usage example using the wrapper creation function.
fn main() {
    let input = guitar_tab_generator::CompositionInput {
        pitches: "E4
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
        width: 55,
        padding: 2,
        playback_index: Some(12),
    };

    let compositions = guitar_tab_generator::wrapper_create_arrangements(input).unwrap();
    // dbg!(&compositions);
    println!("Tab:\n{}", compositions[0].tab);
}
