use crate::{arrangement::PitchVec, error::TabError, pitch::Pitch, string_number::StringNumber};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};
use strum::IntoEnumIterator;

/// The assignment of a single `Pitch` to a specific `StringNumber` and `fret` position.
///
/// The `Debug` impl renders as `"<pitch> | <string> => <fret>"` (e.g. `"A♯B♭4 | 2_B => 3"`).
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PitchFingering {
    pub(crate) string_number: StringNumber,
    pub(crate) fret: u8,
    pub(crate) pitch: Pitch,
}
impl PitchFingering {
    /// The guitar string the pitch is fretted on.
    #[inline]
    #[must_use]
    pub fn string_number(&self) -> StringNumber {
        self.string_number
    }

    /// The fret position, `0` for an open string.
    #[inline]
    #[must_use]
    pub fn fret(&self) -> u8 {
        self.fret
    }

    /// The sounding pitch.
    #[inline]
    #[must_use]
    pub fn pitch(&self) -> Pitch {
        self.pitch
    }
}
impl fmt::Debug for PitchFingering {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} | {:?} => {}",
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
            pitch: Pitch::ASharpBFlat4,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };

        assert_eq!(format!("{pitch_fingering:?}"), "A♯B♭4 | 2_B => 3");
    }
}

/// Open-string pitches for the standard 6-string guitar tuning, from string 1 (highest, E4)
/// to string 6 (lowest, E2).
pub(crate) const STD_6_STRING_TUNING_OPEN_PITCHES: [Pitch; 6] = [
    Pitch::E4,
    Pitch::B3,
    Pitch::G3,
    Pitch::D3,
    Pitch::A2,
    Pitch::E2,
];
/// Builds a tuning map from a slice of open-string pitches, numbering them from string 1
/// (highest) to string N (lowest).
///
/// # Errors
///
/// Returns an error if the input slice is longer than the maximum supported string count
/// (12), which would cause `StringNumber::new` to reject the generated numbers.
pub fn create_string_tuning(
    open_string_pitches: &[Pitch],
) -> Result<BTreeMap<StringNumber, Pitch>, TabError> {
    open_string_pitches
        .iter()
        .enumerate()
        .map(|(i, p)| {
            // `i + 1` is the 1-indexed string number. `StringNumber::new` owns the upper
            // bound and reports the offending number, so route every too-high case through
            // it. A slice longer than u8::MAX saturates to u8::MAX, which `new` still
            // rejects; in practice the collect short-circuits at string 13 (StringNumber::MAX
            // is 12) long before the cast could overflow.
            let string_number = u8::try_from(i + 1).unwrap_or(u8::MAX);
            StringNumber::new(string_number).map(|sn| (sn, *p))
        })
        .collect()
}

/// A guitar configuration: tuning, fret count, playable fret count and reachable pitch set,
/// and per-string ranges.
///
/// Construct with `Guitar::new` for validated input or `Guitar::default` for a standard
/// 18-fret 6-string instrument.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Guitar {
    pub(crate) tuning: BTreeMap<StringNumber, Pitch>,
    pub(crate) playable_frets: u8,
    pub(crate) range: BTreeSet<Pitch>,
    pub(crate) string_ranges: BTreeMap<StringNumber, Box<[Pitch]>>,
}
impl Default for Guitar {
    fn default() -> Guitar {
        let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES)
            .expect("BUG: standard tuning has 6 strings");
        Guitar::new(tuning, 18, 0).expect("BUG: Default guitar should be valid")
    }
}
impl Guitar {
    /// Upper bound on the fret count accepted by [`Guitar::new`].
    pub const MAX_NUM_FRETS: u8 = 30;
    /// Upper bound on the capo position accepted by [`Guitar::new`].
    pub const MAX_CAPO: u8 = 8;

