use guitar_tab_generator::{
    create_arrangements, create_string_tuning, parse_lines, render_tab, Guitar, Line,
    NumArrangements, Pitch, TabError,
};

extern crate guitar_tab_generator;

/// Advanced usage example using the individual component functions.
fn main() -> Result<(), TabError> {
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

    // Mirror the cache-hit-safe unwrap pattern from `generate_arrangements`:
    // a fresh `Arc` (cache miss) unwraps cleanly, a shared one (cache hit) clones.
    let lines: Vec<Line<Vec<Pitch>>> = parse_lines(input).map_err(|errs| {
        let errors = std::sync::Arc::try_unwrap(errs).unwrap_or_else(|arc| (*arc).clone());
        TabError::Parse { errors }
    })?;

    let tuning = create_string_tuning(&[
        Pitch::E4,
        Pitch::B3,
        Pitch::G3,
        Pitch::D3,
        Pitch::A2,
        Pitch::E2,
    ])?;

    let guitar_num_frets = 18;
    let guitar_capo = 0;
    let guitar = Guitar::new(tuning, guitar_num_frets, guitar_capo)?;

    let num_arrangements = NumArrangements::try_new(1)?;
    let arrangements = create_arrangements(guitar.clone(), lines, num_arrangements, None)
        .map_err(|arc| std::sync::Arc::try_unwrap(arc).unwrap_or_else(|a| (*a).clone()))?;

    // dbg!(&arrangements);

    let tab_width = 20;
    let padding = 1;
    let playback_index = Some(2);
    let tab = render_tab(
        arrangements[0].lines(),
        &guitar,
        tab_width,
        padding,
        playback_index,
    );
    println!("Tab:\n{tab}");

    Ok(())
}
