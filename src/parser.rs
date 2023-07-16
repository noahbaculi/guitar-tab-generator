use crate::{arrangement::InvalidInput, pitch::Pitch};
use itertools::Itertools;
use std::str::FromStr;
use strum_macros::EnumString;

#[derive(EnumString, Debug, PartialEq)]
enum Color {
    Red,
    Green { range: usize },
    Blue(usize),
    Yellow,
}

pub fn parse_arrangements(input: String) -> String {
    let _x = input
        .lines()
        .enumerate()
        .map(|(input_index, input_line)| parse_line(input_index, input_line.to_owned()))
        .collect_vec();
    // dbg!(&_x);

    // let _a = Color::from_str("Green").unwrap();
    let _a = Pitch::from_str("Bb0");
    // dbg!(&_a);

    "Hi".to_owned()
}

fn parse_line(_input_index: usize, mut input_line: String) -> Result<Vec<Pitch>, InvalidInput> {
    dbg!(&input_line);

    // Iterate over 3 character chunks first to identify sharp or flat pitches
    let three_char_pitches = input_line
        .chars()
        .tuple_windows()
        // .inspect(|(a, b, c)| println!("{} - {} - {}", a, b, c))
        .map(|(a, b, c)| format!("{a}{b}{c}"))
        .filter(|three_char_window| Pitch::from_str(three_char_window).is_ok())
        .collect::<Vec<_>>();
    dbg!(&three_char_pitches);
    for three_char_pitch in three_char_pitches {
        input_line = input_line.replace(&three_char_pitch, "");
    }

    dbg!(&input_line);

    println!("------");
    Ok(vec![])
}