    /// Constructs a validated `Guitar` from a tuning map, fret count, and capo position.
    ///
    /// The capo shifts every open-string pitch up by `capo` semitones and reduces the
    /// effective `num_frets` by the same amount.
    ///
    /// # Errors
    ///
    /// Returns a [`TabError`] variant for any of: fret count above [`Guitar::MAX_NUM_FRETS`],
    /// capo above [`Guitar::MAX_CAPO`], `capo > num_frets`, an open-string pitch shifted out
    /// of the supported `Pitch` range, or a string range that exceeds the highest pitch (`B9`).
    pub fn new(
        tuning: BTreeMap<StringNumber, Pitch>,
        num_frets: u8,
        capo: u8,
    ) -> Result<Self, TabError> {
        check_fret_number(num_frets)?;
        check_capo_number(capo)?;
        if capo > num_frets {
            return Err(TabError::CapoExceedsFrets { capo, num_frets });
        }
        let playable_frets = num_frets - capo;
        let adjusted_tuning = tuning
            .into_iter()
            .map(|(string_num, pitch)| -> Result<_, TabError> {
                let adjusted =
                    pitch
                        .plus_offset(capo as i16)
                        .ok_or(TabError::OpenPitchOutOfRange {
                            string: string_num.get(),
                            semitones: capo as i16,
                        })?;
                Ok((string_num, adjusted))
            })
            .collect::<Result<BTreeMap<_, _>, TabError>>()?;

        let mut string_ranges: BTreeMap<StringNumber, Box<[Pitch]>> = BTreeMap::new();
        for (string_number, string_open_pitch) in adjusted_tuning.iter() {
            string_ranges.insert(
                *string_number,
                create_string_range(string_open_pitch, playable_frets)?.into_boxed_slice(),
            );
        }

        let range =
            string_ranges
                .iter()
                .fold(BTreeSet::new(), |mut all_pitches, string_pitches| {
                    all_pitches.extend(string_pitches.1);
                    all_pitches
                });

        Ok(Guitar {
            tuning: adjusted_tuning,
            playable_frets,
            range,
            string_ranges,
        })
    }
}
#[cfg(test)]
mod test_create_guitar {
    use super::*;

    #[test]
    fn valid_simple() -> Result<(), TabError> {
        let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES)?;

        const NUM_FRETS: u8 = 3;

        let expected_guitar = Guitar {
            tuning: tuning.clone(),
            playable_frets: NUM_FRETS,
            range: BTreeSet::from([
                Pitch::E2,
                Pitch::F2,
                Pitch::FSharpGFlat2,
                Pitch::G2,
                Pitch::A2,
                Pitch::ASharpBFlat2,
                Pitch::B2,
                Pitch::C3,
                Pitch::D3,
                Pitch::DSharpEFlat3,
                Pitch::E3,
                Pitch::F3,
                Pitch::G3,
                Pitch::GSharpAFlat3,
                Pitch::A3,
                Pitch::ASharpBFlat3,
                Pitch::B3,
                Pitch::C4,
                Pitch::CSharpDFlat4,
                Pitch::D4,
                Pitch::E4,
                Pitch::F4,
                Pitch::FSharpGFlat4,
                Pitch::G4,
            ]),
            string_ranges: BTreeMap::from([
                (
                    StringNumber::new(1).unwrap(),
                    Box::from([Pitch::E4, Pitch::F4, Pitch::FSharpGFlat4, Pitch::G4]),
                ),
                (
                    StringNumber::new(2).unwrap(),
                    Box::from([Pitch::B3, Pitch::C4, Pitch::CSharpDFlat4, Pitch::D4]),
                ),
                (
                    StringNumber::new(3).unwrap(),
                    Box::from([
                        Pitch::G3,
                        Pitch::GSharpAFlat3,
                        Pitch::A3,
                        Pitch::ASharpBFlat3,
                    ]),
                ),
                (
                    StringNumber::new(4).unwrap(),
                    Box::from([Pitch::D3, Pitch::DSharpEFlat3, Pitch::E3, Pitch::F3]),
                ),
                (
                    StringNumber::new(5).unwrap(),
                    Box::from([Pitch::A2, Pitch::ASharpBFlat2, Pitch::B2, Pitch::C3]),
                ),
                (
                    StringNumber::new(6).unwrap(),
                    Box::from([Pitch::E2, Pitch::F2, Pitch::FSharpGFlat2, Pitch::G2]),
                ),
            ]),
        };

        assert_eq!(Guitar::new(tuning, NUM_FRETS, 0)?, expected_guitar);

