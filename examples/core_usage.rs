use anyhow::Result;
use guitar_tab_generator::{
    arrangement::{create_arrangements, Line},
    guitar::{create_string_tuning, Guitar},
    parser::parse_lines,
    pitch::Pitch,
    renderer::render_tab,
};

extern crate guitar_tab_generator;

fn main() -> Result<()> {
    let input = "E4
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
    .to_string();

    let lines: Vec<Line<Vec<Pitch>>> = parse_lines(input)?;

    let tuning = create_string_tuning(&[
        Pitch::E4,
        Pitch::B3,
        Pitch::G3,
        Pitch::D3,
        Pitch::A2,
        Pitch::E2,
    ]);

    let guitar_num_frets = 18;
    let guitar_capo = 0;
    let guitar = Guitar::new(tuning, guitar_num_frets, guitar_capo)?;
    dbg!(&guitar);

    let num_arrangements = 1;
    let arrangements = create_arrangements(guitar.clone(), lines, num_arrangements)?;
    dbg!(&guitar);

    let tab_width = 20;
    let padding = 1;
    let playback_index = Some(2);
    let tab = render_tab(
        &arrangements[0].lines,
        &guitar,
        tab_width,
        padding,
        playback_index,
    );
    println!("Tab:\n{}", tab);

    Ok(())
}
