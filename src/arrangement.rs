use crate::{
    guitar::{generate_pitch_fingerings, Guitar, PitchFingering},
    pitch::Pitch,
};
use anyhow::{anyhow, Result};
use average::Mean;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use pathfinding::prelude::yen;
use std::{collections::HashSet, sync::Arc};

#[derive(Debug)]
pub struct InvalidInput {
    value: String,
    line_number: u16,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Line<T> {
    MeasureBreak,
    Rest,
    Playable(T),
}
use Line::{MeasureBreak, Playable, Rest};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
enum Node {
    Start,
    Rest {
        line_index: u16,
    },
    Note {
        line_index: u16,
        beat_fingering_combo: BeatFingeringCombo,
    },
}

pub type PitchVec<T> = Vec<T>;
pub type BeatVec<T> = Vec<T>;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[allow(dead_code)]
pub struct BeatFingeringCombo {
    fingering_combo: BeatVec<PitchFingering>,
    avg_non_zero_fret: Option<OrderedFloat<f32>>,
    non_zero_fret_span: u8,
}
impl BeatFingeringCombo {
    pub fn new(beat_fingering_candidate: BeatVec<&PitchFingering>) -> Self {
        BeatFingeringCombo {
            fingering_combo: beat_fingering_candidate
                .clone()
                .into_iter()
                .cloned()
                .collect(),
            avg_non_zero_fret: calc_avg_non_zero_fret(&beat_fingering_candidate),
            non_zero_fret_span: calc_fret_span(beat_fingering_candidate).unwrap_or(0),
        }
    }
}
#[cfg(test)]
mod test_create_beat_fingering_combo {
    use super::*;
    use crate::string_number::StringNumber;

    #[test]
    fn simple() {
        let pitch_fingering_1 = PitchFingering {
            pitch: Pitch::A0,
            string_number: StringNumber::new(1).unwrap(),
            fret: 2,
        };

        let BeatFingeringCombo {
            fingering_combo,
            avg_non_zero_fret,
            non_zero_fret_span,
        } = BeatFingeringCombo::new(vec![&pitch_fingering_1]);

        assert_eq!(fingering_combo, vec![pitch_fingering_1]);
        assert_eq!(avg_non_zero_fret, Some(OrderedFloat(2.0)));
        assert_eq!(non_zero_fret_span, 0);
    }
    #[test]
    fn complex() {
        let pitch_fingering_1 = PitchFingering {
            pitch: Pitch::A0,
            string_number: StringNumber::new(1).unwrap(),
            fret: 2,
        };
        let pitch_fingering_2 = PitchFingering {
            pitch: Pitch::B1,
            string_number: StringNumber::new(2).unwrap(),
            fret: 5,
        };
        let pitch_fingering_3 = PitchFingering {
            pitch: Pitch::C2,
            string_number: StringNumber::new(3).unwrap(),
            fret: 0,
        };
        let pitch_fingering_4 = PitchFingering {
            pitch: Pitch::D3,
            string_number: StringNumber::new(4).unwrap(),
            fret: 1,
        };

        let BeatFingeringCombo {
            fingering_combo,
            avg_non_zero_fret,
            non_zero_fret_span,
        } = BeatFingeringCombo::new(vec![
            &pitch_fingering_1,
            &pitch_fingering_2,
            &pitch_fingering_3,
            &pitch_fingering_4,
        ]);

        assert_eq!(
            fingering_combo,
            vec![
                pitch_fingering_1,
                pitch_fingering_2,
                pitch_fingering_3,
                pitch_fingering_4
            ]
        );
        assert_eq!(avg_non_zero_fret, Some(OrderedFloat(8.0 / 3.0)));
        assert_eq!(non_zero_fret_span, 4);
    }
}

fn calc_avg_non_zero_fret(
    beat_fingering_candidate: &[&PitchFingering],
) -> Option<OrderedFloat<f32>> {
    let non_zero_fingerings = beat_fingering_candidate
        .iter()
        .filter(|fingering| fingering.fret != 0)
        .map(|fingering| fingering.fret as f64)
        .collect::<Mean>();

    match non_zero_fingerings.is_empty() {
        true => None,
        false => Some(OrderedFloat(non_zero_fingerings.mean() as f32)),
    }
}
#[cfg(test)]
mod test_calc_avg_non_zero_fret {
    use super::*;
    use crate::string_number::StringNumber;

