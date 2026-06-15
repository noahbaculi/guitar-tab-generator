//! Error types crossing the WASM boundary.
//!
//! `ParseError` is used both internally by the parser and as a leaf of `TabError::Parse`.
//! `TabError` is the tagged enum the WASM boundary throws on failure.
//!
//! `Display` and `Error` are hand-rolled rather than derived via `thiserror`. The wire
//! format is owned by `tsify` (which JS code matches on by `kind`), so the Rust
//! `Display` form is a developer-facing fallback only and doesn't justify an extra
//! transitive dependency.

use serde::Serialize;
use tsify::Tsify;

/// One unparseable substring in the input, with its 1-indexed line number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct ParseError {
    pub line: u32,
    pub text: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Input '{}' on line {} could not be parsed into a pitch.",
            self.text, self.line
        )
    }
}

/// A pitch that could not be played on the configured guitar, with its 1-indexed line number.
///
/// Public payload of [`TabError::UnplayablePitches`]. The structured `{ value, line }`
/// record replaced the free-form prose string used before 2.0.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct UnplayablePitch {
    pub value: String,
    pub line: u32,
}

impl std::fmt::Display for UnplayablePitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pitch {} on line {} cannot be played on any strings of the configured guitar.",
            self.value, self.line
        )
    }
}

/// Top-level error variant for the WASM boundary.
///
/// Additional variants may be added in a non-breaking release. The `#[non_exhaustive]`
/// attribute requires external matches to include a wildcard arm. JS consumers should keep a
/// `default` arm in any `switch (err.kind)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[non_exhaustive]
pub enum TabError {
    Parse {
        errors: Vec<ParseError>,
    },
    /// The input has more lines than the pathfinding graph can index. `max` is the inclusive
    /// line limit (`u16::MAX`). Distinct from [`TabError::Parse`] because no single line is at
    /// fault, so there is no `ParseError` line or text to report.
    InputTooManyLines {
        max: u32,
    },
    NumFretsTooHigh {
        num_frets: u8,
        max: u8,
    },
    CapoTooHigh {
        capo: u8,
        max: u8,
    },
    CapoExceedsFrets {
        capo: u8,
        num_frets: u8,
    },
    StringNumberOutOfRange {
        value: u8,
        max: u8,
    },
    /// `semitones` is `i16` (not `u8`) to mirror the offset arithmetic in [`crate::pitch::Pitch::plus_offset`]
    /// and to leave room for negative tuning offsets without a future breaking change. The
    /// 2.x emit site populates `0..=Guitar::MAX_CAPO` only.
    OpenPitchOutOfRange {
        string: u8,
        semitones: i16,
    },
    FretRangeExceedsPitchRange {
        open_pitch: String,
        playable_frets: u8,
    },
    UnplayablePitches {
        pitches: Vec<UnplayablePitch>,
    },
    NoArrangementsFound,
    NumArrangementsOutOfRange {
        value: u8,
        max: u8,
    },
    TuningNameUnknown {
        value: String,
    },
    IndexOutOfBounds {
        index: usize,
        len: usize,
    },
    /// The requested render `width` is below the minimum needed to lay out one beat at the
    /// given `padding`. `ArrangementSet::render` rejects widths below `min_render_width(padding)`.
    RenderWidthTooSmall {
        width: u16,
        min: u16,
    },
    /// A supplied difficulty weight was negative or non-finite. `field` names
    /// the offending coefficient.
    DifficultyWeightOutOfRange {
        field: &'static str,
    },
}