        Ok(())
    }
    #[test]
    fn valid_simple_capo() -> Result<(), TabError> {
        let tuning = create_string_tuning(&[Pitch::E4, Pitch::B3, Pitch::G3])?;

        const NUM_FRETS: u8 = 18;
        const CAPO: u8 = 4;

        let expected_guitar = Guitar {
            tuning: create_string_tuning(&[Pitch::GSharpAFlat4, Pitch::DSharpEFlat4, Pitch::B3])?,
            playable_frets: NUM_FRETS - CAPO,
            range: BTreeSet::from([
                Pitch::G5,
                Pitch::D4,
                Pitch::A5,
                Pitch::CSharpDFlat5,
                Pitch::ASharpBFlat4,
                Pitch::B4,
                Pitch::GSharpAFlat4,
                Pitch::D5,
                Pitch::E4,
                Pitch::E5,
                Pitch::C5,
                Pitch::DSharpEFlat5,
                Pitch::DSharpEFlat4,
                Pitch::F4,
                Pitch::GSharpAFlat5,
                Pitch::G4,
                Pitch::C4,
                Pitch::ASharpBFlat5,
                Pitch::CSharpDFlat4,
                Pitch::B3,
                Pitch::FSharpGFlat4,
                Pitch::F5,
                Pitch::A4,
                Pitch::FSharpGFlat5,
            ]),
            string_ranges: BTreeMap::from([
                (
                    StringNumber::new(1).unwrap(),
                    Box::from([
                        Pitch::GSharpAFlat4,
                        Pitch::A4,
                        Pitch::ASharpBFlat4,
                        Pitch::B4,
                        Pitch::C5,
                        Pitch::CSharpDFlat5,
                        Pitch::D5,
                        Pitch::DSharpEFlat5,
                        Pitch::E5,
                        Pitch::F5,
                        Pitch::FSharpGFlat5,
                        Pitch::G5,
                        Pitch::GSharpAFlat5,
                        Pitch::A5,
                        Pitch::ASharpBFlat5,
                    ]),
                ),
                (
                    StringNumber::new(2).unwrap(),
                    Box::from([
                        Pitch::DSharpEFlat4,
                        Pitch::E4,
                        Pitch::F4,
                        Pitch::FSharpGFlat4,
                        Pitch::G4,
                        Pitch::GSharpAFlat4,
                        Pitch::A4,
                        Pitch::ASharpBFlat4,
                        Pitch::B4,
                        Pitch::C5,
                        Pitch::CSharpDFlat5,
                        Pitch::D5,
                        Pitch::DSharpEFlat5,
                        Pitch::E5,
                        Pitch::F5,
                    ]),
                ),
                (
                    StringNumber::new(3).unwrap(),
                    Box::from([
                        Pitch::B3,
                        Pitch::C4,
                        Pitch::CSharpDFlat4,
                        Pitch::D4,
                        Pitch::DSharpEFlat4,
                        Pitch::E4,
                        Pitch::F4,
                        Pitch::FSharpGFlat4,
                        Pitch::G4,
                        Pitch::GSharpAFlat4,
                        Pitch::A4,
                        Pitch::ASharpBFlat4,
                        Pitch::B4,
                        Pitch::C5,
                        Pitch::CSharpDFlat5,
                    ]),
                ),
            ]),
        };

        assert_eq!(Guitar::new(tuning, NUM_FRETS, CAPO)?, expected_guitar);

        Ok(())
    }
    #[test]
    fn valid_normal() -> Result<(), TabError> {
        let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES)?;

        const NUM_FRETS: u8 = 18;

        let expected_guitar = Guitar {
            tuning: tuning.clone(),
            playable_frets: NUM_FRETS,
            range: BTreeSet::from([
                Pitch::E2,
                Pitch::F2,
                Pitch::FSharpGFlat2,
                Pitch::G2,
                Pitch::GSharpAFlat2,
                Pitch::A2,
                Pitch::ASharpBFlat2,
                Pitch::B2,
                Pitch::C3,
                Pitch::CSharpDFlat3,
                Pitch::D3,
                Pitch::DSharpEFlat3,
                Pitch::E3,
                Pitch::F3,
                Pitch::FSharpGFlat3,
                Pitch::G3,
                Pitch::GSharpAFlat3,
                Pitch::A3,
                Pitch::ASharpBFlat3,
                Pitch::B3,
                Pitch::C4,
                Pitch::CSharpDFlat4,
                Pitch::D4,
                Pitch::DSharpEFlat4,
                Pitch::E4,
                Pitch::F4,
                Pitch::FSharpGFlat4,
                Pitch::G4,
                Pitch::GSharpAFlat4,
                Pitch::A4,
                Pitch::ASharpBFlat4,
                Pitch::B4,
                Pitch::C5,
                Pitch::CSharpDFlat5,
                Pitch::D5,
                Pitch::DSharpEFlat5,
                Pitch::E5,
                Pitch::F5,
                Pitch::FSharpGFlat5,
                Pitch::G5,
                Pitch::GSharpAFlat5,
                Pitch::A5,
                Pitch::ASharpBFlat5,
            ]),
            string_ranges: BTreeMap::from([
                (
                    StringNumber::new(1).unwrap(),
                    Box::from([
                        Pitch::E4,
                        Pitch::F4,
                        Pitch::FSharpGFlat4,
                        Pitch::G4,
                        Pitch::GSharpAFlat4,
                        Pitch::A4,
                        Pitch::ASharpBFlat4,
                        Pitch::B4,
                        Pitch::C5,
                        Pitch::CSharpDFlat5,
                        Pitch::D5,
                        Pitch::DSharpEFlat5,
                        Pitch::E5,
                        Pitch::F5,
                        Pitch::FSharpGFlat5,
                        Pitch::G5,
                        Pitch::GSharpAFlat5,
                        Pitch::A5,
                        Pitch::ASharpBFlat5,
                    ]),
                ),
                (
                    StringNumber::new(2).unwrap(),
                    Box::from([
                        Pitch::B3,
                        Pitch::C4,
                        Pitch::CSharpDFlat4,
                        Pitch::D4,
                        Pitch::DSharpEFlat4,
                        Pitch::E4,
                        Pitch::F4,
                        Pitch::FSharpGFlat4,
                        Pitch::G4,
                        Pitch::GSharpAFlat4,
                        Pitch::A4,
                        Pitch::ASharpBFlat4,
                        Pitch::B4,
                        Pitch::C5,
                        Pitch::CSharpDFlat5,
                        Pitch::D5,
                        Pitch::DSharpEFlat5,
                        Pitch::E5,
                        Pitch::F5,
                    ]),
                ),
                (
                    StringNumber::new(3).unwrap(),
                    Box::from([
                        Pitch::G3,
                        Pitch::GSharpAFlat3,
                        Pitch::A3,
                        Pitch::ASharpBFlat3,
                        Pitch::B3,
                        Pitch::C4,
                        Pitch::CSharpDFlat4,
                        Pitch::D4,
                        Pitch::DSharpEFlat4,
                        Pitch::E4,
                        Pitch::F4,
                        Pitch::FSharpGFlat4,
                        Pitch::G4,
                        Pitch::GSharpAFlat4,
                        Pitch::A4,
                        Pitch::ASharpBFlat4,
                        Pitch::B4,
                        Pitch::C5,
                        Pitch::CSharpDFlat5,
                    ]),
                ),
                (
                    StringNumber::new(4).unwrap(),
                    Box::from([
                        Pitch::D3,
                        Pitch::DSharpEFlat3,
                        Pitch::E3,
                        Pitch::F3,
                        Pitch::FSharpGFlat3,
                        Pitch::G3,
                        Pitch::GSharpAFlat3,
                        Pitch::A3,
                        Pitch::ASharpBFlat3,
                        Pitch::B3,
                        Pitch::C4,
                        Pitch::CSharpDFlat4,
                        Pitch::D4,
                        Pitch::DSharpEFlat4,
                        Pitch::E4,
                        Pitch::F4,
                        Pitch::FSharpGFlat4,
                        Pitch::G4,
                        Pitch::GSharpAFlat4,
                    ]),
                ),
                (
                    StringNumber::new(5).unwrap(),
                    Box::from([
                        Pitch::A2,
                        Pitch::ASharpBFlat2,
                        Pitch::B2,
                        Pitch::C3,
                        Pitch::CSharpDFlat3,
                        Pitch::D3,
                        Pitch::DSharpEFlat3,
                        Pitch::E3,
                        Pitch::F3,
                        Pitch::FSharpGFlat3,
                        Pitch::G3,
                        Pitch::GSharpAFlat3,
                        Pitch::A3,
                        Pitch::ASharpBFlat3,
                        Pitch::B3,
                        Pitch::C4,
                        Pitch::CSharpDFlat4,
                        Pitch::D4,
                        Pitch::DSharpEFlat4,
                    ]),
                ),
                (
                    StringNumber::new(6).unwrap(),
                    Box::from([
                        Pitch::E2,
                        Pitch::F2,
                        Pitch::FSharpGFlat2,
                        Pitch::G2,
                        Pitch::GSharpAFlat2,
                        Pitch::A2,
                        Pitch::ASharpBFlat2,
                        Pitch::B2,
                        Pitch::C3,
                        Pitch::CSharpDFlat3,
                        Pitch::D3,
                        Pitch::DSharpEFlat3,
                        Pitch::E3,
                        Pitch::F3,
                        Pitch::FSharpGFlat3,
                        Pitch::G3,
                        Pitch::GSharpAFlat3,
                        Pitch::A3,
                        Pitch::ASharpBFlat3,
                    ]),
                ),
            ]),
        };

        assert_eq!(Guitar::new(tuning, NUM_FRETS, 0)?, expected_guitar);

        Ok(())
    }

    #[test]
    fn capo_exceeds_num_frets_returns_typed_error() {
        let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES).unwrap();
        let err = Guitar::new(tuning, 2, 4).unwrap_err();
        match err {
            TabError::CapoExceedsFrets { capo, num_frets } => {
                assert_eq!(capo, 4);
                assert_eq!(num_frets, 2);
            }
            other => panic!("expected CapoExceedsFrets, got {other:?}"),
        }
    }

    #[test]
    fn open_pitch_out_of_range_returns_typed_error() {
        // A single string tuned to the top of the pitch range (B9); any capo offset pushes
        // the open pitch past B9, which `plus_offset` reports as `None`.
        let tuning = create_string_tuning(&[Pitch::B9]).unwrap();
        let err = Guitar::new(tuning, 8, 8).unwrap_err();
        match err {
            TabError::OpenPitchOutOfRange { string, semitones } => {
                assert_eq!(string, 1);
                assert_eq!(semitones, 8);
            }
            other => panic!("expected OpenPitchOutOfRange, got {other:?}"),
        }
    }
}

