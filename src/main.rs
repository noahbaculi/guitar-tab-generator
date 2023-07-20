pub mod pitch;
use pitch::Pitch;

pub mod string_number;

pub mod guitar;
use guitar::Guitar;

pub mod arrangement;
use arrangement::Line::{MeasureBreak, Playable, Rest};

use crate::guitar::{create_string_tuning, STD_6_STRING_TUNING_OPEN_PITCHES};

mod parser;

fn main() {
    let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES);

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
    let _arrangements = match arrangement::create_arrangements(_guitar, input_pitches, 3) {
        Ok(arrangements) => arrangements,
        Err(err) => panic!("{}", err),
    };
    // dbg!(&_arrangements);

    // let _arrangement_outputs =
    //     parser::parse_lines("bb5C7d#2/hi//there\nG3noaha2aaron\nb3\n\nD4G4\n---\nC2".to_owned());
    let _arrangement_outputs = parser::parse_lines("E2//there\nG3\nb3\n\nD4G4\n---\nC2".to_owned());
    dbg!(&_arrangement_outputs);
}
