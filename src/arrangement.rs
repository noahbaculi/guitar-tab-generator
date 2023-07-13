use std::collections::HashSet;

use crate::{
    guitar::{generate_pitch_fingerings, Fingering},
    Guitar, Pitch,
};
use anyhow::{anyhow, Result};
use itertools::Itertools;

#[derive(Debug)]
pub struct InvalidInput {
    value: String,
    line_number: u16,
}

// #[derive(Debug, Clone)]
// pub struct BeatFingering {
//     fingerings: BeatVec<Fingering>,
//     non_zero_avg_fret: f32,
// }

// TODO! filter unplayable fingering options from beat_fingering_candidates (based
// on the fret span and whether there are any candidates with smaller fret
// spans)
// TODO! let non_zero_fret_avg = non_zero_frets.iter().sum::<usize>() as f32 /
// non_zero_frets.len() as f32;
// TODO! pathfinding (https://docs.rs/pathfinding/latest/pathfinding/)
// TODO! investigate property testing (https://altsysrq.github.io/proptest-book/)
// TODO! benchmarking via Criterion (https://crates.io/crates/criterion)

pub type PitchVec<T> = Vec<T>;
type BeatVec<T> = Vec<T>;

#[derive(Debug)]
pub struct Arrangement {}

impl Arrangement {
    pub fn new(guitar: Guitar, input_pitches: Vec<BeatVec<Pitch>>) -> Result<Self> {
        let pitch_fingering_candidates: Vec<BeatVec<PitchVec<Fingering>>> =
            validate_fingerings(&guitar, &input_pitches)?;

        // let x: Vec<_> = vec![vec![1, 2], vec![10, 20], vec![100, 200]]
        //     .into_iter()
        //     .multi_cartesian_product()
        //     .collect();
        // dbg!(&x);
        // dbg!(&pitch_fingering_candidates);

        const PLAYABLE_FRET_SPAN: u8 = 5;
        let _beat_fingering_candidates = pitch_fingering_candidates
            .iter()
            .map(|beat_pitch_candidates| {
                beat_pitch_candidates
                    .iter()
                    .multi_cartesian_product()
                    .filter(no_duplicate_strings)
                    .filter(|beat_fingering_option| {
                        calc_fret_span(beat_fingering_option).unwrap_or(0) <= PLAYABLE_FRET_SPAN
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Ok(Arrangement {})
    }
}

/// Generates fingerings for each pitch, and returns a result containing the fingerings or
/// an error message if any impossible pitches (with no fingerings) are found.
///
/// Arguments:
///
/// * `guitar`: A reference to a `Guitar` object, which contains information about the guitar's
/// string ranges.
/// * `input_pitches`: A slice of vectors, where each vector represents a beat and contains a
/// vector of pitches.
///
/// Returns:
///
/// The function `validate_fingerings` returns a `Result` containing either a
/// `Vec<Vec<Vec<Fingering>>>` if the input pitches are valid, or an `Err` containing an error
/// message if there are invalid pitches.
fn validate_fingerings(
    guitar: &Guitar,
    input_pitches: &[BeatVec<Pitch>],
) -> Result<Vec<BeatVec<PitchVec<Fingering>>>> {
    let mut impossible_pitches: Vec<InvalidInput> = vec![];
    let fingerings: Vec<BeatVec<PitchVec<Fingering>>> = input_pitches[0..]
        .iter()
        .enumerate()
        .map(|(beat_index, beat_pitches)| {
            beat_pitches
                .iter()
                .map(|beat_pitch| {
                    let pitch_fingerings: PitchVec<Fingering> =
                        generate_pitch_fingerings(&guitar.string_ranges, beat_pitch);
                    if pitch_fingerings.is_empty() {
                        impossible_pitches.push(InvalidInput {
                            value: format!("{:?}", beat_pitch),
                            line_number: (beat_index as u16) + 1,
                        })
                    }
                    pitch_fingerings
                })
                .collect()
        })
        .collect();

    if !impossible_pitches.is_empty() {
        let error_string = impossible_pitches
            .iter()
            .map(|invalid_input| {
                format!(
                    "Pitch {} on line {} cannot be played on any strings of the configured guitar.",
                    invalid_input.value, invalid_input.line_number
                )
            })
            .collect::<Vec<String>>()
            .join("\n");

        return Err(anyhow!(error_string));
    }

    Ok(fingerings)
}
#[cfg(test)]
mod test_validate_fingerings {
    use super::*;
    use crate::StringNumber;
    use std::collections::{BTreeMap, HashSet};

    fn generate_standard_guitar() -> Guitar {
        Guitar {
            tuning: BTreeMap::from([
                (StringNumber::new(1).unwrap(), Pitch::E4),
                (StringNumber::new(2).unwrap(), Pitch::B3),
                (StringNumber::new(3).unwrap(), Pitch::G3),
                (StringNumber::new(4).unwrap(), Pitch::D3),
                (StringNumber::new(5).unwrap(), Pitch::A2),
                (StringNumber::new(6).unwrap(), Pitch::E2),
            ]),
            num_frets: 12,
            range: HashSet::from([
                Pitch::E2,
                Pitch::F2,
                Pitch::FSharp2,
                Pitch::G2,
                Pitch::A2,
                Pitch::ASharp2,
                Pitch::B2,
                Pitch::C3,
                Pitch::D3,
                Pitch::DSharp3,
                Pitch::E3,
                Pitch::F3,
                Pitch::G3,
                Pitch::GSharp3,
                Pitch::A3,
                Pitch::ASharp3,
                Pitch::B3,
                Pitch::C4,
                Pitch::CSharp4,
                Pitch::D4,
                Pitch::E4,
                Pitch::F4,
                Pitch::FSharp4,
                Pitch::G4,
            ]),
            string_ranges: BTreeMap::from([
                (
                    StringNumber::new(1).unwrap(),
                    vec![Pitch::E4, Pitch::F4, Pitch::FSharp4, Pitch::G4],
                ),
                (
                    StringNumber::new(2).unwrap(),
                    vec![Pitch::B3, Pitch::C4, Pitch::CSharp4, Pitch::D4],
                ),
                (
                    StringNumber::new(3).unwrap(),
                    vec![Pitch::G3, Pitch::GSharp3, Pitch::A3, Pitch::ASharp3],
                ),
                (
                    StringNumber::new(4).unwrap(),
                    vec![Pitch::D3, Pitch::DSharp3, Pitch::E3, Pitch::F3],
                ),
                (
                    StringNumber::new(5).unwrap(),
                    vec![Pitch::A2, Pitch::ASharp2, Pitch::B2, Pitch::C3],
                ),
                (
                    StringNumber::new(6).unwrap(),
                    vec![Pitch::E2, Pitch::F2, Pitch::FSharp2, Pitch::G2],
                ),
            ]),
        }
    }

    #[test]
    fn valid_simple() {
        let guitar = generate_standard_guitar();
        let input_pitches = vec![vec![Pitch::G3]];
        let expected_fingerings = vec![vec![generate_pitch_fingerings(
            &guitar.string_ranges,
            &Pitch::G3,
        )]];

        assert_eq!(
            validate_fingerings(&guitar, &input_pitches).unwrap(),
            expected_fingerings
        );
    }
    #[test]
    fn valid_complex() {
        let guitar = generate_standard_guitar();
        let input_pitches = vec![vec![Pitch::G3], vec![Pitch::B3], vec![Pitch::D4, Pitch::G4]];
        let expected_fingerings = vec![
            vec![generate_pitch_fingerings(&guitar.string_ranges, &Pitch::G3)],
            vec![generate_pitch_fingerings(&guitar.string_ranges, &Pitch::B3)],
            vec![
                generate_pitch_fingerings(&guitar.string_ranges, &Pitch::D4),
                generate_pitch_fingerings(&guitar.string_ranges, &Pitch::G4),
            ],
        ];

        assert_eq!(
            validate_fingerings(&guitar, &input_pitches).unwrap(),
            expected_fingerings
        );
    }
    #[test]
    fn invalid_simple() {
        let guitar = generate_standard_guitar();
        let input_pitches = vec![vec![Pitch::B9]];

        let error = validate_fingerings(&guitar, &input_pitches).unwrap_err();
        let error_string = format!("{error}");
        let expected_error_string =
            "Pitch B9 on line 1 cannot be played on any strings of the configured guitar.";
        assert_eq!(error_string, expected_error_string);
    }
    #[test]
    fn invalid_complex() {
        let guitar = generate_standard_guitar();
        let input_pitches = vec![
            vec![Pitch::A1],
            vec![Pitch::G3],
            vec![Pitch::B3],
            vec![Pitch::A1, Pitch::B1],
            vec![Pitch::G3, Pitch::D2],
            vec![Pitch::D4, Pitch::G4],
        ];

        let error = validate_fingerings(&guitar, &input_pitches).unwrap_err();
        let error_string = format!("{error}");
        let expected_error_string =
            "Pitch A1 on line 1 cannot be played on any strings of the configured guitar.\n\
            Pitch A1 on line 4 cannot be played on any strings of the configured guitar.\n\
            Pitch B1 on line 4 cannot be played on any strings of the configured guitar.\n\
            Pitch D2 on line 5 cannot be played on any strings of the configured guitar.";
        assert_eq!(error_string, expected_error_string);
    }
}

/// Checks if there are any duplicate strings in a vector of `Fingering`
/// objects to ensure that all pitches can be played.
fn no_duplicate_strings(beat_fingering_option: &Vec<&Fingering>) -> bool {
    let num_pitches = beat_fingering_option.len();
    let num_strings = beat_fingering_option
        .iter()
        .map(|fingering| fingering.string_number)
        .collect::<HashSet<_>>()
        .len();

    num_pitches == num_strings
}
#[cfg(test)]
mod test_no_duplicate_strings {
    use super::*;
    use crate::StringNumber;

    #[test]
    fn valid_simple() {
        let fingering_1 = Fingering {
            pitch: Pitch::B6,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };
        let beat_fingering_option: &Vec<&Fingering> = &vec![&fingering_1];

        assert!(no_duplicate_strings(beat_fingering_option));
    }
    #[test]
    fn valid_complex() {
        let fingering_1 = Fingering {
            pitch: Pitch::CSharp2,
            string_number: StringNumber::new(1).unwrap(),
            fret: 1,
        };
        let fingering_2 = Fingering {
            pitch: Pitch::F4,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };
        let fingering_3 = Fingering {
            pitch: Pitch::A5,
            string_number: StringNumber::new(4).unwrap(),
            fret: 4,
        };
        let fingering_4 = Fingering {
            pitch: Pitch::DSharp6,
            string_number: StringNumber::new(11).unwrap(),
            fret: 0,
        };
        let beat_fingering_option: &Vec<&Fingering> =
            &vec![&fingering_1, &fingering_2, &fingering_3, &fingering_4];

        assert!(no_duplicate_strings(beat_fingering_option));
    }
    #[test]
    fn invalid_simple() {
        let fingering_1 = Fingering {
            pitch: Pitch::CSharp2,
            string_number: StringNumber::new(4).unwrap(),
            fret: 1,
        };
        let fingering_2 = Fingering {
            pitch: Pitch::F4,
            string_number: StringNumber::new(4).unwrap(),
            fret: 3,
        };
        let beat_fingering_option: &Vec<&Fingering> = &vec![&fingering_1, &fingering_2];

        assert!(!no_duplicate_strings(beat_fingering_option));
    }
    #[test]
    fn invalid_complex() {
        let fingering_1 = Fingering {
            pitch: Pitch::CSharp2,
            string_number: StringNumber::new(1).unwrap(),
            fret: 1,
        };
        let fingering_2 = Fingering {
            pitch: Pitch::F4,
            string_number: StringNumber::new(3).unwrap(),
            fret: 3,
        };
        let fingering_3 = Fingering {
            pitch: Pitch::A5,
            string_number: StringNumber::new(6).unwrap(),
            fret: 4,
        };
        let fingering_4 = Fingering {
            pitch: Pitch::DSharp6,
            string_number: StringNumber::new(3).unwrap(),
            fret: 0,
        };
        let beat_fingering_option: &Vec<&Fingering> =
            &vec![&fingering_1, &fingering_2, &fingering_3, &fingering_4];

        assert!(!no_duplicate_strings(beat_fingering_option));
    }
    #[test]
    fn empty_input() {
        assert!(no_duplicate_strings(&vec![]));
    }
}

fn calc_fret_span(beat_fingering_option: &Vec<&Fingering>) -> Option<u8> {
    let beat_fingering_option_fret_numbers = beat_fingering_option
        .iter()
        .filter(|fingering| fingering.fret != 0)
        .map(|fingering| fingering.fret);

    let min_non_zero_fret = match beat_fingering_option_fret_numbers.clone().min() {
        None => return None,
        Some(fret_num) => fret_num,
    };
    let max_non_zero_fret = match beat_fingering_option_fret_numbers.clone().max() {
        None => return None,
        Some(fret_num) => fret_num,
    };

    Some(max_non_zero_fret - min_non_zero_fret)
}
#[cfg(test)]
mod test_calc_fret_span {
    use super::*;
    use crate::StringNumber;

    #[test]
    fn simple() {
        let fingering_1 = Fingering {
            pitch: Pitch::B6,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };
        let beat_fingering_option: &Vec<&Fingering> = &vec![&fingering_1];

        assert_eq!(calc_fret_span(beat_fingering_option).unwrap(), 0);
    }
    #[test]
    fn complex() {
        let fingering_1 = Fingering {
            pitch: Pitch::CSharp2,
            string_number: StringNumber::new(1).unwrap(),
            fret: 1,
        };
        let fingering_2 = Fingering {
            pitch: Pitch::F4,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };
        let fingering_3 = Fingering {
            pitch: Pitch::A5,
            string_number: StringNumber::new(4).unwrap(),
            fret: 4,
        };
        let fingering_4 = Fingering {
            pitch: Pitch::DSharp6,
            string_number: StringNumber::new(11).unwrap(),
            fret: 0,
        };
        let beat_fingering_option: &Vec<&Fingering> =
            &vec![&fingering_1, &fingering_2, &fingering_3, &fingering_4];

        assert_eq!(calc_fret_span(beat_fingering_option).unwrap(), 3);
    }
    #[test]
    fn empty_input() {
        assert!(calc_fret_span(&vec![]).is_none());
    }
}