    #[test]
    fn single_non_zero_fret() {
        let pitch_fingering_1 = PitchFingering {
            pitch: Pitch::A0,
            string_number: StringNumber::new(1).unwrap(),
            fret: 2,
        };

        assert_eq!(
            calc_avg_non_zero_fret(&[&pitch_fingering_1]),
            Some(OrderedFloat(2.0))
        );
    }
    #[test]
    fn single_zero_fret() {
        let pitch_fingering_1 = PitchFingering {
            pitch: Pitch::A0,
            string_number: StringNumber::new(1).unwrap(),
            fret: 0,
        };

        assert_eq!(calc_avg_non_zero_fret(&[&pitch_fingering_1]), None);
    }
    #[test]
    fn multiple_zero_frets() {
        let pitch_fingering_1 = PitchFingering {
            pitch: Pitch::A0,
            string_number: StringNumber::new(1).unwrap(),
            fret: 0,
        };
        let pitch_fingering_2 = PitchFingering {
            pitch: Pitch::B2,
            string_number: StringNumber::new(2).unwrap(),
            fret: 0,
        };

        assert_eq!(
            calc_avg_non_zero_fret(&[&pitch_fingering_1, &pitch_fingering_2]),
            None
        );
    }
    #[test]
    fn multiple_mixed_frets() {
        let pitch_fingering_1 = PitchFingering {
            pitch: Pitch::A0,
            string_number: StringNumber::new(1).unwrap(),
            fret: 2,
        };
        let pitch_fingering_2 = PitchFingering {
            pitch: Pitch::B1,
            string_number: StringNumber::new(2).unwrap(),
            fret: 5,
        };
        let pitch_fingering_3 = PitchFingering {
            pitch: Pitch::C2,
            string_number: StringNumber::new(3).unwrap(),
            fret: 0,
        };
        let pitch_fingering_4 = PitchFingering {
            pitch: Pitch::D3,
            string_number: StringNumber::new(4).unwrap(),
            fret: 1,
        };

        assert_eq!(
            calc_avg_non_zero_fret(&[
                &pitch_fingering_1,
                &pitch_fingering_2,
                &pitch_fingering_3,
                &pitch_fingering_4,
            ]),
            Some(OrderedFloat(8.0 / 3.0))
        );
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Arrangement {
    pub lines: Vec<Line<BeatVec<PitchFingering>>>,
    difficulty: i32,
    max_fret_span: u8,
}
impl Arrangement {
    pub fn max_fret_span(&self) -> u8 {
        self.max_fret_span
    }
}
#[cfg(test)]
mod test_max_fret_span {
    use super::*;

    #[test]
    fn test_max_fret_span() {
        let arrangement = Arrangement {
            lines: vec![],
            difficulty: 4,
            max_fret_span: 5,
        };
        assert_eq!(arrangement.max_fret_span(), 5);
    }
}

use memoize::memoize;
#[memoize(Capacity: 10)]
pub fn create_arrangements(
    guitar: Guitar,
    input_lines: Vec<Line<BeatVec<Pitch>>>,
    num_arrangements: u8,
) -> Result<Vec<Arrangement>, Arc<anyhow::Error>> {
    const MAX_NUM_ARRANGEMENTS: u8 = 20;
    match num_arrangements {
        1..=MAX_NUM_ARRANGEMENTS => (),
        0 => return Err(Arc::new(anyhow!("No arrangements were requested."))),
        _ => {
            return Err(Arc::new(anyhow!(
                "Too many arrangements to calculate. The maximum is {}.",
                MAX_NUM_ARRANGEMENTS
            )))
        }
    };

    let input_playable_lines = input_lines
        .iter()
        .filter(|line| matches!(line, Line::Playable(_)))
        .collect_vec();
    if input_playable_lines.is_empty() {
        let empty_compositions = vec![
            Arrangement {
                lines: vec![],
                difficulty: 0,
                max_fret_span: 0,
            };
            num_arrangements as usize
        ];
        return Ok(empty_compositions);
    }

    let first_playable_index = input_lines
        .iter()
        .position(|line| matches!(line, Line::Playable(_)))
        .unwrap_or(0);

    let lines = input_lines
        .iter()
        .skip(first_playable_index)
        .cloned()
        .collect_vec();

    let pitch_fingering_candidates: Vec<Line<BeatVec<PitchVec<PitchFingering>>>> =
        validate_fingerings(&guitar, &lines)?;

    let measure_break_indices: Vec<usize> = pitch_fingering_candidates
        .iter()
        .enumerate()
        .filter(|(.., line_candidate)| matches!(line_candidate, MeasureBreak))
        .map(|(line_index, ..)| line_index)
        .collect_vec();

    let path_node_groups: Vec<BeatVec<Node>> = pitch_fingering_candidates
        .iter()
        .filter(|line_candidate| !matches!(line_candidate, MeasureBreak))
        .enumerate()
        .map(|(line_index, line_candidate)| match line_candidate {
            MeasureBreak => unreachable!("Measure breaks should have been filtered out."),
            Rest => vec![Node::Rest {
                line_index: line_index as u16,
            }],
            Playable(beat_fingerings_per_pitch) => {
                generate_fingering_combos(beat_fingerings_per_pitch)
                    .iter()
                    .map(|pitch_fingering_group| Node::Note {
                        line_index: line_index as u16,
                        beat_fingering_combo: BeatFingeringCombo::new(
                            pitch_fingering_group.to_vec(),
                        ),
                    })
                    .collect()
            }
        })
        .collect();

    let num_path_node_groups = path_node_groups.len();

    let path_nodes: Vec<Node> = path_node_groups.into_iter().flatten().collect_vec();

    let path_results: Vec<(Vec<Node>, i32)> = yen(
        &Node::Start,
        |current_node| calc_next_nodes(current_node, path_nodes.clone()),
        |current_node| match current_node {
            Node::Start => false,
            Node::Rest { line_index } | Node::Note { line_index, .. } => {
                // Pathfinding goal is reached when the node is in the last node group
                *line_index == (num_path_node_groups - 1) as u16
            }
        },
        num_arrangements as usize,
    );
    // dbg!(&path_results);

    if path_results.is_empty() {
        return Err(Arc::new(anyhow!("No arrangements could be calculated.")));
    }

    let arrangements = path_results
        .into_iter()
        .map(|path_result| {
            process_path(path_result.0, path_result.1, measure_break_indices.clone())
        })
        .collect_vec();

    // const WARNING_FRET_SPAN: u8 = 4;

    Ok(arrangements)
}
#[cfg(test)]
mod test_create_arrangements {
    use super::*;
    use crate::string_number::StringNumber;

    #[test]
    fn single_line_single_pitch() {
        let input_pitches: Vec<Line<BeatVec<Pitch>>> = vec![Line::Playable(vec![Pitch::E4])];
        let expected_arrangements: Vec<Arrangement> = vec![Arrangement {
            lines: vec![Line::Playable(vec![PitchFingering {
                pitch: Pitch::E4,
                string_number: StringNumber::new(1).unwrap(),
                fret: 0,
            }])],
            difficulty: 0,
            max_fret_span: 0,
        }];

        let arrangements = create_arrangements(Guitar::default(), input_pitches, 1).unwrap();

        assert_eq!(arrangements, expected_arrangements);
    }
    #[test]
    fn single_line_single_pitch_multiple_arrangements() {
        let input_pitches: Vec<Line<BeatVec<Pitch>>> = vec![Line::Playable(vec![Pitch::E4])];
        let expected_arrangements: Vec<Arrangement> = vec![
            Arrangement {
                lines: vec![Line::Playable(vec![PitchFingering {
                    pitch: Pitch::E4,
                    string_number: StringNumber::new(1).unwrap(),
                    fret: 0,
                }])],
                difficulty: 0,
                max_fret_span: 0,
            },
            Arrangement {
                lines: vec![Line::Playable(vec![PitchFingering {
                    pitch: Pitch::E4,
                    string_number: StringNumber::new(2).unwrap(),
                    fret: 5,
                }])],
                difficulty: 5,
                max_fret_span: 0,
            },
            Arrangement {
                lines: vec![Line::Playable(vec![PitchFingering {
                    pitch: Pitch::E4,
                    string_number: StringNumber::new(3).unwrap(),
                    fret: 9,
                }])],
                difficulty: 9,
                max_fret_span: 0,
            },
            Arrangement {
                lines: vec![Line::Playable(vec![PitchFingering {
                    pitch: Pitch::E4,
                    string_number: StringNumber::new(4).unwrap(),
                    fret: 14,
                }])],
                difficulty: 14,
                max_fret_span: 0,
            },
        ];

        let arrangements = create_arrangements(Guitar::default(), input_pitches, 10).unwrap();

        assert_eq!(arrangements, expected_arrangements);
    }
    #[test]
    fn single_lines_all_variants() {
        let input_pitches: Vec<Line<BeatVec<Pitch>>> = vec![
            Line::Playable(vec![Pitch::E4]),
            Line::Rest,
            Line::MeasureBreak,
        ];
        let expected_arrangements: Vec<Arrangement> = vec![Arrangement {
            lines: vec![
                Line::Playable(vec![PitchFingering {
                    pitch: Pitch::E4,
                    string_number: StringNumber::new(1).unwrap(),
                    fret: 0,
                }]),
                Line::Rest,
                Line::MeasureBreak,
            ],
            difficulty: 0,
            max_fret_span: 0,
        }];

        let arrangements = create_arrangements(Guitar::default(), input_pitches, 1).unwrap();

        assert_eq!(arrangements, expected_arrangements);
    }
    #[test]
    fn empty_input() {
        let input_pitches: Vec<Line<BeatVec<Pitch>>> = vec![];

        let arrangements = create_arrangements(Guitar::default(), input_pitches, 2).unwrap();

        let expected_arrangements: Vec<Arrangement> = vec![
            Arrangement {
                lines: vec![],
                difficulty: 0,
                max_fret_span: 0,
            };
            2
        ];

        assert_eq!(arrangements, expected_arrangements);
    }
    #[test]
    fn empty_start_lines_input() {
        let input_pitches: Vec<Line<BeatVec<Pitch>>> = vec![
            Line::Rest,
            Line::MeasureBreak,
            Line::Rest,
            Line::Playable(vec![Pitch::E4]),
            Line::Rest,
        ];

        let arrangements = create_arrangements(Guitar::default(), input_pitches, 1).unwrap();

        let expected_arrangements: Vec<Arrangement> = vec![Arrangement {
            lines: vec![
                Line::Playable(vec![PitchFingering {
                    pitch: Pitch::E4,
                    string_number: StringNumber::new(1).unwrap(),
                    fret: 0,
                }]),
                Line::Rest,
            ],
            difficulty: 0,
            max_fret_span: 0,
        }];

        assert_eq!(arrangements, expected_arrangements);
    }
    #[test]
    fn zero_arrangements_requested() {
        let input_pitches: Vec<Line<BeatVec<Pitch>>> = vec![Line::Playable(vec![Pitch::E4])];

        let error = create_arrangements(Guitar::default(), input_pitches, 0).unwrap_err();
        let error_msg = format!("{error}");
        assert_eq!(error_msg, "No arrangements were requested.");
    }
    #[test]
    fn too_many_arrangements_requested() {
        let input_pitches: Vec<Line<BeatVec<Pitch>>> = vec![Line::Playable(vec![Pitch::E4])];

        let error = create_arrangements(Guitar::default(), input_pitches, 22).unwrap_err();
        let error_msg = format!("{error}");
        assert_eq!(
            error_msg,
            "Too many arrangements to calculate. The maximum is 20."
        );
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
/// Returns a `Result` containing either a
/// `Vec<Vec<Vec<Fingering>>>` if the input pitches are valid, or an `Err` containing an error
/// message if there are invalid pitches.
fn validate_fingerings(
    guitar: &Guitar,
    input_pitches: &[Line<BeatVec<Pitch>>],
) -> Result<Vec<Line<BeatVec<PitchVec<PitchFingering>>>>> {
    let mut impossible_pitches: Vec<InvalidInput> = vec![];
    let fingerings: Vec<Line<BeatVec<PitchVec<PitchFingering>>>> = input_pitches
        .iter()
        .enumerate()
        .map(|(beat_index, beat_input)| match beat_input {
            MeasureBreak => MeasureBreak,
            Rest => Rest,
            Playable(beat_pitches) => Playable(
                beat_pitches
                    .iter()
                    .map(|beat_pitch| {
                        let pitch_fingerings: PitchVec<PitchFingering> =
                            generate_pitch_fingerings(&guitar.string_ranges, beat_pitch);
                        if pitch_fingerings.is_empty() {
                            impossible_pitches.push(InvalidInput {
                                value: format!("{:?}", beat_pitch),
                                line_number: (beat_index as u16) + 1,
                            })
                        }
                        pitch_fingerings
                    })
                    .collect(),
            ),
        })
        .collect();

    if !impossible_pitches.is_empty() {
        let error_msg = impossible_pitches
            .iter()
            .map(|invalid_input| {
                format!(
                    "Pitch {} on line {} cannot be played on any strings of the configured guitar.",
                    invalid_input.value, invalid_input.line_number
                )
            })
            .collect::<Vec<String>>()
            .join("\n");

        return Err(anyhow!(error_msg));
    }

    Ok(fingerings)
}
#[cfg(test)]
mod test_validate_fingerings {
    use super::*;

    #[test]
    fn valid_simple() {
        let guitar = Guitar::default();
        let input_pitches = vec![Playable(vec![Pitch::G3])];
        let expected_fingerings = vec![Playable(vec![generate_pitch_fingerings(
            &guitar.string_ranges,
            &Pitch::G3,
        )])];

        assert_eq!(
            validate_fingerings(&guitar, &input_pitches).unwrap(),
            expected_fingerings
        );
    }
    #[test]
    fn valid_complex() {
        let guitar = Guitar::default();
        let input_pitches = vec![
            Playable(vec![Pitch::G3]),
            MeasureBreak,
            Playable(vec![Pitch::B3]),
            Rest,
            Playable(vec![Pitch::D4, Pitch::G4]),
        ];
        let expected_fingerings = vec![
            Playable(vec![generate_pitch_fingerings(
                &guitar.string_ranges,
                &Pitch::G3,
            )]),
            MeasureBreak,
            Playable(vec![generate_pitch_fingerings(
                &guitar.string_ranges,
                &Pitch::B3,
            )]),
            Rest,
            Playable(vec![
                generate_pitch_fingerings(&guitar.string_ranges, &Pitch::D4),
                generate_pitch_fingerings(&guitar.string_ranges, &Pitch::G4),
            ]),
        ];

        assert_eq!(
            validate_fingerings(&guitar, &input_pitches).unwrap(),
            expected_fingerings
        );
    }
    #[test]
    fn invalid_simple() {
        let guitar = Guitar::default();
        let input_pitches = vec![Playable(vec![Pitch::B9])];

        let error = validate_fingerings(&guitar, &input_pitches).unwrap_err();
        let error_msg = format!("{error}");
        let expected_error_msg =
            "Pitch B9 on line 1 cannot be played on any strings of the configured guitar.";
        assert_eq!(error_msg, expected_error_msg);
    }
    #[test]
    fn invalid_complex() {
        let guitar = Guitar::default();
        let input_pitches = vec![
            Playable(vec![Pitch::A1]),
            Playable(vec![Pitch::G3]),
            Playable(vec![Pitch::B3]),
            Playable(vec![Pitch::A1, Pitch::B1]),
            Playable(vec![Pitch::G3, Pitch::D2]),
            Playable(vec![Pitch::D4, Pitch::G4]),
        ];

        let error = validate_fingerings(&guitar, &input_pitches).unwrap_err();
        let error_msg = format!("{error}");
        let expected_error_msg =
            "Pitch A1 on line 1 cannot be played on any strings of the configured guitar.\n\
            Pitch A1 on line 4 cannot be played on any strings of the configured guitar.\n\
            Pitch B1 on line 4 cannot be played on any strings of the configured guitar.\n\
            Pitch D2 on line 5 cannot be played on any strings of the configured guitar.";
        assert_eq!(error_msg, expected_error_msg);
    }
}

/// Generates all playable combinations of fingerings for all the pitches in a beat.
fn generate_fingering_combos(
    beat_fingerings_per_pitch: &[Vec<PitchFingering>],
) -> Vec<BeatVec<&PitchFingering>> {
    if beat_fingerings_per_pitch.is_empty() {
        unreachable!("Beat pitch fingerings should not be empty.")
    }

    beat_fingerings_per_pitch
        .iter()
        .multi_cartesian_product()
        .filter(no_duplicate_strings)
        .collect_vec()
}
#[cfg(test)]
mod test_generate_fingering_combos {
    use super::*;
    use crate::string_number::StringNumber;

    #[test]
    fn simple() {
        let pitch_fingering = PitchFingering {
            pitch: Pitch::B6,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };

        let beat_fingerings_per_pitch = vec![vec![pitch_fingering]];
        let expected_fingering_combos = vec![vec![&pitch_fingering]];

        assert_eq!(
            generate_fingering_combos(&beat_fingerings_per_pitch),
            expected_fingering_combos
        );
    }
    #[test]
    fn complex() {
        let pitch_fingering_a_string_2 = PitchFingering {
            pitch: Pitch::B6,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };
        let pitch_fingering_a_string_3 = PitchFingering {
            pitch: Pitch::B6,
            string_number: StringNumber::new(3).unwrap(),
            fret: 8,
        };
        let pitch_fingering_b_string_2 = PitchFingering {
            pitch: Pitch::C7,
            string_number: StringNumber::new(2).unwrap(),
            fret: 4,
        };
        let pitch_fingering_b_string_3 = PitchFingering {
            pitch: Pitch::C7,
            string_number: StringNumber::new(3).unwrap(),
            fret: 9,
        };
        let pitch_fingering_b_string_4 = PitchFingering {
            pitch: Pitch::C7,
            string_number: StringNumber::new(4).unwrap(),
            fret: 14,
        };

        let beat_fingerings_per_pitch = vec![
            vec![pitch_fingering_a_string_2, pitch_fingering_a_string_3],
            vec![
                pitch_fingering_b_string_2,
                pitch_fingering_b_string_3,
                pitch_fingering_b_string_4,
            ],
        ];
        let expected_fingering_combos = vec![
            vec![&pitch_fingering_a_string_2, &pitch_fingering_b_string_3],
            vec![&pitch_fingering_a_string_2, &pitch_fingering_b_string_4],
            vec![&pitch_fingering_a_string_3, &pitch_fingering_b_string_2],
            vec![&pitch_fingering_a_string_3, &pitch_fingering_b_string_4],
        ];

        assert_eq!(
            generate_fingering_combos(&beat_fingerings_per_pitch),
            expected_fingering_combos
        );
    }

    #[test]
    #[should_panic]
    fn empty_input() {
        generate_fingering_combos(&[]);
    }
}

/// Checks if there are any duplicate strings in a vector of `Fingering`
/// objects to ensure that all pitches can be played.
fn no_duplicate_strings(beat_fingering_option: &Vec<&PitchFingering>) -> bool {
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
    use crate::string_number::StringNumber;

    #[test]
    fn valid_simple() {
        let fingering_1 = PitchFingering {
            pitch: Pitch::B6,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };
        let beat_fingering_option: &Vec<&PitchFingering> = &vec![&fingering_1];

        assert!(no_duplicate_strings(beat_fingering_option));
    }
    #[test]
    fn valid_complex() {
        let fingering_1 = PitchFingering {
            pitch: Pitch::CSharpDFlat2,
            string_number: StringNumber::new(1).unwrap(),
            fret: 1,
        };
        let fingering_2 = PitchFingering {
            pitch: Pitch::F4,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };
        let fingering_3 = PitchFingering {
            pitch: Pitch::A5,
            string_number: StringNumber::new(4).unwrap(),
            fret: 4,
        };
        let fingering_4 = PitchFingering {
            pitch: Pitch::DSharpEFlat6,
            string_number: StringNumber::new(11).unwrap(),
            fret: 0,
        };
        let beat_fingering_option: &Vec<&PitchFingering> =
            &vec![&fingering_1, &fingering_2, &fingering_3, &fingering_4];

        assert!(no_duplicate_strings(beat_fingering_option));
    }
    #[test]
    fn invalid_simple() {
        let fingering_1 = PitchFingering {
            pitch: Pitch::CSharpDFlat2,
            string_number: StringNumber::new(4).unwrap(),
            fret: 1,
        };
        let fingering_2 = PitchFingering {
            pitch: Pitch::F4,
            string_number: StringNumber::new(4).unwrap(),
            fret: 3,
        };
        let beat_fingering_option: &Vec<&PitchFingering> = &vec![&fingering_1, &fingering_2];

        assert!(!no_duplicate_strings(beat_fingering_option));
    }
    #[test]
    fn invalid_complex() {
        let fingering_1 = PitchFingering {
            pitch: Pitch::CSharpDFlat2,
            string_number: StringNumber::new(1).unwrap(),
            fret: 1,
        };
        let fingering_2 = PitchFingering {
            pitch: Pitch::F4,
            string_number: StringNumber::new(3).unwrap(),
            fret: 3,
        };
        let fingering_3 = PitchFingering {
            pitch: Pitch::A5,
            string_number: StringNumber::new(6).unwrap(),
            fret: 4,
        };
        let fingering_4 = PitchFingering {
            pitch: Pitch::DSharpEFlat6,
            string_number: StringNumber::new(3).unwrap(),
            fret: 0,
        };
        let beat_fingering_option: &Vec<&PitchFingering> =
            &vec![&fingering_1, &fingering_2, &fingering_3, &fingering_4];

        assert!(!no_duplicate_strings(beat_fingering_option));
    }
    #[test]
    fn empty_input() {
        assert!(no_duplicate_strings(&vec![]));
    }
}

/// Calculates the difference between the maximum and minimum non-zero
/// fret numbers in a given vector of fingerings.
fn calc_fret_span(beat_fingering_candidate: Vec<&PitchFingering>) -> Option<u8> {
    let beat_fingering_option_fret_numbers = beat_fingering_candidate
        .iter()
        .filter(|fingering| fingering.fret != 0)
        .map(|fingering| fingering.fret);

    let min_non_zero_fret = match beat_fingering_option_fret_numbers.clone().min() {
        None => return None,
        Some(fret_num) => fret_num,
    };
    let max_non_zero_fret = match beat_fingering_option_fret_numbers.clone().max() {
        None => unreachable!("A maximum should exist if a minimum exists."),
        Some(fret_num) => fret_num,
    };

    Some(max_non_zero_fret - min_non_zero_fret)
}
#[cfg(test)]
mod test_calc_fret_span {
    use super::*;
    use crate::string_number::StringNumber;

    #[test]
    fn simple() {
        let fingering_1 = PitchFingering {
            pitch: Pitch::B6,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };

        assert_eq!(calc_fret_span(vec![&fingering_1]).unwrap(), 0);
    }
    #[test]
    fn complex() {
        let fingering_1 = PitchFingering {
            pitch: Pitch::CSharpDFlat2,
            string_number: StringNumber::new(1).unwrap(),
            fret: 1,
        };
        let fingering_2 = PitchFingering {
            pitch: Pitch::F4,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };
        let fingering_3 = PitchFingering {
            pitch: Pitch::A5,
            string_number: StringNumber::new(4).unwrap(),
            fret: 4,
        };
        let fingering_4 = PitchFingering {
            pitch: Pitch::DSharpEFlat6,
            string_number: StringNumber::new(11).unwrap(),
            fret: 0,
        };
        let beat_fingering_option: Vec<&PitchFingering> =
            vec![&fingering_1, &fingering_2, &fingering_3, &fingering_4];

        assert_eq!(calc_fret_span(beat_fingering_option).unwrap(), 3);
    }
    #[test]
    fn empty_input() {
        assert!(calc_fret_span(vec![]).is_none());
    }
}

/// Calculates the next nodes and their costs based on the current node and a
/// list of all path nodes.
///
/// Returns a vector of tuples, where each tuple contains a `Node` the `i32`
/// cost of moving to that node.
fn calc_next_nodes(current_node: &Node, path_nodes: Vec<Node>) -> Vec<(Node, i32)> {
    let next_node_index = match current_node {
        Node::Start => 0,
        Node::Rest { line_index } | Node::Note { line_index, .. } => line_index + 1,
    };

    let next_nodes: Vec<(Node, i32)> = path_nodes
        .iter()
        .filter(|&node| {
            next_node_index
                == match node {
                    Node::Start => unreachable!("Start should never be a future node."),
                    Node::Rest { line_index } | Node::Note { line_index, .. } => *line_index,
                }
        })
        .map(|next_node| {
            (
                next_node.clone(),
                calculate_node_difficulty(current_node, next_node),
            )
        })
        .collect_vec();

    next_nodes
}
#[cfg(test)]
mod test_calc_next_nodes {
    use super::*;

    fn create_test_path_nodes() -> Vec<Node> {
        vec![
            Node::Note {
                line_index: 0,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(0.1)),
                    non_zero_fret_span: 0,
                },
            },
            Node::Note {
                line_index: 0,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(0.2)),
                    non_zero_fret_span: 0,
                },
            },
            Node::Note {
                line_index: 1,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(1.1)),
                    non_zero_fret_span: 1,
                },
            },
            Node::Rest { line_index: 2 },
            Node::Rest { line_index: 3 },
            Node::Note {
                line_index: 4,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(4.1)),
                    non_zero_fret_span: 4,
                },
            },
            Node::Note {
                line_index: 4,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(4.1)),
                    non_zero_fret_span: 4,
                },
            },
        ]
    }

    #[test]
    fn from_start_to_note() {
        let current_node = Node::Start;

        let expected_nodes_and_costs = [
            Node::Note {
                line_index: 0,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(0.1)),
                    non_zero_fret_span: 0,
                },
            },
            Node::Note {
                line_index: 0,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(0.2)),
                    non_zero_fret_span: 0,
                },
            },
        ]
        .iter()
        .map(|node| (node.clone(), calculate_node_difficulty(&current_node, node)))
        .collect_vec();

        assert_eq!(
            calc_next_nodes(&current_node, create_test_path_nodes()),
            expected_nodes_and_costs
        );
    }
    #[test]
    fn from_note_to_note() {
        let current_node = Node::Note {
            line_index: 0,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(0.1)),
                non_zero_fret_span: 0,
            },
        };

        let expected_nodes_and_costs = [Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(1.1)),
                non_zero_fret_span: 1,
            },
        }]
        .iter()
        .map(|node| (node.clone(), calculate_node_difficulty(&current_node, node)))
        .collect_vec();

        assert_eq!(
            calc_next_nodes(&current_node, create_test_path_nodes()),
            expected_nodes_and_costs
        );
    }
    #[test]
    fn from_note_to_rest() {
        let current_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(1.1)),
                non_zero_fret_span: 1,
            },
        };

        let expected_nodes_and_costs = [Node::Rest { line_index: 2 }]
            .iter()
            .map(|node| (node.clone(), calculate_node_difficulty(&current_node, node)))
            .collect_vec();

        assert_eq!(
            calc_next_nodes(&current_node, create_test_path_nodes()),
            expected_nodes_and_costs
        );
    }
    #[test]
    fn from_rest_to_rest() {
        let current_node = Node::Rest { line_index: 2 };

        let expected_nodes_and_costs = [Node::Rest { line_index: 3 }]
            .iter()
            .map(|node| (node.clone(), calculate_node_difficulty(&current_node, node)))
            .collect_vec();

        assert_eq!(
            calc_next_nodes(&current_node, create_test_path_nodes()),
            expected_nodes_and_costs
        );
    }
    #[test]
    fn from_rest_to_note() {
        let current_node = Node::Rest { line_index: 3 };

        let expected_nodes_and_costs = [
            Node::Note {
                line_index: 4,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(4.1)),
                    non_zero_fret_span: 4,
                },
            },
            Node::Note {
                line_index: 4,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(4.1)),
                    non_zero_fret_span: 4,
                },
            },
        ]
        .iter()
        .map(|node| (node.clone(), calculate_node_difficulty(&current_node, node)))
        .collect_vec();

        assert_eq!(
            calc_next_nodes(&current_node, create_test_path_nodes()),
            expected_nodes_and_costs
        );
    }

    #[test]
    #[should_panic]
    fn to_start() {
        calc_next_nodes(
            &Node::Rest { line_index: 3 },
            vec![Node::Rest { line_index: 4 }, Node::Start],
        );
    }
}

