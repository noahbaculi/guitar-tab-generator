use std::error::Error;

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, EnumIter)]
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

#[derive(Debug)]
pub enum GuitarString {
    #[allow(non_camel_case_types)]
    e = 1,
    B = 2,
    G = 3,
    D = 4,
    A = 5,
    E = 6,
}
#[derive(Debug, Clone)]
#[allow(non_snake_case)]
pub struct StringCollection<T> {
    pub e: T,
    pub B: T,
    pub G: T,
    pub D: T,
    pub A: T,
    pub E: T,
}
impl<T: std::cmp::Ord + Clone> StringCollection<T> {
    pub fn as_array(&self) -> [&T; 6] {
        [&self.e, &self.B, &self.G, &self.D, &self.A, &self.E]
    }
    pub fn into_array(self) -> [T; 6] {
        [self.e, self.B, self.G, self.D, self.A, self.E]
    }
}

#[derive(Debug)]
pub struct Fingering {
    pub guitar_string: GuitarString,
    pub fret: u8,
}

#[derive(Debug, Clone)]
pub struct InvalidFretError {
    pub fret: u8,
}

impl Fingering {
    pub fn new(guitar_string: GuitarString, fret: u8) -> Result<Self, InvalidFretError> {
        match fret {
            0..=18 => Ok(Fingering {
                guitar_string,
                fret,
            }),
            _ => Err(InvalidFretError { fret }),
        }
    }
}

// use anyhow::Result;

#[derive(Debug)]
pub struct Guitar {
    pub tuning: StringCollection<Pitch>,
    pub num_frets: usize,
    pub range: Vec<Pitch>,
    pub string_ranges: StringCollection<Vec<Pitch>>,
}
impl Guitar {
    fn create_string_range(open_string_pitch: &Pitch, num_frets: usize) -> Vec<Pitch> {
        let lowest_pitch_index = Pitch::iter().position(|x| &x == open_string_pitch).unwrap();

        Pitch::iter().collect::<Vec<_>>()[lowest_pitch_index..=lowest_pitch_index + num_frets]
            .to_vec()
    }

    pub fn new(tuning: StringCollection<Pitch>, num_frets: usize) -> Result<Self, Box<dyn Error>> {
        let max_num_frets = 18;
        if num_frets > max_num_frets {
            return Err(format!(
                "Too many frets ({}). The maximum is {}.",
                num_frets, max_num_frets
            )
            .into());
        }

        let all_pitches = Pitch::iter();

        let lowest_pitch = tuning.as_array().into_iter().min().unwrap().to_owned();
        let lowest_pitch_index = all_pitches.clone().position(|x| x == lowest_pitch).unwrap();

        let highest_pitch = tuning.as_array().into_iter().max().unwrap().to_owned();
        let highest_pitch_index = all_pitches
            .clone()
            .position(|x| x == highest_pitch)
            .unwrap();

        let range =
            &all_pitches.collect::<Vec<_>>()[lowest_pitch_index..=highest_pitch_index + num_frets];

        let string_ranges = StringCollection {
            e: Guitar::create_string_range(&tuning.e, num_frets),
            B: Guitar::create_string_range(&tuning.B, num_frets),
            G: Guitar::create_string_range(&tuning.G, num_frets),
            D: Guitar::create_string_range(&tuning.D, num_frets),
            A: Guitar::create_string_range(&tuning.A, num_frets),
            E: Guitar::create_string_range(&tuning.E, num_frets),
        };

        // TODO range should be the unique elements from string_ranges

        Ok(Guitar {
            tuning,
            num_frets,
            range: range.to_vec(),
            string_ranges,
        })
    }
}

pub struct Arrangement {}

impl Arrangement {
    pub fn new(guitar: Guitar, input_pitches: Vec<Vec<Pitch>>) -> Result<Self, Box<dyn Error>> {
        dbg!(input_pitches);
        dbg!(guitar);
        Ok(Arrangement {})
    }
}

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
