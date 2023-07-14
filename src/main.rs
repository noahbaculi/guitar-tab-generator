use std::collections::BTreeMap;

pub mod pitch;
use pitch::Pitch;

pub mod string_number;
use string_number::StringNumber;

pub mod guitar;
use guitar::Guitar;

pub mod arrangement;
use arrangement::Arrangement;
use arrangement::Line::{MeasureBreak, Playable};

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
    // ]; // invalid
    let input_pitches = vec![
        Playable(vec![Pitch::G3]),
        MeasureBreak,
        Playable(vec![Pitch::B3]),
        Playable(vec![Pitch::D4, Pitch::G4]),
    ];
    // let input_pitches = vec![vec![Pitch::D4, Pitch::G4]];
    let _arrangement = match Arrangement::new(_guitar, input_pitches) {
        Ok(arrangement) => arrangement,
        Err(err) => {
            panic!("{}", err);
        }
    };
    // dbg!(&_arrangement);
}