/// Calculates the cost of transitioning from one node to another based on the
/// average fret difference and fret span.
fn calculate_node_difficulty(current_node: &Node, next_node: &Node) -> i32 {
    let current_avg_fret = match current_node {
        Node::Note {
            beat_fingering_combo,
            ..
        } => beat_fingering_combo.avg_non_zero_fret,
        _ => None,
    };

    let (next_avg_fret, next_fret_span) = match next_node {
        Node::Start => unreachable!("Start should never be a future node."),
        Node::Rest { .. } => (None, 0.0),
        Node::Note {
            beat_fingering_combo,
            ..
        } => (
            beat_fingering_combo.avg_non_zero_fret,
            beat_fingering_combo.non_zero_fret_span as f32,
        ),
    };

    let mut avg_fret_difference = 0.0;
    if let (Some(current_avg_fret_num), Some(next_avg_fret_num)) = (current_avg_fret, next_avg_fret)
    {
        avg_fret_difference = (next_avg_fret_num - current_avg_fret_num).abs();
    }

    ((avg_fret_difference * 100.0)
        + (next_fret_span * 10.0)
        + (next_avg_fret.unwrap_or(OrderedFloat(0.0))).into_inner()) as i32
}
#[cfg(test)]
mod test_calculate_node_difficulty {
    use super::*;

