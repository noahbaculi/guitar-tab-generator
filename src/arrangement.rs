use crate::{guitar::Fingering, Guitar, Pitch};
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct InvalidInput {
    value: String,
    line_number: u8,
}

#[derive(Debug)]
pub struct Arrangement {}

impl Arrangement {
    pub fn new(guitar: Guitar, input_pitches: Vec<Vec<Pitch>>) -> Result<Self> {
        let fingerings: Vec<Vec<Fingering>> = input_pitches[0..]
            .iter()
            .map(|beat_pitches| {
                beat_pitches
                    .iter()
                    .map(|beat_pitch| {
                        Guitar::generate_pitch_fingering(&guitar.string_ranges, beat_pitch)
                    })
                    .collect()
            })
            .collect();

        Arrangement::check_for_invalid_pitches(fingerings)?;

        Ok(Arrangement {})
    }

    fn check_for_invalid_pitches(fingerings: Vec<Vec<Fingering>>) -> Result<()> {
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
                    .unwrap() as u8;

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

            return Err(anyhow!(error_string));
        }
        Ok(())
    }
}

#[cfg(test)]
mod test_check_for_invalid_pitches {
    use super::*;
    use crate::StringNumber;
    use std::collections::BTreeMap;

    #[test]
    fn valid_simple() {
        let fingerings = vec![vec![Fingering {
            pitch: Pitch::G3,
            fingering: BTreeMap::from([
                (StringNumber::new(3).unwrap(), 0),
                (StringNumber::new(4).unwrap(), 5),
                (StringNumber::new(5).unwrap(), 10),
            ]),
        }]];

        assert!(Arrangement::check_for_invalid_pitches(fingerings).is_ok());
    }
    #[test]
    fn valid_complex() {
        let fingerings = vec![
            vec![Fingering {
                pitch: Pitch::G3,
                fingering: BTreeMap::from([
                    (StringNumber::new(3).unwrap(), 0),
                    (StringNumber::new(4).unwrap(), 5),
                    (StringNumber::new(5).unwrap(), 10),
                    (StringNumber::new(6).unwrap(), 15),
                ]),
            }],
            vec![Fingering {
                pitch: Pitch::B3,
                fingering: BTreeMap::from([
                    (StringNumber::new(2).unwrap(), 0),
                    (StringNumber::new(3).unwrap(), 4),
                    (StringNumber::new(4).unwrap(), 9),
                    (StringNumber::new(5).unwrap(), 14),
                ]),
            }],
            vec![
                Fingering {
                    pitch: Pitch::D4,
                    fingering: BTreeMap::from([
                        (StringNumber::new(2).unwrap(), 3),
                        (StringNumber::new(3).unwrap(), 7),
                        (StringNumber::new(4).unwrap(), 12),
                        (StringNumber::new(5).unwrap(), 17),
                    ]),
                },
                Fingering {
                    pitch: Pitch::G4,
                    fingering: BTreeMap::from([
                        (StringNumber::new(1).unwrap(), 3),
                        (StringNumber::new(2).unwrap(), 8),
                        (StringNumber::new(3).unwrap(), 12),
                        (StringNumber::new(4).unwrap(), 17),
                    ]),
                },
            ],
        ];

        assert!(Arrangement::check_for_invalid_pitches(fingerings).is_ok());
    }
    #[test]
    fn invalid_simple() {
        let fingerings = vec![vec![
            Fingering {
                pitch: Pitch::G3,
                fingering: BTreeMap::from([
                    (StringNumber::new(3).unwrap(), 0),
                    (StringNumber::new(4).unwrap(), 5),
                    (StringNumber::new(5).unwrap(), 10),
                    (StringNumber::new(6).unwrap(), 15),
                ]),
            },
            Fingering {
                pitch: Pitch::CSharp6,
                fingering: BTreeMap::from([]),
            },
        ]];

        let expected_error_string = "Invalid pitch CSharp6 on line 0.";
        let error = Arrangement::check_for_invalid_pitches(fingerings).unwrap_err();
        let error_string = format!("{error}");

        assert_eq!(error_string, expected_error_string);
    }
    #[test]
    fn invalid_complex() {
        let fingerings = vec![
            vec![Fingering {
                pitch: Pitch::A1,
                fingering: BTreeMap::from([]),
            }],
            vec![Fingering {
                pitch: Pitch::G3,
                fingering: BTreeMap::from([
                    (StringNumber::new(3).unwrap(), 0),
                    (StringNumber::new(4).unwrap(), 5),
                    (StringNumber::new(5).unwrap(), 10),
                    (StringNumber::new(6).unwrap(), 15),
                ]),
            }],
            vec![Fingering {
                pitch: Pitch::B3,
                fingering: BTreeMap::from([
                    (StringNumber::new(2).unwrap(), 0),
                    (StringNumber::new(3).unwrap(), 4),
                    (StringNumber::new(4).unwrap(), 9),
                    (StringNumber::new(5).unwrap(), 14),
                ]),
            }],
            vec![
                Fingering {
                    pitch: Pitch::A1,
                    fingering: BTreeMap::from([]),
                },
                Fingering {
                    pitch: Pitch::B1,
                    fingering: BTreeMap::from([]),
                },
            ],
            vec![
                Fingering {
                    pitch: Pitch::G3,
                    fingering: BTreeMap::from([
                        (StringNumber::new(3).unwrap(), 0),
                        (StringNumber::new(4).unwrap(), 5),
                        (StringNumber::new(5).unwrap(), 10),
                        (StringNumber::new(6).unwrap(), 15),
                    ]),
                },
                Fingering {
                    pitch: Pitch::D2,
                    fingering: BTreeMap::from([]),
                },
            ],
            vec![
                Fingering {
                    pitch: Pitch::D4,
                    fingering: BTreeMap::from([
                        (StringNumber::new(2).unwrap(), 3),
                        (StringNumber::new(3).unwrap(), 7),
                        (StringNumber::new(4).unwrap(), 12),
                        (StringNumber::new(5).unwrap(), 17),
                    ]),
                },
                Fingering {
                    pitch: Pitch::G4,
                    fingering: BTreeMap::from([
                        (StringNumber::new(1).unwrap(), 3),
                        (StringNumber::new(2).unwrap(), 8),
                        (StringNumber::new(3).unwrap(), 12),
                        (StringNumber::new(4).unwrap(), 17),
                    ]),
                },
            ],
        ];

        let expected_error_string = "Invalid pitch A1 on line 0.\nInvalid pitch A1 on line 3.\nInvalid pitch B1 on line 3.\nInvalid pitch D2 on line 4.";
        let error = Arrangement::check_for_invalid_pitches(fingerings).unwrap_err();
        let error_string = format!("{error}");

        assert_eq!(error_string, expected_error_string);
    }
}
