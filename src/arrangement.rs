use crate::{
    guitar::{generate_pitch_fingerings, Guitar, PitchFingering},
    pitch::Pitch,
};
use anyhow::{anyhow, Result};
use average::Mean;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use pathfinding::prelude::yen;
use std::collections::HashSet;

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
    non_zero_avg_fret: Option<OrderedFloat<f32>>,
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
            non_zero_avg_fret: calc_non_zero_avg_fret(&beat_fingering_candidate),
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
            non_zero_avg_fret,
            non_zero_fret_span,
        } = BeatFingeringCombo::new(vec![&pitch_fingering_1]);

        assert_eq!(fingering_combo, vec![pitch_fingering_1]);
        assert_eq!(non_zero_avg_fret, Some(OrderedFloat(2.0)));
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
            non_zero_avg_fret,
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
        assert_eq!(non_zero_avg_fret, Some(OrderedFloat(8.0 / 3.0)));
        assert_eq!(non_zero_fret_span, 4);
    }
}

fn calc_non_zero_avg_fret(
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
mod test_calc_non_zero_avg_fret {
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
            calc_non_zero_avg_fret(&[&pitch_fingering_1]),
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

        assert_eq!(calc_non_zero_avg_fret(&[&pitch_fingering_1]), None);
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
            calc_non_zero_avg_fret(&[&pitch_fingering_1, &pitch_fingering_2]),
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
            calc_non_zero_avg_fret(&[
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
    lines: Vec<Line<BeatVec<PitchFingering>>>,
    difficulty: i32,
    max_fret_span: u8,
}

#[allow(unused_variables)]
pub fn render_tab(
    arrangement: Arrangement,
    guitar: Guitar,
    width: u16,
    playback_beat_num: Option<u16>,
) -> String {
    let num_strings = guitar.string_ranges.len();
    let columns = arrangement
        .lines
        .iter()
        .map(|line| render_line(line, num_strings))
        .collect_vec();

    let rows = transpose(columns);
    dbg!(rows);

    "Heyo".to_string()
}

/// Renders Line as a vector of strings representing the fret positions on a guitar.
fn render_line(line: &Line<BeatVec<PitchFingering>>, num_strings: usize) -> Vec<String> {
    let pitch_fingerings = match line {
        Line::MeasureBreak => return vec!["|".to_owned(); num_strings],
        Line::Rest => return vec!["-".to_owned(); num_strings],
        Line::Playable(pitch_fingerings) => pitch_fingerings.iter().sorted().collect_vec(),
    };
    let fret_width_max = calc_fret_width_max(&pitch_fingerings);

    // Instantiate vec with rest dashes for all strings with the max fret width
    let mut playable_render = vec!["-".repeat(fret_width_max); num_strings];

    // Add the rendered frets for the strings that are played
    for fingering in pitch_fingerings {
        playable_render[fingering.string_number.get() as usize - 1] =
            render_fret(fingering.fret, fret_width_max)
    }

    playable_render
}
#[cfg(test)]
mod test_render_line {
    use super::*;
    use crate::string_number::StringNumber;

    const NUM_STRINGS: usize = 6;

    #[test]
    fn measure_break() {
        assert_eq!(
            render_line(&Line::MeasureBreak, NUM_STRINGS),
            vec!["|".to_owned(); NUM_STRINGS]
        );
    }
    #[test]
    fn rest() {
        assert_eq!(
            render_line(&Line::Rest, NUM_STRINGS),
            vec!["-".to_owned(); NUM_STRINGS]
        );
    }
    #[test]
    fn playable_basic() {
        let pitch_fingerings = vec![
            PitchFingering {
                string_number: StringNumber::new(2).unwrap(),
                fret: 2,
                pitch: Pitch::G4,
            },
            PitchFingering {
                string_number: StringNumber::new(5).unwrap(),
                fret: 13,
                pitch: Pitch::G4,
            },
        ];
        let expected_line_render = vec!["--", "-2", "--", "--", "13", "--"];

        assert_eq!(
            render_line(&Line::Playable(pitch_fingerings), 6),
            expected_line_render
        );
    }
    #[test]
    fn playable_complex() {
        let pitch_fingerings = vec![
            PitchFingering {
                string_number: StringNumber::new(1).unwrap(),
                fret: 9,
                pitch: Pitch::G4,
            },
            PitchFingering {
                string_number: StringNumber::new(2).unwrap(),
                fret: 0,
                pitch: Pitch::G4,
            },
            PitchFingering {
                string_number: StringNumber::new(4).unwrap(),
                fret: 8,
                pitch: Pitch::G4,
            },
            PitchFingering {
                string_number: StringNumber::new(5).unwrap(),
                fret: 10,
                pitch: Pitch::G4,
            },
            PitchFingering {
                string_number: StringNumber::new(6).unwrap(),
                fret: 0,
                pitch: Pitch::G4,
            },
            PitchFingering {
                string_number: StringNumber::new(7).unwrap(),
                fret: 11,
                pitch: Pitch::G4,
            },
            PitchFingering {
                string_number: StringNumber::new(8).unwrap(),
                fret: 12,
                pitch: Pitch::G4,
            },
        ];
        let expected_line_render = vec!["-9", "-0", "--", "-8", "10", "-0", "11", "12"];

        assert_eq!(
            render_line(&Line::Playable(pitch_fingerings), 8),
            expected_line_render
        );
    }
    #[test]
    #[should_panic]
    fn playable_more_fingerings_than_strings() {
        let pitch_fingerings = vec![
            PitchFingering {
                string_number: StringNumber::new(1).unwrap(),
                fret: 9,
                pitch: Pitch::G4,
            },
            PitchFingering {
                string_number: StringNumber::new(2).unwrap(),
                fret: 0,
                pitch: Pitch::G4,
            },
        ];
        render_line(&Line::Playable(pitch_fingerings), 1);
    }
}

/// Creates a string with the fret number padded with dashes to match the maximum width.
///
/// # Panics
///
/// Panics if the width of the fret string representation is greater than `fret_width_max`.
fn render_fret(fret: u8, fret_width_max: usize) -> String {
    let fret_repr = fret.to_string();
    let fret_width = fret_repr.len();
    let filler_width = fret_width_max - fret_width;
    let filler: String = "-".repeat(filler_width);
    format!("{filler}{fret_repr}")
}
#[cfg(test)]
mod test_render_fret {
    use super::*;

    #[test]
    fn one_digit_in_one_digit_max() {
        assert_eq!(render_fret(4, 1), "4");
    }
    #[test]
    fn one_digit_in_two_digit_max() {
        assert_eq!(render_fret(3, 2), "-3");
    }
    #[test]
    fn two_digit_in_two_digit_max() {
        assert_eq!(render_fret(12, 2), "12");
    }
    #[test]
    #[should_panic]
    fn input_wider_than_max_width() {
        render_fret(123, 2);
    }
}

/// Calculates the maximum width of the the string representations of fret numbers in a given array of pitch fingerings.
fn calc_fret_width_max(pitch_fingerings: &[&PitchFingering]) -> usize {
    pitch_fingerings
        .iter()
        .map(|fingering| fingering.fret.to_string().len())
        .max()
        .expect("Playable line pitch fingerings should not be empty.")
}
#[cfg(test)]
mod test_calc_fret_width_max {
    use crate::string_number::StringNumber;

    use super::*;

    #[test]
    fn fret_width_one() {
        let fingering = PitchFingering {
            string_number: StringNumber::new(1).unwrap(),
            fret: 2,
            pitch: Pitch::G4,
        };
        assert_eq!(calc_fret_width_max(&[&fingering]), 1);
    }

    #[test]
    fn fret_width_one_multiple_fingerings() {
        let fingering1 = PitchFingering {
            string_number: StringNumber::new(1).unwrap(),
            fret: 0,
            pitch: Pitch::G4,
        };
        let fingering2 = PitchFingering {
            string_number: StringNumber::new(2).unwrap(),
            fret: 2,
            pitch: Pitch::G4,
        };
        let fingering3 = PitchFingering {
            string_number: StringNumber::new(5).unwrap(),
            fret: 8,
            pitch: Pitch::G4,
        };
        let fingerings = vec![&fingering1, &fingering2, &fingering3];
        assert_eq!(calc_fret_width_max(&fingerings), 1);
    }
    #[test]
    fn fret_width_two_multiple_fingerings() {
        let fingering1 = PitchFingering {
            string_number: StringNumber::new(1).unwrap(),
            fret: 2,
            pitch: Pitch::G4,
        };
        let fingering2 = PitchFingering {
            string_number: StringNumber::new(2).unwrap(),
            fret: 11,
            pitch: Pitch::G4,
        };
        let fingering3 = PitchFingering {
            string_number: StringNumber::new(4).unwrap(),
            fret: 3,
            pitch: Pitch::G4,
        };
        let fingerings = vec![&fingering1, &fingering2, &fingering3];
        assert_eq!(calc_fret_width_max(&fingerings), 2);
    }

    #[test]
    #[should_panic]
    fn empty_input() {
        let fingerings: Vec<&PitchFingering> = Vec::new();
        calc_fret_width_max(&fingerings);
    }
}

fn transpose<T>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
    assert!(!v.is_empty());
    let len = v[0].len();
    let mut iters: Vec<_> = v.into_iter().map(|n| n.into_iter()).collect();
    (0..len)
        .map(|_| {
            iters
                .iter_mut()
                .map(|n| n.next().unwrap())
                .collect::<Vec<T>>()
        })
        .collect()
}

// TODO! Handle duplicate pitches in the same line? BeatVec -> Hashset?
pub fn create_arrangements(
    guitar: Guitar,
    input_pitches: Vec<Line<BeatVec<Pitch>>>,
    num_arrangements: u8,
) -> Result<Vec<Arrangement>> {
    const MAX_NUM_ARRANGEMENTS: u8 = 20;
    match num_arrangements {
        1..=MAX_NUM_ARRANGEMENTS => (),
        0 => return Err(anyhow!("No arrangements were requested.")),
        _ => {
            return Err(anyhow!(
                "Too many arrangements to calculate. The maximum is {}.",
                MAX_NUM_ARRANGEMENTS
            ))
        }
    };

    let pitch_fingering_candidates: Vec<Line<BeatVec<PitchVec<PitchFingering>>>> =
        validate_fingerings(&guitar, &input_pitches)?;

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
        return Err(anyhow!("No arrangements could be calculated."));
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

        let error = create_arrangements(Guitar::default(), input_pitches, 1).unwrap_err();
        let error_msg = format!("{error}");
        assert_eq!(error_msg, "No arrangements could be calculated.");
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
                    non_zero_avg_fret: Some(OrderedFloat(0.1)),
                    non_zero_fret_span: 0,
                },
            },
            Node::Note {
                line_index: 0,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    non_zero_avg_fret: Some(OrderedFloat(0.2)),
                    non_zero_fret_span: 0,
                },
            },
            Node::Note {
                line_index: 1,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    non_zero_avg_fret: Some(OrderedFloat(1.1)),
                    non_zero_fret_span: 1,
                },
            },
            Node::Rest { line_index: 2 },
            Node::Rest { line_index: 3 },
            Node::Note {
                line_index: 4,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    non_zero_avg_fret: Some(OrderedFloat(4.1)),
                    non_zero_fret_span: 4,
                },
            },
            Node::Note {
                line_index: 4,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    non_zero_avg_fret: Some(OrderedFloat(4.1)),
                    non_zero_fret_span: 4,
                },
            },
        ]
    }

    #[test]
    fn from_start_to_note() {
        let current_node = Node::Start;

        let expected_nodes_and_costs = vec![
            Node::Note {
                line_index: 0,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    non_zero_avg_fret: Some(OrderedFloat(0.1)),
                    non_zero_fret_span: 0,
                },
            },
            Node::Note {
                line_index: 0,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    non_zero_avg_fret: Some(OrderedFloat(0.2)),
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
                non_zero_avg_fret: Some(OrderedFloat(0.1)),
                non_zero_fret_span: 0,
            },
        };

        let expected_nodes_and_costs = vec![Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                non_zero_avg_fret: Some(OrderedFloat(1.1)),
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
                non_zero_avg_fret: Some(OrderedFloat(1.1)),
                non_zero_fret_span: 1,
            },
        };

        let expected_nodes_and_costs = vec![Node::Rest { line_index: 2 }]
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

        let expected_nodes_and_costs = vec![Node::Rest { line_index: 3 }]
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

        let expected_nodes_and_costs = vec![
            Node::Note {
                line_index: 4,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    non_zero_avg_fret: Some(OrderedFloat(4.1)),
                    non_zero_fret_span: 4,
                },
            },
            Node::Note {
                line_index: 4,
                beat_fingering_combo: BeatFingeringCombo {
                    fingering_combo: vec![],
                    non_zero_avg_fret: Some(OrderedFloat(4.1)),
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
        } => beat_fingering_combo.non_zero_avg_fret,
        _ => None,
    };

    let (next_avg_fret, next_fret_span) = match next_node {
        Node::Start => unreachable!("Start should never be a future node."),
        Node::Rest { .. } => (None, 0.0),
        Node::Note {
            beat_fingering_combo,
            ..
        } => (
            beat_fingering_combo.non_zero_avg_fret,
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
                non_zero_avg_fret: Some(OrderedFloat(3.5)),
                non_zero_fret_span: 0,
            },
        };
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                non_zero_avg_fret: Some(OrderedFloat(3.5)),
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
                non_zero_avg_fret: Some(OrderedFloat(3.5)),
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
                non_zero_avg_fret: Some(OrderedFloat(3.5)),
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
                non_zero_avg_fret: Some(OrderedFloat(3.5)),
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
                non_zero_avg_fret: Some(OrderedFloat(3.0)),
                non_zero_fret_span: 0,
            },
        };
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                non_zero_avg_fret: Some(OrderedFloat(1.6)),
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
                non_zero_avg_fret: Some(OrderedFloat(4.133333)),
                non_zero_fret_span: 0,
            },
        };
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                non_zero_avg_fret: Some(OrderedFloat(4.133333)),
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
                non_zero_avg_fret: Some(OrderedFloat(5.0)),
                non_zero_fret_span: 0,
            },
        };
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                non_zero_avg_fret: Some(OrderedFloat(2.0)),
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
                non_zero_avg_fret: Some(OrderedFloat(7.3333333)),
                non_zero_fret_span: 0,
            },
        };
        let next_node = Node::Note {
            line_index: 1,
            beat_fingering_combo: BeatFingeringCombo {
                fingering_combo: vec![],
                non_zero_avg_fret: Some(OrderedFloat(3.6666666)),
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
            non_zero_avg_fret: Some(OrderedFloat(3.0)),
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
            non_zero_avg_fret: Some(OrderedFloat(3.0)),
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
