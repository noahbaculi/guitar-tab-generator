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
            1 => "1 (e)".to_owned(),
            2 => "2 (B)".to_owned(),
            3 => "3 (G)".to_owned(),
            4 => "4 (D)".to_owned(),
            5 => "5 (A)".to_owned(),
            6 => "6 (E)".to_owned(),
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

    fn create_string_range(open_string_pitch: &Pitch, num_frets: usize) -> Vec<Pitch> {
        let lowest_pitch_index = Pitch::iter().position(|x| &x == open_string_pitch).unwrap();

        Pitch::iter().collect::<Vec<_>>()[lowest_pitch_index..=lowest_pitch_index + num_frets]
            .to_vec()
    }

    fn generate_pitch_fingering(&self, pitch: Pitch) -> Fingering {
        let mut fingering: BTreeMap<StringNumber, usize> = BTreeMap::new();
        for (string_number, string_range) in self.string_ranges.iter() {
            match string_range.iter().position(|x| x == &pitch) {
                None => (),
                Some(fret_number) => {
                    fingering.insert(string_number.clone().to_owned(), fret_number);
                }
            }
        }

        Fingering { pitch, fingering }
    }
}

#[derive(Debug)]
pub struct Fingering {
    pitch: Pitch,
    fingering: BTreeMap<StringNumber, usize>,
}

#[derive(Debug)]
pub struct InvalidInput {
    value: String,
    line_number: usize,
}
pub struct Arrangement {}

impl Arrangement {
    pub fn new(guitar: Guitar, input_pitches: Vec<Vec<Pitch>>) -> Result<Self, Box<dyn Error>> {
        let fingerings: Vec<Vec<Fingering>> = input_pitches[0..]
            .iter()
            .map(|beat_pitches| {
                beat_pitches
                    .iter()
                    .map(|beat_pitch| guitar.generate_pitch_fingering(beat_pitch.clone()))
                    .collect()
            })
            .collect();

        Arrangement::check_for_invalid_pitches(fingerings)?;

        Ok(Arrangement {})
    }

    fn check_for_invalid_pitches(fingerings: Vec<Vec<Fingering>>) -> Result<(), Box<dyn Error>> {
        let impossible_pitches: Vec<Vec<Pitch>> = fingerings
            .iter()
            .map(|beat_fingerings| {
                {
                    beat_fingerings
                        .iter()
                        .filter(|beat_fingering| beat_fingering.fingering.is_empty())
                        .map(|beat_fingering| beat_fingering.pitch)
                        .collect()
                }
            })
            .collect();
        let invalid_inputs: Vec<InvalidInput> = impossible_pitches
            .iter()
            .filter(|beat_impossible_pitches| !beat_impossible_pitches.is_empty())
            .flat_map(|beat_impossible_pitches| {
                let line_number = impossible_pitches
                    .iter()
                    .position(|x| x == beat_impossible_pitches)
                    .unwrap();

                beat_impossible_pitches
                    .iter()
                    .map(move |beat_impossible_pitch| InvalidInput {
                        value: format!("{:?}", beat_impossible_pitch),
                        line_number,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        if !invalid_inputs.is_empty() {
            let error_string = invalid_inputs
                .iter()
                .map(|invalid_input| {
                    format!(
                        "Invalid pitch {} on line {}.",
                        invalid_input.value, invalid_input.line_number
                    )
                })
                .collect::<Vec<String>>()
                .join("\n");

            return Err(error_string.into());
        }
        Ok(())
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
        assert_eq!(result, 4);
    }
}
