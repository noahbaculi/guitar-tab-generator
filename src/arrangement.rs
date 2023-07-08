use crate::{guitar::Fingering, Guitar, Pitch};
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct InvalidInput {
    value: String,
    line_number: u16,
}

#[derive(Debug)]
pub struct Arrangement {}

impl Arrangement {
    pub fn new(guitar: Guitar, input_pitches: Vec<Vec<Pitch>>) -> Result<Self> {
        // TODO! add type alias for BeatVec, PitchVec, Candidates, ...
        // https://doc.rust-lang.org/book/ch19-04-advanced-types.html#creating-type-synonyms-with-type-aliases

        let pitch_fingering_options = Arrangement::validate_fingerings(&guitar, &input_pitches)?;
        dbg!(&pitch_fingering_options);

        Ok(Arrangement {})
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
    /// TODO! write tests
    fn validate_fingerings(
        guitar: &Guitar,
        input_pitches: &[Vec<Pitch>],
    ) -> Result<Vec<Vec<Vec<Fingering>>>> {
        let mut impossible_pitches: Vec<InvalidInput> = vec![];
        let fingerings: Vec<Vec<Vec<Fingering>>> = input_pitches[0..]
            .iter()
            .enumerate()
            .map(|(beat_index, beat_pitches)| {
                beat_pitches
                    .iter()
                    .map(|beat_pitch| {
                        let pitch_fingerings =
                            Guitar::generate_pitch_fingerings(&guitar.string_ranges, beat_pitch);
                        if pitch_fingerings.is_empty() {
                            impossible_pitches.push(InvalidInput {
                                value: format!("{:?}", beat_pitch),
                                line_number: beat_index as u16,
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
                        "Invalid pitch {} on line {}.",
                        invalid_input.value, invalid_input.line_number
                    )
                })
                .collect::<Vec<String>>()
                .join("\n");

            return Err(anyhow!(error_string));
        }

        Ok(fingerings)
    }
}

// #[cfg(test)]
// mod test_check_for_invalid_pitches {
//     use super::*;
//     use crate::StringNumber;
//     use std::collections::BTreeMap;

//     #[test]
//     fn valid_simple() {
//         let fingerings = vec![vec![PitchFingerings {
//             pitch: Pitch::G3,
//             fingering: BTreeMap::from([
//                 (StringNumber::new(3).unwrap(), 0),
//                 (StringNumber::new(4).unwrap(), 5),
//                 (StringNumber::new(5).unwrap(), 10),
//             ]),
//             non_zero_fret_avg: 0.0,
//         }]];

//         assert!(Arrangement::check_for_invalid_pitches(&fingerings).is_ok());
//     }
//     #[test]
//     fn valid_complex() {
//         let fingerings = vec![
//             vec![PitchFingerings {
//                 pitch: Pitch::G3,
//                 fingering: BTreeMap::from([
//                     (StringNumber::new(3).unwrap(), 0),
//                     (StringNumber::new(4).unwrap(), 5),
//                     (StringNumber::new(5).unwrap(), 10),
//                     (StringNumber::new(6).unwrap(), 15),
//                 ]),
//                 non_zero_fret_avg: 0.0,
//             }],
//             vec![PitchFingerings {
//                 pitch: Pitch::B3,
//                 fingering: BTreeMap::from([
//                     (StringNumber::new(2).unwrap(), 0),
//                     (StringNumber::new(3).unwrap(), 4),
//                     (StringNumber::new(4).unwrap(), 9),
//                     (StringNumber::new(5).unwrap(), 14),
//                 ]),
//                 non_zero_fret_avg: 0.0,
//             }],
//             vec![
//                 PitchFingerings {
//                     pitch: Pitch::D4,
//                     fingering: BTreeMap::from([
//                         (StringNumber::new(2).unwrap(), 3),
//                         (StringNumber::new(3).unwrap(), 7),
//                         (StringNumber::new(4).unwrap(), 12),
//                         (StringNumber::new(5).unwrap(), 17),
//                     ]),
//                     non_zero_fret_avg: 0.0,
//                 },
//                 PitchFingerings {
//                     pitch: Pitch::G4,
//                     fingering: BTreeMap::from([
//                         (StringNumber::new(1).unwrap(), 3),
//                         (StringNumber::new(2).unwrap(), 8),
//                         (StringNumber::new(3).unwrap(), 12),
//                         (StringNumber::new(4).unwrap(), 17),
//                     ]),
//                     non_zero_fret_avg: 0.0,
//                 },
//             ],
//         ];

//         assert!(Arrangement::check_for_invalid_pitches(&fingerings).is_ok());
//     }
//     #[test]
//     fn invalid_simple() {
//         let fingerings = vec![vec![
//             PitchFingerings {
//                 pitch: Pitch::G3,
//                 fingering: BTreeMap::from([
//                     (StringNumber::new(3).unwrap(), 0),
//                     (StringNumber::new(4).unwrap(), 5),
//                     (StringNumber::new(5).unwrap(), 10),
//                     (StringNumber::new(6).unwrap(), 15),
//                 ]),
//                 non_zero_fret_avg: 0.0,
//             },
//             PitchFingerings {
//                 pitch: Pitch::CSharp6,
//                 fingering: BTreeMap::from([]),
//                 non_zero_fret_avg: 0.0,
//             },
//         ]];

//         let expected_error_string = "Invalid pitch CSharp6 on line 0.";
//         let error = Arrangement::check_for_invalid_pitches(&fingerings).unwrap_err();
//         let error_string = format!("{error}");

//         assert_eq!(error_string, expected_error_string);
//     }
//     #[test]
//     fn invalid_complex() {
//         let fingerings = vec![
//             vec![PitchFingerings {
//                 pitch: Pitch::A1,
//                 fingering: BTreeMap::from([]),
//                 non_zero_fret_avg: 0.0,
//             }],
//             vec![PitchFingerings {
//                 pitch: Pitch::G3,
//                 fingering: BTreeMap::from([
//                     (StringNumber::new(3).unwrap(), 0),
//                     (StringNumber::new(4).unwrap(), 5),
//                     (StringNumber::new(5).unwrap(), 10),
//                     (StringNumber::new(6).unwrap(), 15),
//                 ]),
//                 non_zero_fret_avg: 0.0,
//             }],
//             vec![PitchFingerings {
//                 pitch: Pitch::B3,
//                 fingering: BTreeMap::from([
//                     (StringNumber::new(2).unwrap(), 0),
//                     (StringNumber::new(3).unwrap(), 4),
//                     (StringNumber::new(4).unwrap(), 9),
//                     (StringNumber::new(5).unwrap(), 14),
//                 ]),
//                 non_zero_fret_avg: 0.0,
//             }],
//             vec![
//                 PitchFingerings {
//                     pitch: Pitch::A1,
//                     fingering: BTreeMap::from([]),
//                     non_zero_fret_avg: 0.0,
//                 },
//                 PitchFingerings {
//                     pitch: Pitch::B1,
//                     fingering: BTreeMap::from([]),
//                     non_zero_fret_avg: 0.0,
//                 },
//             ],
//             vec![
//                 PitchFingerings {
//                     pitch: Pitch::G3,
//                     fingering: BTreeMap::from([
//                         (StringNumber::new(3).unwrap(), 0),
//                         (StringNumber::new(4).unwrap(), 5),
//                         (StringNumber::new(5).unwrap(), 10),
//                         (StringNumber::new(6).unwrap(), 15),
//                     ]),
//                     non_zero_fret_avg: 0.0,
//                 },
//                 PitchFingerings {
//                     pitch: Pitch::D2,
//                     fingering: BTreeMap::from([]),
//                     non_zero_fret_avg: 0.0,
//                 },
//             ],
//             vec![
//                 PitchFingerings {
//                     pitch: Pitch::D4,
//                     fingering: BTreeMap::from([
//                         (StringNumber::new(2).unwrap(), 3),
//                         (StringNumber::new(3).unwrap(), 7),
//                         (StringNumber::new(4).unwrap(), 12),
//                         (StringNumber::new(5).unwrap(), 17),
//                     ]),
//                     non_zero_fret_avg: 0.0,
//                 },
//                 PitchFingerings {
//                     pitch: Pitch::G4,
//                     fingering: BTreeMap::from([
//                         (StringNumber::new(1).unwrap(), 3),
//                         (StringNumber::new(2).unwrap(), 8),
//                         (StringNumber::new(3).unwrap(), 12),
//                         (StringNumber::new(4).unwrap(), 17),
//                     ]),
//                     non_zero_fret_avg: 0.0,
//                 },
//             ],
//         ];

//         let expected_error_string = "Invalid pitch A1 on line 0.\nInvalid pitch A1 on line 3.\nInvalid pitch B1 on line 3.\nInvalid pitch D2 on line 4.";
//         let error = Arrangement::check_for_invalid_pitches(&fingerings).unwrap_err();
//         let error_string = format!("{error}");

//         assert_eq!(error_string, expected_error_string);
//     }
// }
