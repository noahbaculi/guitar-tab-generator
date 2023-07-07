use std::collections::BTreeMap;

use guitar_tab_generator::guitar::Guitar;
use guitar_tab_generator::pitch::Pitch;
use guitar_tab_generator::string_number::StringNumber;
use guitar_tab_generator::*;

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

    println!("{} {}", Pitch::E2, Pitch::CSharp2);

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
    let input_pitches = vec![vec![Pitch::G3], vec![Pitch::B3], vec![Pitch::D4, Pitch::G4]];
    let _arrangement = match Arrangement::new(_guitar, input_pitches) {
        Ok(arrangement) => arrangement,
        Err(err) => {
            panic!("{}", err);
        }
    };
    // dbg!(&_arrangement);
}