impl std::fmt::Display for TabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TabError::Parse { errors } => {
                if errors.is_empty() {
                    return write!(f, "Input could not be parsed.");
                }
                let joined = errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                write!(f, "{joined}")
            }
            TabError::InputTooManyLines { max } => {
                write!(f, "The input is too large. The maximum is {max} lines.")
            }
            TabError::NumFretsTooHigh { num_frets, max } => {
                write!(f, "Too many frets ({num_frets}). The maximum is {max}.")
            }
            TabError::CapoTooHigh { capo, max } => {
                write!(
                    f,
                    "The capo fret ({capo}) is too high. The maximum is {max}."
                )
            }
            TabError::CapoExceedsFrets { capo, num_frets } => {
                write!(
                    f,
                    "The capo fret ({capo}) cannot exceed the number of frets ({num_frets})."
                )
            }
            TabError::StringNumberOutOfRange { value, max } => {
                if *value == 0 {
                    write!(
                        f,
                        "A guitar cannot have a string number of zero (0). Guitar string numbering commences at one (1)."
                    )
                } else {
                    write!(
                        f,
                        "The string number ({value}) is too high. The maximum is {max}."
                    )
                }
            }
            TabError::OpenPitchOutOfRange { string, semitones } => {
                write!(
                    f,
                    "Capo offset of {semitones} semitones on string {string} would push the open pitch out of the supported range."
                )
            }
            TabError::FretRangeExceedsPitchRange {
                open_pitch,
                playable_frets,
            } => {
                write!(
                    f,
                    "Too many frets ({playable_frets}) for string starting at pitch {open_pitch}. The highest playable pitch is B9."
                )
            }
            TabError::UnplayablePitches { pitches } => {
                if pitches.is_empty() {
                    return write!(
                        f,
                        "Some pitches could not be played on the configured guitar."
                    );
                }
                let joined = pitches
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                write!(f, "{joined}")
            }
            TabError::NoArrangementsFound => {
                write!(f, "No arrangements could be calculated.")
            }
            TabError::NumArrangementsOutOfRange { value, max } => {
                write!(
                    f,
                    "The number of arrangements ({value}) must be between 1 and {max}."
                )
            }
            TabError::TuningNameUnknown { value } => {
                write!(
                    f,
                    "The tuning name ({value:?}) is not recognized. Use \"standard\" or another supported tuning name."
                )
            }
            TabError::IndexOutOfBounds { index, len } => {
                write!(f, "index {index} is out of bounds for set of length {len}")
            }
            TabError::RenderWidthTooSmall { width, min } => {
                write!(
                    f,
                    "The render width ({width}) is too small. The minimum is {min}."
                )
            }
            TabError::DifficultyWeightOutOfRange { field } => {
                write!(
                    f,
                    "The {field} difficulty weight must be a finite, non-negative number."
                )
            }
        }
    }
}

impl std::error::Error for TabError {}

#[cfg(test)]
mod test_parse_error_display {
    use super::*;

    #[test]
    fn reproduces_legacy_message_format() {
        let err = ParseError {
            line: 4,
            text: "BB.2".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "Input 'BB.2' on line 4 could not be parsed into a pitch."
        );
    }
}

#[cfg(test)]
mod test_tab_error_display {
    use super::*;

    #[test]
    fn parse_variant_joins_errors_with_newlines() {
        let err = TabError::Parse {
            errors: vec![
                ParseError {
                    line: 1,
                    text: "xyz".to_owned(),
                },
                ParseError {
                    line: 4,
                    text: "BB.2".to_owned(),
                },
            ],
        };
        assert_eq!(
            err.to_string(),
            "Input 'xyz' on line 1 could not be parsed into a pitch.\nInput 'BB.2' on line 4 could not be parsed into a pitch."
        );
    }

    #[test]
    fn parse_variant_with_empty_errors_falls_back_to_a_message() {
        let err = TabError::Parse { errors: vec![] };
        assert_eq!(err.to_string(), "Input could not be parsed.");
    }

    #[test]
    fn unplayable_pitches_with_empty_vec_falls_back_to_a_message() {
        let err = TabError::UnplayablePitches { pitches: vec![] };
        assert_eq!(
            err.to_string(),
            "Some pitches could not be played on the configured guitar."
        );
    }
}

#[cfg(test)]
mod test_new_variant_display {
    use super::*;

    #[test]
    fn input_too_many_lines() {
        let err = TabError::InputTooManyLines { max: 65535 };
        assert_eq!(
            err.to_string(),
            "The input is too large. The maximum is 65535 lines."
        );
    }

