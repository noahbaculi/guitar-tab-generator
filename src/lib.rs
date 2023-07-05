use std::{
    collections::{BTreeMap, HashSet},
    error::Error,
    fmt,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, EnumIter)]
pub enum Pitch {
    A1,
    A1Sharp,
    B1,
    C2,
    C2Sharp,
    D2,
    D2Sharp,
    E2,
    F2,
    F2Sharp,
    G2,
    G2Sharp,
    A2,
    A2Sharp,
    B2,
    C3,
    C3Sharp,
    D3,
    D3Sharp,
    E3,
    F3,
    F3Sharp,
    G3,
    G3Sharp,
    A3,
    A3Sharp,
    B3,
    C4,
    C4Sharp,
    D4,
    D4Sharp,
    E4,
    F4,
    F4Sharp,
    G4,
    G4Sharp,
    A4,
    A4Sharp,
    B4,
    C5,
    C5Sharp,
    D5,
    D5Sharp,
    E5,
    F5,
    F5Sharp,
    G5,
    G5Sharp,
    A5,
    A5Sharp,
    B5,
    C6,
    C6Sharp,
    D6,
    D6Sharp,
    E6,
    F6,
    F6Sharp,
    G6,
    G6Sharp,
}

// #[derive(Debug)]
// pub enum GuitarString {
//     #[allow(non_camel_case_types)]
//     e = 1,
//     B = 2,
//     G = 3,
//     D = 4,
//     A = 5,
//     E = 6,
// }
// #[derive(Debug, Clone)]
// #[allow(non_snake_case)]
// pub struct StringCollection<T> {
//     pub e: T,
//     pub B: T,
//     pub G: T,
//     pub D: T,
//     pub A: T,
//     pub E: T,
// }
// impl<T: std::cmp::Ord + Clone> StringCollection<T> {
//     pub fn as_array(&self) -> [&T; 6] {
//         [&self.e, &self.B, &self.G, &self.D, &self.A, &self.E]
//     }
//     pub fn into_array(self) -> [T; 6] {
//         [self.e, self.B, self.G, self.D, self.A, self.E]
//     }
// }

// #[derive(Debug)]
// pub struct Fingering {
//     pub guitar_string: GuitarString,
//     pub fret: u8,
// }

// #[derive(Debug, Clone)]
// pub struct InvalidFretError {
//     pub fret: u8,
// }

// impl Fingering {
//     pub fn new(guitar_string: GuitarString, fret: u8) -> Result<Self, InvalidFretError> {
//         match fret {
//             0..=18 => Ok(Fingering {
//                 guitar_string,
//                 fret,
//             }),
//             _ => Err(InvalidFretError { fret }),
//         }
//     }
// }

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StringNumber(usize);
impl StringNumber {
    pub fn new(string_number: usize) -> Result<Self, Box<dyn Error>> {
        const MAX_NUM_STRINGS: usize = 12;
        if string_number > MAX_NUM_STRINGS {
            return Err(format!(
                "The string number ({}) is too high. The maximum is {}.",
                string_number, MAX_NUM_STRINGS
            )
            .into());
        }
        Ok(StringNumber(string_number))
    }
}
impl fmt::Debug for StringNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // write!(f, "{}", self.0)
        let string_number = self.0;
        let string_pitch_letter = match string_number {
            1 => "e".to_owned(),
            2 => "B".to_owned(),
            3 => "G".to_owned(),
            4 => "D".to_owned(),
            5 => "A".to_owned(),
            6 => "E".to_owned(),
            string_number => string_number.to_string(),
        };

        write!(f, "{}", string_pitch_letter)
    }
}

#[derive(Debug)]
pub struct Guitar {
    pub tuning: BTreeMap<StringNumber, Pitch>,
    pub num_frets: usize,
    pub range: HashSet<Pitch>,
    pub string_ranges: BTreeMap<StringNumber, Vec<Pitch>>,
}
impl Guitar {
    fn create_string_range(open_string_pitch: &Pitch, num_frets: usize) -> Vec<Pitch> {
        let lowest_pitch_index = Pitch::iter().position(|x| &x == open_string_pitch).unwrap();

        Pitch::iter().collect::<Vec<_>>()[lowest_pitch_index..=lowest_pitch_index + num_frets]
            .to_vec()
    }

    pub fn new(
        tuning: BTreeMap<StringNumber, Pitch>,
        num_frets: usize,
    ) -> Result<Self, Box<dyn Error>> {
        const MAX_NUM_FRETS: usize = 18;
        if num_frets > MAX_NUM_FRETS {
            return Err(format!(
                "Too many frets ({}). The maximum is {}.",
                num_frets, MAX_NUM_FRETS
            )
            .into());
        }

        let mut string_ranges: BTreeMap<StringNumber, Vec<Pitch>> = BTreeMap::new();
        for (string_number, string_open_pitch) in tuning.iter() {
            string_ranges.insert(
                string_number.clone().to_owned(),
                Guitar::create_string_range(string_open_pitch, num_frets),
            );
        }
        dbg!(&tuning);
        dbg!(&string_ranges);

        let range = string_ranges.clone().into_iter().fold(
            HashSet::new(),
            |mut all_pitches, string_pitches| {
                all_pitches.extend(string_pitches.1);
                all_pitches
            },
        );

        Ok(Guitar {
            tuning,
            num_frets,
            range,
            string_ranges,
        })
    }
}

// pub struct Arrangement {}

// impl Arrangement {
//     pub fn new(guitar: Guitar, input_pitches: Vec<Vec<Pitch>>) -> Result<Self, Box<dyn Error>> {
//         for beat_pitches in input_pitches[0..1].to_vec() {
//             for beat_pitch in beat_pitches {
//                 // for string_range in guitar.string_ranges.as_array() {
//                 //     dbg!(string_range);
//                 // }
//                 // let x = guitar.string_ranges.e.iter().position(|&x| x == beat_pitch);
//                 let x = StringCollection {
//                     e: guitar.string_ranges.e.iter().position(|&x| x == beat_pitch),
//                     B: guitar.string_ranges.B.iter().position(|&x| x == beat_pitch),
//                     G: guitar.string_ranges.G.iter().position(|&x| x == beat_pitch),
//                     D: guitar.string_ranges.D.iter().position(|&x| x == beat_pitch),
//                     A: guitar.string_ranges.A.iter().position(|&x| x == beat_pitch),
//                     E: guitar.string_ranges.E.iter().position(|&x| x == beat_pitch),
//                 };

//                 dbg!(beat_pitch, x);
//             }
//             println!();
//         }

//         // dbg!(input_pitches);
//         // dbg!(guitar);
//         Ok(Arrangement {})
//     }
// }

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        dbg!(GuitarString::E);
        assert_eq!(result, 4);
    }
}
