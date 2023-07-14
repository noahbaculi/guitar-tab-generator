use crate::{arrangement::PitchVec, Pitch, StringNumber};
use anyhow::{anyhow, Result};
use std::{
    collections::{BTreeMap, HashSet},
    fmt,
};
use strum::IntoEnumIterator;

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PitchFingering {
    pub pitch: Pitch,
    pub string_number: StringNumber,
    pub fret: u8,
}
impl fmt::Debug for PitchFingering {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} | {:?} ⇒ {}",
            self.pitch, self.string_number, self.fret
        )
    }
}
#[cfg(test)]
mod test_pitch_fingering_debug {
    use super::*;
    #[test]
    fn simple() {
        let pitch_fingering = PitchFingering {
            pitch: Pitch::ASharp4,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };

        assert_eq!(format!("{:?}", pitch_fingering), "A#4 | 2_B ⇒ 3");
    }
}

#[derive(Debug, PartialEq)]
pub struct Guitar {
    pub tuning: BTreeMap<StringNumber, Pitch>,
    pub num_frets: u8,
    pub range: HashSet<Pitch>,
    pub string_ranges: BTreeMap<StringNumber, Vec<Pitch>>,
}
impl Guitar {
    pub fn new(tuning: BTreeMap<StringNumber, Pitch>, num_frets: u8) -> Result<Self> {
        check_fret_number(num_frets)?;

        let mut string_ranges: BTreeMap<StringNumber, Vec<Pitch>> = BTreeMap::new();
        for (string_number, string_open_pitch) in tuning.iter() {
            string_ranges.insert(
                string_number.clone().to_owned(),
                create_string_range(string_open_pitch, num_frets)?,
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
}
#[cfg(test)]
mod test_create_guitar {
    use super::*;

    fn create_default_tuning() -> BTreeMap<StringNumber, Pitch> {
        BTreeMap::from([
            (StringNumber::new(1).unwrap(), Pitch::E4),
            (StringNumber::new(2).unwrap(), Pitch::B3),
            (StringNumber::new(3).unwrap(), Pitch::G3),
            (StringNumber::new(4).unwrap(), Pitch::D3),
            (StringNumber::new(5).unwrap(), Pitch::A2),
            (StringNumber::new(6).unwrap(), Pitch::E2),
        ])
    }

    #[test]
    fn valid_simple() -> Result<()> {
        let tuning = create_default_tuning();

        const NUM_FRETS: u8 = 3;

        let expected_guitar = Guitar {
            tuning: tuning.clone(),
            num_frets: NUM_FRETS,
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
        };

        assert_eq!(Guitar::new(tuning, NUM_FRETS)?, expected_guitar);

        Ok(())
    }
    #[test]
    fn valid_normal() -> Result<()> {
        let tuning = create_default_tuning();

        const NUM_FRETS: u8 = 18;

        let expected_guitar = Guitar {
            tuning: tuning.clone(),
            num_frets: NUM_FRETS,
            range: HashSet::from([
                Pitch::E2,
                Pitch::F2,
                Pitch::FSharp2,
                Pitch::G2,
                Pitch::GSharp2,
                Pitch::A2,
                Pitch::ASharp2,
                Pitch::B2,
                Pitch::C3,
                Pitch::CSharp3,
                Pitch::D3,
                Pitch::DSharp3,
                Pitch::E3,
                Pitch::F3,
                Pitch::FSharp3,
                Pitch::G3,
                Pitch::GSharp3,
                Pitch::A3,
                Pitch::ASharp3,
                Pitch::B3,
                Pitch::C4,
                Pitch::CSharp4,
                Pitch::D4,
                Pitch::DSharp4,
                Pitch::E4,
                Pitch::F4,
                Pitch::FSharp4,
                Pitch::G4,
                Pitch::GSharp4,
                Pitch::A4,
                Pitch::ASharp4,
                Pitch::B4,
                Pitch::C5,
                Pitch::CSharp5,
                Pitch::D5,
                Pitch::DSharp5,
                Pitch::E5,
                Pitch::F5,
                Pitch::FSharp5,
                Pitch::G5,
                Pitch::GSharp5,
                Pitch::A5,
                Pitch::ASharp5,
            ]),
            string_ranges: BTreeMap::from([
                (
                    StringNumber::new(1).unwrap(),
                    vec![
                        Pitch::E4,
                        Pitch::F4,
                        Pitch::FSharp4,
                        Pitch::G4,
                        Pitch::GSharp4,
                        Pitch::A4,
                        Pitch::ASharp4,
                        Pitch::B4,
                        Pitch::C5,
                        Pitch::CSharp5,
                        Pitch::D5,
                        Pitch::DSharp5,
                        Pitch::E5,
                        Pitch::F5,
                        Pitch::FSharp5,
                        Pitch::G5,
                        Pitch::GSharp5,
                        Pitch::A5,
                        Pitch::ASharp5,
                    ],
                ),
                (
                    StringNumber::new(2).unwrap(),
                    vec![
                        Pitch::B3,
                        Pitch::C4,
                        Pitch::CSharp4,
                        Pitch::D4,
                        Pitch::DSharp4,
                        Pitch::E4,
                        Pitch::F4,
                        Pitch::FSharp4,
                        Pitch::G4,
                        Pitch::GSharp4,
                        Pitch::A4,
                        Pitch::ASharp4,
                        Pitch::B4,
                        Pitch::C5,
                        Pitch::CSharp5,
                        Pitch::D5,
                        Pitch::DSharp5,
                        Pitch::E5,
                        Pitch::F5,
                    ],
                ),
                (
                    StringNumber::new(3).unwrap(),
                    vec![
                        Pitch::G3,
                        Pitch::GSharp3,
                        Pitch::A3,
                        Pitch::ASharp3,
                        Pitch::B3,
                        Pitch::C4,
                        Pitch::CSharp4,
                        Pitch::D4,
                        Pitch::DSharp4,
                        Pitch::E4,
                        Pitch::F4,
                        Pitch::FSharp4,
                        Pitch::G4,
                        Pitch::GSharp4,
                        Pitch::A4,
                        Pitch::ASharp4,
                        Pitch::B4,
                        Pitch::C5,
                        Pitch::CSharp5,
                    ],
                ),
                (
                    StringNumber::new(4).unwrap(),
                    vec![
                        Pitch::D3,
                        Pitch::DSharp3,
                        Pitch::E3,
                        Pitch::F3,
                        Pitch::FSharp3,
                        Pitch::G3,
                        Pitch::GSharp3,
                        Pitch::A3,
                        Pitch::ASharp3,
                        Pitch::B3,
                        Pitch::C4,
                        Pitch::CSharp4,
                        Pitch::D4,
                        Pitch::DSharp4,
                        Pitch::E4,
                        Pitch::F4,
                        Pitch::FSharp4,
                        Pitch::G4,
                        Pitch::GSharp4,
                    ],
                ),
                (
                    StringNumber::new(5).unwrap(),
                    vec![
                        Pitch::A2,
                        Pitch::ASharp2,
                        Pitch::B2,
                        Pitch::C3,
                        Pitch::CSharp3,
                        Pitch::D3,
                        Pitch::DSharp3,
                        Pitch::E3,
                        Pitch::F3,
                        Pitch::FSharp3,
                        Pitch::G3,
                        Pitch::GSharp3,
                        Pitch::A3,
                        Pitch::ASharp3,
                        Pitch::B3,
                        Pitch::C4,
                        Pitch::CSharp4,
                        Pitch::D4,
                        Pitch::DSharp4,
                    ],
                ),
                (
                    StringNumber::new(6).unwrap(),
                    vec![
                        Pitch::E2,
                        Pitch::F2,
                        Pitch::FSharp2,
                        Pitch::G2,
                        Pitch::GSharp2,
                        Pitch::A2,
                        Pitch::ASharp2,
                        Pitch::B2,
                        Pitch::C3,
                        Pitch::CSharp3,
                        Pitch::D3,
                        Pitch::DSharp3,
                        Pitch::E3,
                        Pitch::F3,
                        Pitch::FSharp3,
                        Pitch::G3,
                        Pitch::GSharp3,
                        Pitch::A3,
                        Pitch::ASharp3,
                    ],
                ),
            ]),
        };

        assert_eq!(Guitar::new(tuning, NUM_FRETS)?, expected_guitar);

        Ok(())
    }
    #[test]
    fn invalid_num_frets() {
        assert!(Guitar::new(create_default_tuning(), 35).is_err());
    }
}

/// Check if the number of frets is within a maximum limit and returns an error if it exceeds the limit.
fn check_fret_number(num_frets: u8) -> Result<()> {
    const MAX_NUM_FRETS: u8 = 30;
    if num_frets > MAX_NUM_FRETS {
        return Err(anyhow!(
            "Too many frets ({num_frets}). The maximum is {MAX_NUM_FRETS}."
        ));
    }

    Ok(())
}
#[cfg(test)]
mod test_check_fret_number {
    use super::*;
    #[test]
    fn valid() {
        assert!(check_fret_number(0).is_ok());
        assert!(check_fret_number(2).is_ok());
        assert!(check_fret_number(7).is_ok());
        assert!(check_fret_number(20).is_ok());
    }
    #[test]
    fn invalid() {
        assert!(check_fret_number(0).is_ok());
        assert!(check_fret_number(12).is_ok());
        assert!(check_fret_number(18).is_ok());
        assert!(check_fret_number(27).is_ok());
        assert!(check_fret_number(31).is_err());
        assert!(check_fret_number(100).is_err());
    }
}

/// Generates a vector of pitches representing the range of the string.
///
/// Arguments:
///
/// * `open_string_pitch`: The `open_string_pitch` parameter represents the pitch of the open
/// string.
/// * `num_frets`: The `num_frets` parameter represents the number of
///   subsequent number of half steps to include in the range.
fn create_string_range(open_string_pitch: &Pitch, num_frets: u8) -> Result<Vec<Pitch>> {
    let lowest_pitch_index = Pitch::iter().position(|x| &x == open_string_pitch).unwrap();

    let all_pitches_vec: Vec<Pitch> = Pitch::iter().collect();
    let string_range_result =
        all_pitches_vec.get(lowest_pitch_index..=lowest_pitch_index + num_frets as usize);

    match string_range_result {
        Some(string_range_slice) => Ok(string_range_slice.to_vec()),
        None => {
            let highest_pitch = all_pitches_vec
                .last()
                .expect("The Pitch enum should not be empty.");
            let highest_pitch_fret = highest_pitch.index() - open_string_pitch.index();
            let err_msg = format!("Too many frets ({num_frets}) for string starting at pitch {open_string_pitch}. \
                The highest pitch is {highest_pitch}, which would only exist at fret number {highest_pitch_fret}.");

            Err(anyhow!(err_msg))
        }
    }
}
#[cfg(test)]
mod test_create_string_range {
    use super::*;
    #[test]
    fn valid() -> Result<()> {
        assert_eq!(create_string_range(&Pitch::E2, 0)?, vec![Pitch::E2]);
        assert_eq!(
            create_string_range(&Pitch::E2, 3)?,
            vec![Pitch::E2, Pitch::F2, Pitch::FSharp2, Pitch::G2]
        );
        assert_eq!(
            create_string_range(&Pitch::E2, 12)?,
            vec![
                Pitch::E2,
                Pitch::F2,
                Pitch::FSharp2,
                Pitch::G2,
                Pitch::GSharp2,
                Pitch::A2,
                Pitch::ASharp2,
                Pitch::B2,
                Pitch::C3,
                Pitch::CSharp3,
                Pitch::D3,
                Pitch::DSharp3,
                Pitch::E3
            ]
        );
        Ok(())
    }
    #[test]
    fn invalid() {
        let error = create_string_range(&Pitch::G9, 5).unwrap_err();
        let error_string = format!("{error}");
        let expected_error_string = "Too many frets (5) for string starting at pitch G9. The highest pitch is B9, which would only exist at fret number 4.";
        assert_eq!(error_string, expected_error_string);

        let error = create_string_range(&Pitch::E2, 100).unwrap_err();
        let error_string = format!("{error}");
        let expected_error_string = "Too many frets (100) for string starting at pitch E2. The highest pitch is B9, which would only exist at fret number 91.";
        assert_eq!(error_string, expected_error_string);
    }
}

/// Takes a pitch as input and returns the fingerings for that pitch on each
///string of the guitar given its tuning.
///
/// If no fingerings are possible on any of the strings of the guitar, an
/// empty vector is returned.
// TODO benchmark memoization
pub fn generate_pitch_fingerings(
    string_ranges: &BTreeMap<StringNumber, Vec<Pitch>>,
    pitch: &Pitch,
) -> PitchVec<PitchFingering> {
    let fingerings: PitchVec<PitchFingering> = string_ranges
        .iter()
        .filter_map(|(string_number, string_range)| {
            string_range
                .iter()
                .position(|x| x == pitch)
                .map(|fret_number| PitchFingering {
                    pitch: *pitch,
                    string_number: *string_number,
                    fret: fret_number as u8,
                })
        })
        .collect();
    // dbg!(&fingerings);

    // let non_zero_fret_avg =
    //     non_zero_frets.iter().sum::<usize>() as f32 / non_zero_frets.len() as f32;

    fingerings
}
#[cfg(test)]
mod test_generate_pitch_fingering {
    use super::*;

    #[test]
    fn valid_normal() -> Result<()> {
        const NUM_FRETS: u8 = 12;
        let string_ranges = BTreeMap::from([
            (
                StringNumber::new(1).unwrap(),
                create_string_range(&Pitch::E4, NUM_FRETS)?,
            ),
            (
                StringNumber::new(2).unwrap(),
                create_string_range(&Pitch::B3, NUM_FRETS)?,
            ),
            (
                StringNumber::new(3).unwrap(),
                create_string_range(&Pitch::G3, NUM_FRETS)?,
            ),
            (
                StringNumber::new(4).unwrap(),
                create_string_range(&Pitch::D3, NUM_FRETS)?,
            ),
            (
                StringNumber::new(5).unwrap(),
                create_string_range(&Pitch::A2, NUM_FRETS)?,
            ),
            (
                StringNumber::new(6).unwrap(),
                create_string_range(&Pitch::E2, NUM_FRETS)?,
            ),
        ]);

        assert_eq!(
            generate_pitch_fingerings(&string_ranges, &Pitch::E2),
            vec![PitchFingering {
                pitch: Pitch::E2,
                string_number: StringNumber::new(6).unwrap(),
                fret: 0
            }]
        );
        assert_eq!(
            generate_pitch_fingerings(&string_ranges, &Pitch::D3),
            vec![
                PitchFingering {
                    pitch: Pitch::D3,
                    string_number: StringNumber::new(4).unwrap(),
                    fret: 0
                },
                PitchFingering {
                    pitch: Pitch::D3,
                    string_number: StringNumber::new(5).unwrap(),
                    fret: 5
                },
                PitchFingering {
                    pitch: Pitch::D3,
                    string_number: StringNumber::new(6).unwrap(),
                    fret: 10
                }
            ]
        );
        assert_eq!(
            generate_pitch_fingerings(&string_ranges, &Pitch::CSharp4),
            vec![
                PitchFingering {
                    pitch: Pitch::CSharp4,
                    string_number: StringNumber::new(2).unwrap(),
                    fret: 2
                },
                PitchFingering {
                    pitch: Pitch::CSharp4,
                    string_number: StringNumber::new(3).unwrap(),
                    fret: 6
                },
                PitchFingering {
                    pitch: Pitch::CSharp4,
                    string_number: StringNumber::new(4).unwrap(),
                    fret: 11
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn valid_simple() -> Result<()> {
        const NUM_FRETS: u8 = 12;
        let string_ranges = BTreeMap::from([
            (
                StringNumber::new(1).unwrap(),
                create_string_range(&Pitch::G4, NUM_FRETS)?,
            ),
            (
                StringNumber::new(2).unwrap(),
                create_string_range(&Pitch::DSharp4, NUM_FRETS)?,
            ),
        ]);

        assert_eq!(
            generate_pitch_fingerings(&string_ranges, &Pitch::DSharp4),
            vec![PitchFingering {
                pitch: Pitch::DSharp4,
                string_number: StringNumber::new(2).unwrap(),
                fret: 0
            }]
        );
        assert_eq!(
            generate_pitch_fingerings(&string_ranges, &Pitch::ASharp4),
            vec![
                PitchFingering {
                    pitch: Pitch::ASharp4,
                    string_number: StringNumber::new(1).unwrap(),
                    fret: 3
                },
                PitchFingering {
                    pitch: Pitch::ASharp4,
                    string_number: StringNumber::new(2).unwrap(),
                    fret: 7
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn valid_few_frets() -> Result<()> {
        const NUM_FRETS: u8 = 2;
        let string_ranges = BTreeMap::from([
            (
                StringNumber::new(1).unwrap(),
                create_string_range(&Pitch::E4, NUM_FRETS)?,
            ),
            (
                StringNumber::new(2).unwrap(),
                create_string_range(&Pitch::B3, NUM_FRETS)?,
            ),
            (
                StringNumber::new(3).unwrap(),
                create_string_range(&Pitch::G3, NUM_FRETS)?,
            ),
            (
                StringNumber::new(4).unwrap(),
                create_string_range(&Pitch::D3, NUM_FRETS)?,
            ),
            (
                StringNumber::new(5).unwrap(),
                create_string_range(&Pitch::A2, NUM_FRETS)?,
            ),
            (
                StringNumber::new(6).unwrap(),
                create_string_range(&Pitch::E2, NUM_FRETS)?,
            ),
        ]);

        assert_eq!(
            generate_pitch_fingerings(&string_ranges, &Pitch::E3),
            vec![PitchFingering {
                pitch: Pitch::E3,
                string_number: StringNumber::new(4).unwrap(),
                fret: 2
            }]
        );
        Ok(())
    }

    #[test]
    fn valid_impossible_pitch() -> Result<()> {
        const NUM_FRETS: u8 = 12;
        let string_ranges = BTreeMap::from([
            (
                StringNumber::new(1).unwrap(),
                create_string_range(&Pitch::E4, NUM_FRETS)?,
            ),
            (
                StringNumber::new(2).unwrap(),
                create_string_range(&Pitch::B3, NUM_FRETS)?,
            ),
            (
                StringNumber::new(3).unwrap(),
                create_string_range(&Pitch::G3, NUM_FRETS)?,
            ),
            (
                StringNumber::new(4).unwrap(),
                create_string_range(&Pitch::D3, NUM_FRETS)?,
            ),
            (
                StringNumber::new(5).unwrap(),
                create_string_range(&Pitch::A2, NUM_FRETS)?,
            ),
            (
                StringNumber::new(6).unwrap(),
                create_string_range(&Pitch::E2, NUM_FRETS)?,
            ),
        ]);

        assert_eq!(
            generate_pitch_fingerings(&string_ranges, &Pitch::D2),
            vec![]
        );
        assert_eq!(
            generate_pitch_fingerings(&string_ranges, &Pitch::F5),
            vec![]
        );
        Ok(())
    }
}