    #[test]
    fn num_frets_too_high() {
        let err = TabError::NumFretsTooHigh {
            num_frets: 31,
            max: 30,
        };
        assert_eq!(err.to_string(), "Too many frets (31). The maximum is 30.");
    }

    #[test]
    fn capo_too_high() {
        let err = TabError::CapoTooHigh { capo: 9, max: 8 };
        assert_eq!(
            err.to_string(),
            "The capo fret (9) is too high. The maximum is 8."
        );
    }

    #[test]
    fn capo_exceeds_frets() {
        let err = TabError::CapoExceedsFrets {
            capo: 8,
            num_frets: 2,
        };
        assert_eq!(
            err.to_string(),
            "The capo fret (8) cannot exceed the number of frets (2)."
        );
    }

    #[test]
    fn string_number_out_of_range_zero() {
        let err = TabError::StringNumberOutOfRange { value: 0, max: 12 };
        assert_eq!(
            err.to_string(),
            "A guitar cannot have a string number of zero (0). Guitar string numbering commences at one (1)."
        );
    }

    #[test]
    fn string_number_out_of_range_above_max() {
        let err = TabError::StringNumberOutOfRange { value: 13, max: 12 };
        assert_eq!(
            err.to_string(),
            "The string number (13) is too high. The maximum is 12."
        );
    }

    #[test]
    fn open_pitch_out_of_range() {
        let err = TabError::OpenPitchOutOfRange {
            string: 1,
            semitones: 8,
        };
        assert_eq!(
            err.to_string(),
            "Capo offset of 8 semitones on string 1 would push the open pitch out of the supported range."
        );
    }

    #[test]
    fn fret_range_exceeds_pitch_range() {
        let err = TabError::FretRangeExceedsPitchRange {
            open_pitch: "G9".to_owned(),
            playable_frets: 5,
        };
        assert_eq!(
            err.to_string(),
            "Too many frets (5) for string starting at pitch G9. The highest playable pitch is B9."
        );
    }

    #[test]
    fn unplayable_pitches_joins_with_newlines() {
        let err = TabError::UnplayablePitches {
            pitches: vec![
                UnplayablePitch {
                    value: "A1".to_owned(),
                    line: 1,
                },
                UnplayablePitch {
                    value: "B1".to_owned(),
                    line: 4,
                },
            ],
        };
        assert_eq!(
            err.to_string(),
            concat!(
                "Pitch A1 on line 1 cannot be played on any strings of the configured guitar.",
                "\n",
                "Pitch B1 on line 4 cannot be played on any strings of the configured guitar.",
            )
        );
    }

    #[test]
    fn unplayable_pitch_display_reproduces_legacy_message() {
        let pitch = UnplayablePitch {
            value: "A1".to_owned(),
            line: 3,
        };
        assert_eq!(
            pitch.to_string(),
            "Pitch A1 on line 3 cannot be played on any strings of the configured guitar."
        );
    }

    #[test]
    fn no_arrangements_found() {
        let err = TabError::NoArrangementsFound;
        assert_eq!(err.to_string(), "No arrangements could be calculated.");
    }

    #[test]
    fn num_arrangements_out_of_range() {
        let err = TabError::NumArrangementsOutOfRange { value: 21, max: 20 };
        assert_eq!(
            err.to_string(),
            "The number of arrangements (21) must be between 1 and 20."
        );
    }

    #[test]
    fn tuning_name_unknown() {
        let err = TabError::TuningNameUnknown {
            value: "openZ".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "The tuning name (\"openZ\") is not recognized. Use \"standard\" or another supported tuning name."
        );
    }

    #[test]
    fn index_out_of_bounds() {
        let err = TabError::IndexOutOfBounds { index: 99, len: 3 };
        assert_eq!(
            err.to_string(),
            "index 99 is out of bounds for set of length 3"
        );
    }

    #[test]
    fn render_width_too_small() {
        let err = TabError::RenderWidthTooSmall { width: 3, min: 4 };
        assert_eq!(
            err.to_string(),
            "The render width (3) is too small. The minimum is 4."
        );
    }
}
