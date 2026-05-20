use anyhow::Result;
use guitar_tab_generator::{
    create_arrangements, create_string_tuning, parse_lines, render_tab, Guitar, Line,
    NumArrangements, Pitch,
};

extern crate guitar_tab_generator;

/// Advanced usage example using the individual component functions.
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

    let lines: Vec<Line<Vec<Pitch>>> = match parse_lines(input) {
        Ok(input_lines) => input_lines,
        Err(errs) => {
            let joined = errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
            return Err(anyhow::anyhow!(joined));
        }
    };

    let tuning = create_string_tuning(&[
        Pitch::E4,
        Pitch::B3,
        Pitch::G3,
        Pitch::D3,
        Pitch::A2,
        Pitch::E2,
    ])
    .unwrap();

    let guitar_num_frets = 18;
    let guitar_capo = 0;
    let guitar = Guitar::new(tuning, guitar_num_frets, guitar_capo)?;
    // dbg!(&guitar);

    let num_arrangements = NumArrangements::try_new(1)?;
    let arrangements = match create_arrangements(guitar.clone(), lines, num_arrangements, None) {
        Ok(arrangements) => arrangements,
        Err(e) => return Err(std::sync::Arc::try_unwrap(e).unwrap()),
    };

    // dbg!(&arrangements);

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
    println!("Tab:\n{tab}");

    Ok(())
}