/// Validates that `num_frets` does not exceed [`Guitar::MAX_NUM_FRETS`].
fn check_fret_number(num_frets: u8) -> Result<(), TabError> {
    if num_frets > Guitar::MAX_NUM_FRETS {
        return Err(TabError::NumFretsTooHigh {
            num_frets,
            max: Guitar::MAX_NUM_FRETS,
        });
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
        assert!(check_fret_number(18).is_ok());
        assert!(check_fret_number(24).is_ok());
        assert!(check_fret_number(30).is_ok());
    }
    #[test]
    fn invalid() {
        assert!(check_fret_number(31).is_err());
        assert!(check_fret_number(100).is_err());
    }

    #[test]
    fn invalid_returns_typed_error() {
        let err = check_fret_number(31).unwrap_err();
        match err {
            TabError::NumFretsTooHigh { num_frets, max } => {
                assert_eq!(num_frets, 31);
                assert_eq!(max, 30);
            }
            other => panic!("expected NumFretsTooHigh, got {other:?}"),
        }
    }
}

/// Validates that `capo` does not exceed [`Guitar::MAX_CAPO`].
fn check_capo_number(capo: u8) -> Result<(), TabError> {
    if capo > Guitar::MAX_CAPO {
        return Err(TabError::CapoTooHigh {
            capo,
            max: Guitar::MAX_CAPO,
        });
    }
    Ok(())
}
#[cfg(test)]
mod test_check_capo_number {
    use super::*;
    #[test]
    fn valid() {
        assert!(check_capo_number(0).is_ok());
        assert!(check_capo_number(2).is_ok());
        assert!(check_capo_number(5).is_ok());
        assert!(check_capo_number(8).is_ok());
    }
    #[test]
    fn invalid() {
        assert!(check_capo_number(9).is_err());
        assert!(check_capo_number(12).is_err());
        assert!(check_capo_number(18).is_err());
        assert!(check_capo_number(27).is_err());
        assert!(check_capo_number(31).is_err());
        assert!(check_capo_number(100).is_err());
    }