    #[test]
    fn simple_no_diff() {
        let current_node = Node::Note {
            line_index: 0,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.5)),
                non_zero_fret_span: 0,
            },
        };
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.5)),
                non_zero_fret_span: 0,
            },
        };

        assert_eq!(calculate_node_difficulty(&current_node, &next_node), 3);
    }
    #[test]
    fn simple_from_start() {
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.5)),
                non_zero_fret_span: 0,
            },
        };

        assert_eq!(calculate_node_difficulty(&Node::Start, &next_node), 3);
    }
    #[test]
    fn simple_from_rest() {
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.5)),
                non_zero_fret_span: 0,
            },
        };

        assert_eq!(
            calculate_node_difficulty(&Node::Rest { line_index: 0 }, &next_node),
            3
        );
    }
    #[test]
    fn simple_to_rest() {
        let current_node = Node::Note {
            line_index: 0,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.5)),
                non_zero_fret_span: 0,
            },
        };

        assert_eq!(
            calculate_node_difficulty(&current_node, &Node::Rest { line_index: 1 }),
            0
        );
    }
    #[test]
    fn simple_avg_fret_diff() {
        let current_node = Node::Note {
            line_index: 0,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.0)),
                non_zero_fret_span: 0,
            },
        };
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(1.6)),
                non_zero_fret_span: 0,
            },
        };

        assert_eq!(calculate_node_difficulty(&current_node, &next_node), 141);
    }
    #[test]
    fn simple_fret_span() {
        let current_node = Node::Note {
            line_index: 0,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(4.133333)),
                non_zero_fret_span: 0,
            },
        };
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(4.133333)),
                non_zero_fret_span: 3,
            },
        };

        assert_eq!(calculate_node_difficulty(&current_node, &next_node), 34);
    }
    #[test]
    fn compound() {
        let current_node = Node::Note {
            line_index: 0,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(5.0)),
                non_zero_fret_span: 0,
            },
        };
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(2.0)),
                non_zero_fret_span: 5,
            },
        };

        assert_eq!(calculate_node_difficulty(&current_node, &next_node), 352);
    }
    #[test]
    fn complex() {
        let current_node = Node::Note {
            line_index: 0,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(7.3333333)),
                non_zero_fret_span: 0,
            },
        };
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.6666666)),
                non_zero_fret_span: 4,
            },
        };

        assert_eq!(calculate_node_difficulty(&current_node, &next_node), 410);
    }
}

