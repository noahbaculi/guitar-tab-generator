use std::collections::BTreeMap;

pub mod pitch;
use pitch::Pitch;

pub mod string_number;
use string_number::StringNumber;

pub mod guitar;
use guitar::Guitar;

pub mod arrangement;
use arrangement::Line::{MeasureBreak, Playable, Rest};

mod parser;

fn main() {
    let tuning = BTreeMap::from([
        (StringNumber::new(1).unwrap(), Pitch::E4),
        (StringNumber::new(2).unwrap(), Pitch::B3),
        (StringNumber::new(3).unwrap(), Pitch::G3),
        (StringNumber::new(4).unwrap(), Pitch::D3),
        (StringNumber::new(5).unwrap(), Pitch::A2),
        (StringNumber::new(6).unwrap(), Pitch::E2),
    ]);

    // dbg!(&tuning);

    let _guitar = match Guitar::new(tuning, 18) {
        Ok(guitar) => guitar,
        Err(err) => {
            panic!("{}", err);
        }
    };
    // dbg!(&_guitar);

    // let input_pitches = vec![
    //     vec![Pitch::A1],
    //     vec![Pitch::G3],
    //     vec![Pitch::B3],
    //     vec![Pitch::A1, Pitch::B1],
    //     vec![Pitch::G3, Pitch::D2],
    //     vec![Pitch::D4, Pitch::G4],
    // ];
    let input_pitches = vec![
        MeasureBreak,
        Playable(vec![Pitch::G3]),
        Rest,
        Playable(vec![Pitch::CSharpDFlat4]),
        MeasureBreak,
        Playable(vec![Pitch::D4, Pitch::G4]),
        Playable(vec![Pitch::A5]),
        Playable(vec![Pitch::B3]),
        MeasureBreak,
        Playable(vec![Pitch::D4, Pitch::G4]),
        Rest,
        Playable(vec![Pitch::A5]),
        Playable(vec![Pitch::B3]),
        MeasureBreak,
        Playable(vec![Pitch::D4, Pitch::G4]),
    ];
    // let input_pitches = vec![vec![Pitch::D4, Pitch::G4]];
    let _arrangements = match arrangement::create_arrangements(_guitar, input_pitches) {
        Ok(arrangements) => arrangements,
        Err(err) => panic!("{}", err),
    };
    // dbg!(&_arrangements);

    let _arrangement_outputs =
        parser::parse_arrangements("bb5C7d#2xyx\nG3noaha2aaron\nb3\nD4G4".to_owned());
    // dbg!(&_arrangement_outputs);
}
