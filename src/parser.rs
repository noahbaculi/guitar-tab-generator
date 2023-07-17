use crate::{arrangement::InvalidInput, pitch::Pitch};
use itertools::Itertools;
use regex::Regex;
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

fn parse_line(_input_index: usize, input_line: String) -> Result<Vec<Pitch>, InvalidInput> {
    dbg!(&input_line);

    let pattern = r"(?P<three_char_pitch>[a-gA-G][#|♯|b|♭][0-9])|(?P<two_char_pitch>[a-gA-G][0-9])";

    let re = Regex::new(pattern).unwrap();

    // for caps in re.captures_iter(&input_line) {
    //     dbg!(&caps[0]);
    // }
    for caps in re.find_iter(&input_line) {
        dbg!(&caps);
    }

    println!("------");
    Ok(vec![])
}