fn process_path(
    path_nodes: Vec<Node>,
    path_difficulty: i32,
    measure_break_indices: Vec<usize>,
) -> Arrangement {
    let mut lines: Vec<Line<BeatVec<PitchFingering>>> = path_nodes
        .iter()
        .filter(|node| node != &&Node::Start)
        .map(|node| match node {
            Node::Start => unreachable!("Start node should have been filtered out."),
            Node::Rest { .. } => Line::Rest,
            Node::Note {
                beat_fingering_combo,
                ..
            } => Line::Playable(beat_fingering_combo.fingering_combo.clone()),
        })
        .collect_vec();
    // Add measure breaks back in
    for measure_break_index in measure_break_indices.into_iter().sorted() {
        lines.insert(measure_break_index, Line::MeasureBreak);
    }

    let max_fret_span: u8 = path_nodes
        .iter()
        .filter(|node| node != &&Node::Start)
        .filter_map(|node| match node {
            Node::Start => unreachable!("Start node should have been filtered out."),
            Node::Rest { .. } => None,
            Node::Note {
                beat_fingering_combo,
                ..
            } => Some(beat_fingering_combo.non_zero_fret_span),
        })
        .max()
        .unwrap_or(0);

    Arrangement {
        lines,
        difficulty: path_difficulty,
        max_fret_span,
    }
}
#[cfg(test)]
mod test_process_path {
    use super::*;
    use crate::string_number::StringNumber;

