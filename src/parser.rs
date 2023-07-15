use crate::pitch::Pitch;
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
    let _x = input.lines().collect_vec();
    // dbg!(&_x);

    // let _a = Color::from_str("Green").unwrap();
    let _a = Pitch::from_str("Db0");
    dbg!(&_a);

    "Hi".to_owned()
}