    #[test]
    fn invalid_returns_typed_error() {
        let err = check_capo_number(9).unwrap_err();
        match err {
            TabError::CapoTooHigh { capo, max } => {
                assert_eq!(capo, 9);
                assert_eq!(max, 8);
            }
            other => panic!("expected CapoTooHigh, got {other:?}"),
        }
    }
}

/// Generates a vector of pitches representing the range of the string.
///
/// Arguments:
///
/// * `open_string_pitch`: The `open_string_pitch` parameter represents the pitch of the open
///   string.
/// * `playable_frets`: the playable fret count: the number of half steps above the open
///   pitch to include.
fn create_string_range(
    open_string_pitch: &Pitch,
    playable_frets: u8,
) -> Result<Vec<Pitch>, TabError> {
    let lowest_pitch_index = Pitch::iter().position(|x| &x == open_string_pitch).unwrap();
    let needed = playable_frets as usize + 1;

    let string_range: Vec<Pitch> = Pitch::iter()
        .skip(lowest_pitch_index)
        .take(needed)
        .collect();

    if string_range.len() == needed {
        Ok(string_range)
    } else {
        Err(TabError::FretRangeExceedsPitchRange {
            open_pitch: open_string_pitch.to_string(),
            playable_frets,
        })
    }
}
#[cfg(test)]
mod test_create_string_range {
    use super::*;
    #[test]
    fn valid() -> Result<(), TabError> {
        assert_eq!(create_string_range(&Pitch::E2, 0)?, vec![Pitch::E2]);
        assert_eq!(
            create_string_range(&Pitch::E2, 3)?,
            vec![Pitch::E2, Pitch::F2, Pitch::FSharpGFlat2, Pitch::G2]
        );
        assert_eq!(
            create_string_range(&Pitch::E2, 12)?,
            vec![
                Pitch::E2,
                Pitch::F2,
                Pitch::FSharpGFlat2,
                Pitch::G2,
                Pitch::GSharpAFlat2,
                Pitch::A2,
                Pitch::ASharpBFlat2,
                Pitch::B2,
                Pitch::C3,
                Pitch::CSharpDFlat3,
                Pitch::D3,
                Pitch::DSharpEFlat3,
                Pitch::E3
            ]
        );
        Ok(())
    }
    #[test]
    fn invalid_returns_typed_error() {
        let err = create_string_range(&Pitch::G9, 5).unwrap_err();
        match err {
            TabError::FretRangeExceedsPitchRange {
                open_pitch,
                playable_frets,
            } => {
                assert_eq!(open_pitch, "G9");
                assert_eq!(playable_frets, 5);
            }
            other => panic!("expected FretRangeExceedsPitchRange, got {other:?}"),
        }

        let err = create_string_range(&Pitch::E2, 100).unwrap_err();
        match err {
            TabError::FretRangeExceedsPitchRange {
                open_pitch,
                playable_frets,
            } => {
                assert_eq!(open_pitch, "E2");
                assert_eq!(playable_frets, 100);
            }
            other => panic!("expected FretRangeExceedsPitchRange, got {other:?}"),
        }
    }
}