    #[test]
    fn simple() {
        let placeholder_beat_fingering_combo = BeatFingeringCombo {
            fingering_combo: vec![PitchFingering {
                pitch: Pitch::C4,
                string_number: StringNumber::new(1).unwrap(),
                fret: 3,
            }],
            avg_non_zero_fret: Some(OrderedFloat(3.0)),
            non_zero_fret_span: 0,
        };

        let path_nodes = vec![
            Node::Start,
            Node::Note {
                line_index: 0,
                beat_fingering_combo: placeholder_beat_fingering_combo.clone(),
            },
        ];

        let arrangement = process_path(path_nodes, 123, vec![]);

        let expected_arrangement = Arrangement {
            lines: vec![Playable(placeholder_beat_fingering_combo.fingering_combo)],
            difficulty: 123,
            max_fret_span: 0,
        };

        assert_eq!(arrangement, expected_arrangement);
    }
    #[test]
    fn complex() {
        let placeholder_beat_fingering_combo = BeatFingeringCombo {
            fingering_combo: vec![PitchFingering {
                pitch: Pitch::C4,
                string_number: StringNumber::new(1).unwrap(),
                fret: 3,
            }],
            avg_non_zero_fret: Some(OrderedFloat(3.0)),
            non_zero_fret_span: 4,
        };

        let path_nodes = vec![
            Node::Start,
            Node::Note {
                line_index: 0,
                beat_fingering_combo: placeholder_beat_fingering_combo.clone(),
            },
            Node::Note {
                line_index: 1,
                beat_fingering_combo: placeholder_beat_fingering_combo.clone(),
            },
            Node::Rest { line_index: 2 },
            Node::Note {
                line_index: 3,
                beat_fingering_combo: placeholder_beat_fingering_combo.clone(),
            },
            Node::Note {
                line_index: 4,
                beat_fingering_combo: placeholder_beat_fingering_combo.clone(),
            },
        ];

        let arrangement = process_path(path_nodes, 321, vec![0, 2, 5, 7]);

        let expected_arrangement = Arrangement {
            lines: vec![
                MeasureBreak,
                Playable(placeholder_beat_fingering_combo.clone().fingering_combo),
                MeasureBreak,
                Playable(placeholder_beat_fingering_combo.clone().fingering_combo),
                Rest,
                MeasureBreak,
                Playable(placeholder_beat_fingering_combo.clone().fingering_combo),
                MeasureBreak,
                Playable(placeholder_beat_fingering_combo.fingering_combo),
            ],
            difficulty: 321,
            max_fret_span: 4,
        };

        assert_eq!(arrangement, expected_arrangement);
    }
}
