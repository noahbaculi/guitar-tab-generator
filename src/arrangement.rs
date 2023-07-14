use crate::{
    guitar::{generate_pitch_fingerings, PitchFingering},
    Guitar, Pitch,
};
use anyhow::{anyhow, Result};
use average::Mean;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use pathfinding::prelude::dijkstra;
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

// #[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
// struct

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
type BeatVec<T> = Vec<T>;
// type Candidates<T> = Vec<T>;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[allow(dead_code)]
pub struct BeatFingeringCombo {
    fingering_combo: BeatVec<PitchFingering>,
    non_zero_avg_fret: OrderedFloat<f32>,
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
            non_zero_avg_fret: OrderedFloat(
                beat_fingering_candidate
                    .clone()
                    .iter()
                    .filter(|fingering| fingering.fret != 0)
                    .map(|fingering| fingering.fret as f64)
                    .collect::<Mean>()
                    .mean() as f32,
            ),

            non_zero_fret_span: calc_fret_span(beat_fingering_candidate).unwrap_or(0),
        }
    }
}

#[derive(Debug)]
pub struct Arrangement {}

impl Arrangement {
    pub fn new(guitar: Guitar, input_pitches: Vec<Line<BeatVec<Pitch>>>) -> Result<Self> {
        let pitch_fingering_candidates: Vec<Line<BeatVec<PitchVec<PitchFingering>>>> =
            validate_fingerings(&guitar, &input_pitches)?;

        let measure_break_indices = pitch_fingering_candidates
            .iter()
            .enumerate()
            .filter(|(.., line_candidate)| line_candidate == &&MeasureBreak)
            .map(|(line_index, ..)| line_index);

        let path_nodes_groups: Vec<BeatVec<Node>> = pitch_fingering_candidates
            .iter()
            .filter(|&line_candidate| line_candidate != &MeasureBreak)
            .enumerate()
            .map(|(line_index, line_candidate)| match line_candidate {
                MeasureBreak => vec![],
                Rest => vec![Node::Rest {
                    line_index: line_index as u16,
                }],
                Playable(beat_fingerings_per_pitch) => {
                    generate_fingering_combos(beat_fingerings_per_pitch)
                        .iter()
                        .map(|beat_fingering_combo| Node::Note {
                            line_index: line_index as u16,
                            beat_fingering_combo: beat_fingering_combo.clone(),
                        })
                        .collect()
                }
            })
            .collect();

        let num_path_node_groups = path_nodes_groups.len();
        let path_nodes: Vec<Node> = path_nodes_groups.into_iter().flatten().collect_vec();

        let path_result = dijkstra(
            &Node::Start,
            |current_node| calc_next_nodes(current_node, path_nodes.clone()),
            |current_node| match current_node {
                Node::Start => false,
                Node::Rest { line_index } | Node::Note { line_index, .. } => {
                    *line_index == (num_path_node_groups - 1) as u16
                }
            },
        );
        // dbg!(&path_result);

        let mut path_lines = path_result
            .expect("Path should exist.")
            .0
            .into_iter()
            .filter(|node| node != &Node::Start)
            .map(|node| match node {
                Node::Start => panic!("Start node should already have been filtered out."),
                Node::Rest { .. } => Line::Rest,
                Node::Note {
                    beat_fingering_combo,
                    ..
                } => Line::Playable(beat_fingering_combo.fingering_combo),
            })
            .collect_vec();

        for measure_break_index in measure_break_indices.sorted() {
            path_lines.insert(measure_break_index, Line::MeasureBreak);
        }

        dbg!(&path_lines);

        // const WARNING_FRET_SPAN: u8 = 4;

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
        let guitar = generate_standard_guitar();
        let input_pitches = vec![
            Playable(vec![Pitch::G3]),
            Playable(vec![Pitch::B3]),
            Playable(vec![Pitch::D4, Pitch::G4]),
        ];
        let expected_fingerings = vec![
            Playable(vec![generate_pitch_fingerings(
                &guitar.string_ranges,
                &Pitch::G3,
            )]),
            Playable(vec![generate_pitch_fingerings(
                &guitar.string_ranges,
                &Pitch::B3,
            )]),
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
        let guitar = generate_standard_guitar();
        let input_pitches = vec![Playable(vec![Pitch::B9])];

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
            Playable(vec![Pitch::A1]),
            Playable(vec![Pitch::G3]),
            Playable(vec![Pitch::B3]),
            Playable(vec![Pitch::A1, Pitch::B1]),
            Playable(vec![Pitch::G3, Pitch::D2]),
            Playable(vec![Pitch::D4, Pitch::G4]),
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

fn generate_fingering_combos(
    beat_fingerings_per_pitch: &[Vec<PitchFingering>],
) -> BeatVec<BeatFingeringCombo> {
    beat_fingerings_per_pitch
        .iter()
        .multi_cartesian_product()
        .filter(no_duplicate_strings)
        .map(BeatFingeringCombo::new)
        .collect::<Vec<_>>()
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
    use crate::StringNumber;

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
            pitch: Pitch::CSharp2,
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
            pitch: Pitch::DSharp6,
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
            pitch: Pitch::CSharp2,
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
            pitch: Pitch::CSharp2,
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
            pitch: Pitch::DSharp6,
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
            pitch: Pitch::CSharp2,
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
            pitch: Pitch::DSharp6,
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

fn calc_next_nodes(current_node: &Node, path_nodes: Vec<Node>) -> Vec<(Node, i32)> {
    let next_node_index = match current_node {
        Node::Start => 0,
        Node::Rest { line_index } | Node::Note { line_index, .. } => line_index + 1,
    };

    let next_nodes = path_nodes
        .iter()
        .filter(|&node| {
            next_node_index
                == match node {
                    Node::Start => panic!("Start should never be a future node."),
                    Node::Rest { line_index } | Node::Note { line_index, .. } => *line_index,
                }
        })
        .map(|next_node| {
            (
                next_node.clone(),
                calculate_node_cost(current_node, next_node),
            )
        })
        .collect_vec();

    // dbg!(&next_nodes);

    next_nodes
}

fn calculate_node_cost(current_node: &Node, next_node: &Node) -> i32 {
    let current_avg_fret = match current_node {
        Node::Start => return 0,
        Node::Rest { .. } => return 0,
        Node::Note {
            beat_fingering_combo,
            ..
        } => beat_fingering_combo.non_zero_avg_fret,
    };

    let (next_avg_fret, next_fret_span) = match next_node {
        Node::Start => panic!("Start should never be a future node."),
        Node::Rest { .. } => return 0,
        Node::Note {
            beat_fingering_combo,
            ..
        } => (
            beat_fingering_combo.non_zero_avg_fret,
            beat_fingering_combo.non_zero_fret_span,
        ),
    };

    let avg_fret_difference = (next_avg_fret - current_avg_fret).abs();

    (avg_fret_difference * 10.0) as i32 + next_fret_span as i32
}