/// Returns all playable `PitchFingering`s for `pitch` on the supplied `string_ranges`.
///
/// An empty vector indicates the pitch cannot be played on any string of this guitar.
#[must_use]
pub(crate) fn generate_pitch_fingerings(
    string_ranges: &BTreeMap<StringNumber, Box<[Pitch]>>,
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

    fingerings
}
#[cfg(test)]
mod test_generate_pitch_fingering {
    use super::*;

    #[test]
    fn valid_normal() -> Result<(), TabError> {
        const NUM_FRETS: u8 = 12;
        let string_ranges = BTreeMap::from([
            (
                StringNumber::new(1).unwrap(),
                create_string_range(&Pitch::E4, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(2).unwrap(),
                create_string_range(&Pitch::B3, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(3).unwrap(),
                create_string_range(&Pitch::G3, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(4).unwrap(),
                create_string_range(&Pitch::D3, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(5).unwrap(),
                create_string_range(&Pitch::A2, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(6).unwrap(),
                create_string_range(&Pitch::E2, NUM_FRETS)?.into_boxed_slice(),
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
            generate_pitch_fingerings(&string_ranges, &Pitch::CSharpDFlat4),
            vec![
                PitchFingering {
                    pitch: Pitch::CSharpDFlat4,
                    string_number: StringNumber::new(2).unwrap(),
                    fret: 2
                },
                PitchFingering {
                    pitch: Pitch::CSharpDFlat4,
                    string_number: StringNumber::new(3).unwrap(),
                    fret: 6
                },
                PitchFingering {
                    pitch: Pitch::CSharpDFlat4,
                    string_number: StringNumber::new(4).unwrap(),
                    fret: 11
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn valid_simple() -> Result<(), TabError> {
        const NUM_FRETS: u8 = 12;
        let string_ranges = BTreeMap::from([
            (
                StringNumber::new(1).unwrap(),
                create_string_range(&Pitch::G4, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(2).unwrap(),
                create_string_range(&Pitch::DSharpEFlat4, NUM_FRETS)?.into_boxed_slice(),
            ),
        ]);

        assert_eq!(
            generate_pitch_fingerings(&string_ranges, &Pitch::DSharpEFlat4),
            vec![PitchFingering {
                pitch: Pitch::DSharpEFlat4,
                string_number: StringNumber::new(2).unwrap(),
                fret: 0
            }]
        );
        assert_eq!(
            generate_pitch_fingerings(&string_ranges, &Pitch::ASharpBFlat4),
            vec![
                PitchFingering {
                    pitch: Pitch::ASharpBFlat4,
                    string_number: StringNumber::new(1).unwrap(),
                    fret: 3
                },
                PitchFingering {
                    pitch: Pitch::ASharpBFlat4,
                    string_number: StringNumber::new(2).unwrap(),
                    fret: 7
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn valid_few_frets() -> Result<(), TabError> {
        const NUM_FRETS: u8 = 2;
        let string_ranges = BTreeMap::from([
            (
                StringNumber::new(1).unwrap(),
                create_string_range(&Pitch::E4, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(2).unwrap(),
                create_string_range(&Pitch::B3, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(3).unwrap(),
                create_string_range(&Pitch::G3, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(4).unwrap(),
                create_string_range(&Pitch::D3, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(5).unwrap(),
                create_string_range(&Pitch::A2, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(6).unwrap(),
                create_string_range(&Pitch::E2, NUM_FRETS)?.into_boxed_slice(),
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
    fn valid_impossible_pitch() -> Result<(), TabError> {
        const NUM_FRETS: u8 = 12;
        let string_ranges = BTreeMap::from([
            (
                StringNumber::new(1).unwrap(),
                create_string_range(&Pitch::E4, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(2).unwrap(),
                create_string_range(&Pitch::B3, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(3).unwrap(),
                create_string_range(&Pitch::G3, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(4).unwrap(),
                create_string_range(&Pitch::D3, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(5).unwrap(),
                create_string_range(&Pitch::A2, NUM_FRETS)?.into_boxed_slice(),
            ),
            (
                StringNumber::new(6).unwrap(),
                create_string_range(&Pitch::E2, NUM_FRETS)?.into_boxed_slice(),
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

#[cfg(test)]
mod test_create_string_tuning_bounds {
    use super::*;

    #[test]
    fn over_long_slice_is_rejected() {
        // 13 open strings exceeds StringNumber::MAX (12). The Result-collect short-circuits
        // at the first over-max string number, so the function rejects rather than truncating.
        let pitches = vec![Pitch::E2; 13];
        let err = create_string_tuning(&pitches).unwrap_err();
        assert!(
            matches!(err, TabError::StringNumberOutOfRange { value: 13, max: 12 }),
            "got {err:?}"
        );
    }
}
