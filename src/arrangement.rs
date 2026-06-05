use crate::{
    error::{TabError, UnplayablePitch},
    guitar::{Guitar, PitchFingering, generate_pitch_fingerings},
    pitch::Pitch,
};
use average::Mean;
use itertools::Itertools;
use memoize::memoize;
use ordered_float::OrderedFloat;
use pathfinding::prelude::yen;
use std::{collections::HashSet, rc::Rc};

/// One logical line of a parsed or arranged composition.
///
/// `Playable` holds the line's content (pitches during parsing, fingerings after
/// arrangement). `Rest` is an empty or comment-only line. `MeasureBreak` is a bar
/// line drawn in the rendered tab.
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
    Playable {
        line_index: u16,
        scored_beat_fingering: Rc<ScoredBeatFingering>,
    },
}

/// One pitch's set of candidate `PitchFingering`s across the guitar's strings.
pub(crate) type PitchVec<T> = Vec<T>;
/// One beat's worth of items (usually `Pitch` or `PitchFingering`).
pub type BeatVec<T> = Vec<T>;

/// Index of the first `Playable` line in `lines`, or `0` if the sequence has none.
///
/// Both `generate_arrangements` and `create_arrangements` skip leading rests before
/// shipping the input downstream, so the predicate lives in one place.
pub(crate) fn first_playable_index<T>(lines: &[Line<T>]) -> usize {
    lines
        .iter()
        .position(|line| matches!(line, Playable(_)))
        .unwrap_or(0)
}

/// A single playable assignment of fingerings for one beat, with precomputed difficulty
/// features (average non-zero fret, non-zero fret span).
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct ScoredBeatFingering {
    beat_fingering: BeatVec<PitchFingering>,
    avg_non_zero_fret: Option<OrderedFloat<f64>>,
    non_zero_fret_span: u8,
}
impl ScoredBeatFingering {
    /// Builds a `ScoredBeatFingering` from a per-beat `PitchFingering` list, precomputing
    /// the difficulty features used to score pathfinding transitions.
    pub(crate) fn new(beat_fingering_candidate: BeatVec<PitchFingering>) -> Self {
        let avg_non_zero_fret = calc_avg_non_zero_fret(&beat_fingering_candidate);
        let non_zero_fret_span = calc_fret_span(&beat_fingering_candidate).unwrap_or(0);

        ScoredBeatFingering {
            beat_fingering: beat_fingering_candidate,
            avg_non_zero_fret,
            non_zero_fret_span,
        }
    }
}
#[cfg(test)]
mod test_create_scored_beat_fingering {
    use super::*;
    use crate::string_number::StringNumber;

    #[test]
    fn simple() {
        let pitch_fingering_1 = PitchFingering {
            pitch: Pitch::A0,
            string_number: StringNumber::new(1).unwrap(),
            fret: 2,
        };

        let ScoredBeatFingering {
            beat_fingering,
            avg_non_zero_fret,
            non_zero_fret_span,
        } = ScoredBeatFingering::new(vec![pitch_fingering_1]);

        assert_eq!(beat_fingering, vec![pitch_fingering_1]);
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

        let ScoredBeatFingering {
            beat_fingering,
            avg_non_zero_fret,
            non_zero_fret_span,
        } = ScoredBeatFingering::new(vec![
            pitch_fingering_1,
            pitch_fingering_2,
            pitch_fingering_3,
            pitch_fingering_4,
        ]);

        assert_eq!(
            beat_fingering,
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
    beat_fingering_candidate: &[PitchFingering],
) -> Option<OrderedFloat<f64>> {
    let non_zero_fingerings = beat_fingering_candidate
        .iter()
        .filter(|fingering| fingering.fret != 0)
        .map(|fingering| fingering.fret as f64)
        .collect::<Mean>();

    if non_zero_fingerings.is_empty() {
        None
    } else {
        Some(OrderedFloat(non_zero_fingerings.mean()))
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
            calc_avg_non_zero_fret(&[pitch_fingering_1]),
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

        assert_eq!(calc_avg_non_zero_fret(&[pitch_fingering_1]), None);
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
            calc_avg_non_zero_fret(&[pitch_fingering_1, pitch_fingering_2]),
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
                pitch_fingering_1,
                pitch_fingering_2,
                pitch_fingering_3,
                pitch_fingering_4,
            ]),
            Some(OrderedFloat(8.0 / 3.0))
        );
    }
}

/// A single ranked guitar arrangement: one fingering choice per beat, ordered by line.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Arrangement {
    pub(crate) lines: Vec<Line<BeatVec<PitchFingering>>>,
    difficulty: i32,
    max_fret_span: u8,
}
impl Arrangement {
    /// Pass directly to [`crate::render_tab`].
    #[must_use]
    pub fn lines(&self) -> &[Line<BeatVec<PitchFingering>>] {
        &self.lines
    }

    /// The maximum non-zero fret span reached on any beat in this arrangement.
    ///
    /// Useful as a coarse "playability" gauge: a smaller span means less hand stretch.
    #[must_use]
    pub fn max_fret_span(&self) -> u8 {
        self.max_fret_span
    }

    /// The difficulty score of this arrangement. Lower is easier. Equal to the sum of
    /// transition difficulties along the chosen path through the fingering graph.
    #[must_use]
    pub fn difficulty(&self) -> i32 {
        self.difficulty
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

/// Computes the N best-scoring guitar arrangements for a parsed sequence of pitches,
/// ranked by ascending difficulty.
///
/// # Errors
///
/// Returns an error if any input line cannot be fingered on the supplied `guitar`
/// (out-of-range pitches). `num_arrangements` is range-checked by
/// [`crate::NumArrangements::try_new`] at construction.
///
/// # Panics
///
/// Panics only if an internal invariant is violated (a BUG condition, not reachable
/// under any valid input): a `MeasureBreak` line leaking past the pathfinding filter,
/// or a `Node::Start` appearing as a future node during path traversal.
#[memoize(Capacity: 10)]
pub fn create_arrangements(
    guitar: Guitar,
    input_lines: Vec<Line<BeatVec<Pitch>>>,
    num_arrangements: crate::NumArrangements,
    max_fret_span_filter: Option<u8>,
) -> Result<Vec<Arrangement>, TabError> {
    let input_playable_lines = input_lines
        .iter()
        .filter(|line| matches!(line, Line::Playable(_)))
        .collect_vec();
    if input_playable_lines.is_empty() {
        let empty_arrangements = vec![
            Arrangement {
                lines: vec![],
                difficulty: 0,
                max_fret_span: 0,
            };
            num_arrangements.get() as usize
        ];
        return Ok(empty_arrangements);
    }

    let first_playable_index = first_playable_index(&input_lines);

    // Validate against the full input so `UnplayablePitch.line` carries the original 1-indexed
    // input line, then drop the leading rests for pathfinding. Skipping before validation would
    // report the line relative to the post-skip beat sequence (off by the leading-rest count).
    let pitch_fingering_candidates: Vec<Line<BeatVec<PitchVec<PitchFingering>>>> =
        validate_fingerings(&guitar, &input_lines)?
            .into_iter()
            .skip(first_playable_index)
            .collect_vec();

    let measure_break_indices: Vec<usize> = pitch_fingering_candidates
        .iter()
        .enumerate()
        .filter(|(.., line_candidate)| matches!(line_candidate, MeasureBreak))
        .map(|(line_index, ..)| line_index)
        .collect_vec();

    let path_node_groups: Vec<BeatVec<Node>> = pitch_fingering_candidates
        .into_iter()
        .filter(|line_candidate| !matches!(line_candidate, MeasureBreak))
        .enumerate()
        .map(|(line_index, line_candidate)| match line_candidate {
            MeasureBreak => unreachable!("Measure breaks should have been filtered out."),
            // `line_index as u16` cannot truncate: `parse_lines` caps input at u16::MAX
            // lines, so the beat index always fits.
            Rest => vec![Node::Rest {
                line_index: line_index as u16,
            }],
            Playable(beat_fingerings_per_pitch) => {
                generate_beat_fingerings(&beat_fingerings_per_pitch)
                    .into_iter()
                    .map(|pitch_fingering_group| Node::Playable {
                        line_index: line_index as u16,
                        scored_beat_fingering: Rc::new(ScoredBeatFingering::new(
                            pitch_fingering_group,
                        )),
                    })
                    .collect()
            }
        })
        .collect::<Vec<_>>();

    let num_path_node_groups = path_node_groups.len();

    let path_nodes: Vec<Node> = path_node_groups.into_iter().flatten().collect_vec();

    let path_results: Vec<(Vec<Node>, i32)> = yen(
        &Node::Start,
        |current_node| calc_next_nodes(current_node, &path_nodes),
        |current_node| match current_node {
            Node::Start => false,
            Node::Rest { line_index } | Node::Playable { line_index, .. } => {
                // Pathfinding goal is reached when the node is in the last node group
                *line_index == (num_path_node_groups - 1) as u16
            }
        },
        num_arrangements.get() as usize,
    );
    if path_results.is_empty() {
        return Err(TabError::NoArrangementsFound);
    }

    let mut arrangements = path_results
        .into_iter()
        .map(|path_result| process_path(path_result.0, path_result.1, &measure_break_indices))
        .collect_vec();

    if let Some(max_span) = max_fret_span_filter {
        arrangements.retain(|a| a.max_fret_span() <= max_span);
    }

    Ok(arrangements)
}
#[cfg(test)]
mod test_create_arrangements {
    use super::*;
    use crate::NumArrangements;
    use crate::parser::parse_lines;
    use crate::string_number::StringNumber;

    #[test]
    fn unreachable_pitch_returns_unplayable_pitches_variant() {
        let lines = parse_lines("A1".to_owned()).unwrap();
        let n = NumArrangements::try_new(1).unwrap();
        let err = create_arrangements(Guitar::default(), lines, n, None).unwrap_err();
        match err {
            TabError::UnplayablePitches { pitches } => {
                assert_eq!(pitches.len(), 1);
                assert_eq!(pitches[0].value, "A1");
                assert_eq!(pitches[0].line, 1);
            }
            other => panic!("expected UnplayablePitches, got {other:?}"),
        }
    }

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

        let arrangements = create_arrangements(
            Guitar::default(),
            input_pitches,
            NumArrangements::try_new(1).unwrap(),
            None,
        )
        .unwrap();

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

        let arrangements = create_arrangements(
            Guitar::default(),
            input_pitches,
            NumArrangements::try_new(10).unwrap(),
            None,
        )
        .unwrap();

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

        let arrangements = create_arrangements(
            Guitar::default(),
            input_pitches,
            NumArrangements::try_new(1).unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(arrangements, expected_arrangements);
    }
    #[test]
    fn duplicate_pitches_in_beat_yield_no_arrangements() {
        // Each E2 is individually playable, but the no-duplicate-strings constraint filters
        // every fingering combination for the beat, so pathfinding finds no route and
        // `create_arrangements` reports `NoArrangementsFound`.
        let input_pitches = vec![Line::Playable(vec![Pitch::E2, Pitch::E2])];

        let err = create_arrangements(
            Guitar::default(),
            input_pitches,
            NumArrangements::try_new(1).unwrap(),
            None,
        )
        .unwrap_err();

        assert!(matches!(err, TabError::NoArrangementsFound), "got {err:?}");
    }
    #[test]
    fn empty_input() {
        let input_pitches: Vec<Line<BeatVec<Pitch>>> = vec![];

        let arrangements = create_arrangements(
            Guitar::default(),
            input_pitches,
            NumArrangements::try_new(2).unwrap(),
            None,
        )
        .unwrap();

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

        let arrangements = create_arrangements(
            Guitar::default(),
            input_pitches,
            NumArrangements::try_new(1).unwrap(),
            None,
        )
        .unwrap();

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
    fn max_fret_span_filter_drops_high_span_arrangements() {
        let tuning =
            crate::guitar::create_string_tuning(&crate::guitar::STD_6_STRING_TUNING_OPEN_PITCHES)
                .unwrap();
        let guitar = crate::guitar::Guitar::new(tuning, 20, 0).unwrap();
        // G2B4 is a chord beat: G2 lands at fret 3 on string 6, B4 at fret 7 on string 1.
        // Some arrangements will have both notes at non-zero frets, producing a span > 0.
        let lines = crate::parser::parse_lines("G2B4".to_owned()).unwrap();

        // Without a filter, at least one arrangement has a non-zero fret span.
        let unfiltered = create_arrangements(
            guitar.clone(),
            lines.clone(),
            NumArrangements::try_new(5).unwrap(),
            None,
        )
        .unwrap();
        assert!(unfiltered.iter().any(|a| a.max_fret_span() > 0));

        // With filter = Some(0), only arrangements that never stretch survive.
        let filtered = create_arrangements(
            guitar.clone(),
            lines,
            NumArrangements::try_new(5).unwrap(),
            Some(0),
        )
        .unwrap();
        assert!(filtered.iter().all(|a| a.max_fret_span() == 0));
        assert!(filtered.len() <= 5);
    }
    #[test]
    fn max_fret_span_filter_can_produce_empty_set() {
        let tuning =
            crate::guitar::create_string_tuning(&crate::guitar::STD_6_STRING_TUNING_OPEN_PITCHES)
                .unwrap();
        let guitar = crate::guitar::Guitar::new(tuning, 20, 0).unwrap();
        // C3E3 forces both notes onto fretted positions (neither is an open string in
        // standard tuning), so every candidate arrangement has a non-zero fret span.
        let lines = crate::parser::parse_lines("C3E3".to_owned()).unwrap();

        let filtered =
            create_arrangements(guitar, lines, NumArrangements::try_new(5).unwrap(), Some(0))
                .expect("filter dropping every candidate is not an error");
        assert!(
            filtered.is_empty(),
            "max_fret_span_filter=Some(0) on an all-fretted chord must drop every candidate",
        );
    }

    #[test]
    fn max_fret_span_filter_keeps_only_low_span_arrangements() {
        let tuning =
            crate::guitar::create_string_tuning(&crate::guitar::STD_6_STRING_TUNING_OPEN_PITCHES)
                .unwrap();
        let guitar = crate::guitar::Guitar::new(tuning, 20, 0).unwrap();
        // C3G3: G3 is open on string 3, C3 must be fretted. Across the 5 best arrangements,
        // some keep the fretted notes tight (span 0) and some stretch (span > 0), so a
        // Some(0) filter drops a strict subset rather than all or none.
        let lines = crate::parser::parse_lines("C3G3".to_owned()).unwrap();

        let unfiltered = create_arrangements(
            guitar.clone(),
            lines.clone(),
            NumArrangements::try_new(5).unwrap(),
            None,
        )
        .unwrap();
        assert_eq!(unfiltered.len(), 5);
        assert!(
            unfiltered.iter().any(|a| a.max_fret_span() == 0),
            "expected at least one span-0 arrangement"
        );
        assert!(
            unfiltered.iter().any(|a| a.max_fret_span() > 0),
            "expected at least one span>0 arrangement so the filter does real work"
        );

        let filtered =
            create_arrangements(guitar, lines, NumArrangements::try_new(5).unwrap(), Some(0))
                .unwrap();
        // Exactly the two span-0 arrangements survive: 0 < 2 < 5, exercising the
        // "return what we have" fallback (fewer than num_arrangements, no error).
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|a| a.max_fret_span() == 0));
    }
}

/// Generates the candidate `PitchFingering`s for every pitch in each beat.
///
/// Returns the per-beat fingerings on success, or [`TabError::UnplayablePitches`] listing
/// every pitch that reached no string on the configured guitar (with its 1-indexed input
/// line). All unplayable pitches are collected before returning, not just the first.
///
/// * `guitar`: the configured guitar, supplying per-string ranges.
/// * `input_pitches`: the parsed beats to place.
fn validate_fingerings(
    guitar: &Guitar,
    input_pitches: &[Line<BeatVec<Pitch>>],
) -> Result<Vec<Line<BeatVec<PitchVec<PitchFingering>>>>, TabError> {
    let mut unplayable_pitches: Vec<UnplayablePitch> = vec![];
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
                            unplayable_pitches.push(UnplayablePitch {
                                value: beat_pitch.plain_text().to_owned(),
                                line: (beat_index as u32) + 1,
                            })
                        }
                        pitch_fingerings
                    })
                    .collect(),
            ),
        })
        .collect();

    if !unplayable_pitches.is_empty() {
        return Err(TabError::UnplayablePitches {
            pitches: unplayable_pitches,
        });
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

        let err = validate_fingerings(&guitar, &input_pitches).unwrap_err();
        match err {
            TabError::UnplayablePitches { pitches } => {
                assert_eq!(pitches.len(), 1);
                assert_eq!(pitches[0].value, "B9");
                assert_eq!(pitches[0].line, 1);
            }
            other => panic!("expected UnplayablePitches, got {other:?}"),
        }
    }
    #[test]
    fn invalid_accidental_reports_plain_text_spelling() {
        // An unplayable accidental reports its plain-text spelling ("Db9"), matching the
        // normalized-input pitch strings, rather than the internal enum name ("CSharpDFlat9").
        let guitar = Guitar::default();
        let input_pitches = vec![Playable(vec![Pitch::CSharpDFlat9])];

        let err = validate_fingerings(&guitar, &input_pitches).unwrap_err();
        match err {
            TabError::UnplayablePitches { pitches } => {
                assert_eq!(pitches.len(), 1);
                assert_eq!(pitches[0].value, "Db9");
                assert_eq!(pitches[0].line, 1);
            }
            other => panic!("expected UnplayablePitches, got {other:?}"),
        }
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

        let err = validate_fingerings(&guitar, &input_pitches).unwrap_err();
        match err {
            TabError::UnplayablePitches { pitches } => {
                assert_eq!(pitches.len(), 4);
                assert_eq!(pitches[0].value, "A1");
                assert_eq!(pitches[0].line, 1);
                assert_eq!(pitches[1].value, "A1");
                assert_eq!(pitches[1].line, 4);
                assert_eq!(pitches[2].value, "B1");
                assert_eq!(pitches[2].line, 4);
                assert_eq!(pitches[3].value, "D2");
                assert_eq!(pitches[3].line, 5);
            }
            other => panic!("expected UnplayablePitches, got {other:?}"),
        }
    }
}

/// Generates all playable combinations of fingerings for all the pitches in a beat.
fn generate_beat_fingerings(
    beat_fingerings_per_pitch: &[Vec<PitchFingering>],
) -> Vec<BeatVec<PitchFingering>> {
    assert!(
        !beat_fingerings_per_pitch.is_empty(),
        "BUG: generate_beat_fingerings called with empty input"
    );

    beat_fingerings_per_pitch
        .iter()
        .multi_cartesian_product()
        .map(|combo| combo.into_iter().copied().collect::<Vec<PitchFingering>>())
        .filter(|x| no_duplicate_strings(x))
        .collect()
}
#[cfg(test)]
mod test_generate_beat_fingerings {
    use super::*;
    use crate::string_number::StringNumber;

    #[test]
    fn simple() {
        let pitch_fingering = PitchFingering {
            pitch: Pitch::B6,
            string_number: StringNumber::new(2).unwrap(),
            fret: 3,
        };

        let beat_fingerings_per_pitch = &[vec![pitch_fingering]];

        assert_eq!(
            generate_beat_fingerings(beat_fingerings_per_pitch),
            beat_fingerings_per_pitch
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
        let expected_beat_fingerings = vec![
            vec![pitch_fingering_a_string_2, pitch_fingering_b_string_3],
            vec![pitch_fingering_a_string_2, pitch_fingering_b_string_4],
            vec![pitch_fingering_a_string_3, pitch_fingering_b_string_2],
            vec![pitch_fingering_a_string_3, pitch_fingering_b_string_4],
        ];

        assert_eq!(
            generate_beat_fingerings(&beat_fingerings_per_pitch),
            expected_beat_fingerings
        );
    }

    #[test]
    #[should_panic(expected = "BUG: generate_beat_fingerings called with empty input")]
    fn empty_input_panics() {
        let _ = generate_beat_fingerings(&[]);
    }
}

/// Checks if there are any duplicate strings in a vector of `Fingering`
/// objects to ensure that all pitches can be played.
fn no_duplicate_strings(beat_fingering_option: &[PitchFingering]) -> bool {
    let mut seen_strings = HashSet::with_capacity(beat_fingering_option.len());
    beat_fingering_option
        .iter()
        .all(|fingering| seen_strings.insert(fingering.string_number))
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

        assert!(no_duplicate_strings(&[fingering_1]));
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

        assert!(no_duplicate_strings(&[
            fingering_1,
            fingering_2,
            fingering_3,
            fingering_4
        ]));
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

        assert!(!no_duplicate_strings(&[fingering_1, fingering_2]));
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

        assert!(!no_duplicate_strings(&[
            fingering_1,
            fingering_2,
            fingering_3,
            fingering_4,
        ]));
    }
    #[test]
    fn empty_input() {
        assert!(no_duplicate_strings(&[]));
    }
}

/// Calculates the difference between the maximum and minimum non-zero
/// fret numbers in a given vector of fingerings.
fn calc_fret_span(beat_fingering_candidate: &[PitchFingering]) -> Option<u8> {
    use itertools::MinMaxResult;

    let non_zero_frets = beat_fingering_candidate
        .iter()
        .filter(|fingering| fingering.fret != 0)
        .map(|fingering| fingering.fret);

    match non_zero_frets.minmax() {
        MinMaxResult::NoElements => None,
        MinMaxResult::OneElement(_) => Some(0),
        // `minmax()` guarantees `max >= min`, so this `u8` subtraction cannot underflow.
        MinMaxResult::MinMax(min, max) => Some(max - min),
    }
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

        assert_eq!(calc_fret_span(&[fingering_1]).unwrap(), 0);
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
        let beat_fingering_option = &[fingering_1, fingering_2, fingering_3, fingering_4];

        assert_eq!(calc_fret_span(beat_fingering_option).unwrap(), 3);
    }
    #[test]
    fn empty_input() {
        assert!(calc_fret_span(&[]).is_none());
    }
}

type NodeDifficulty = i32;

/// Calculates the next nodes and their transition difficulties based on the current node
/// and a list of all path nodes.
///
/// Returns a vector of tuples, where each tuple contains a `Node` and the `NodeDifficulty`
/// of moving to that node.
fn calc_next_nodes(current_node: &Node, path_nodes: &[Node]) -> Vec<(Node, NodeDifficulty)> {
    let next_node_index = match current_node {
        Node::Start => 0,
        // `parse_lines` caps accepted input at `MAX_INPUT_LINES` (`u16::MAX`), so the largest
        // `line_index` is `u16::MAX - 1` and this `+ 1` reaches at most `u16::MAX`: no overflow.
        Node::Rest { line_index } | Node::Playable { line_index, .. } => line_index + 1,
    };

    let next_nodes: Vec<(Node, NodeDifficulty)> = path_nodes
        .iter()
        .filter(|&node| {
            next_node_index
                == match node {
                    Node::Start => unreachable!("Start should never be a future node."),
                    Node::Rest { line_index } | Node::Playable { line_index, .. } => *line_index,
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

    fn create_test_path_nodes() -> [Node; 7] {
        [
            Node::Playable {
                line_index: 0,
                scored_beat_fingering: Rc::new(ScoredBeatFingering {
                    beat_fingering: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(0.1)),
                    non_zero_fret_span: 0,
                }),
            },
            Node::Playable {
                line_index: 0,
                scored_beat_fingering: Rc::new(ScoredBeatFingering {
                    beat_fingering: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(0.2)),
                    non_zero_fret_span: 0,
                }),
            },
            Node::Playable {
                line_index: 1,
                scored_beat_fingering: Rc::new(ScoredBeatFingering {
                    beat_fingering: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(1.1)),
                    non_zero_fret_span: 1,
                }),
            },
            Node::Rest { line_index: 2 },
            Node::Rest { line_index: 3 },
            Node::Playable {
                line_index: 4,
                scored_beat_fingering: Rc::new(ScoredBeatFingering {
                    beat_fingering: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(4.1)),
                    non_zero_fret_span: 4,
                }),
            },
            Node::Playable {
                line_index: 4,
                scored_beat_fingering: Rc::new(ScoredBeatFingering {
                    beat_fingering: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(4.1)),
                    non_zero_fret_span: 4,
                }),
            },
        ]
    }

    #[test]
    fn from_start_to_note() {
        let current_node = Node::Start;

        let expected_nodes_and_costs = [
            Node::Playable {
                line_index: 0,
                scored_beat_fingering: Rc::new(ScoredBeatFingering {
                    beat_fingering: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(0.1)),
                    non_zero_fret_span: 0,
                }),
            },
            Node::Playable {
                line_index: 0,
                scored_beat_fingering: Rc::new(ScoredBeatFingering {
                    beat_fingering: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(0.2)),
                    non_zero_fret_span: 0,
                }),
            },
        ]
        .iter()
        .map(|node| (node.clone(), calculate_node_difficulty(&current_node, node)))
        .collect_vec();

        assert_eq!(
            calc_next_nodes(&current_node, &create_test_path_nodes()),
            expected_nodes_and_costs
        );
    }
    #[test]
    fn from_note_to_note() {
        let current_node = Node::Playable {
            line_index: 0,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(0.1)),
                non_zero_fret_span: 0,
            }),
        };

        let expected_nodes_and_costs = [Node::Playable {
            line_index: 1,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(1.1)),
                non_zero_fret_span: 1,
            }),
        }]
        .iter()
        .map(|node| (node.clone(), calculate_node_difficulty(&current_node, node)))
        .collect_vec();

        assert_eq!(
            calc_next_nodes(&current_node, &create_test_path_nodes()),
            expected_nodes_and_costs
        );
    }
    #[test]
    fn from_note_to_rest() {
        let current_node = Node::Playable {
            line_index: 1,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(1.1)),
                non_zero_fret_span: 1,
            }),
        };

        let expected_nodes_and_costs = [Node::Rest { line_index: 2 }]
            .iter()
            .map(|node| (node.clone(), calculate_node_difficulty(&current_node, node)))
            .collect_vec();

        assert_eq!(
            calc_next_nodes(&current_node, &create_test_path_nodes()),
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
            calc_next_nodes(&current_node, &create_test_path_nodes()),
            expected_nodes_and_costs
        );
    }
    #[test]
    fn from_rest_to_note() {
        let current_node = Node::Rest { line_index: 3 };

        let expected_nodes_and_costs = [
            Node::Playable {
                line_index: 4,
                scored_beat_fingering: Rc::new(ScoredBeatFingering {
                    beat_fingering: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(4.1)),
                    non_zero_fret_span: 4,
                }),
            },
            Node::Playable {
                line_index: 4,
                scored_beat_fingering: Rc::new(ScoredBeatFingering {
                    beat_fingering: vec![],
                    avg_non_zero_fret: Some(OrderedFloat(4.1)),
                    non_zero_fret_span: 4,
                }),
            },
        ]
        .iter()
        .map(|node| (node.clone(), calculate_node_difficulty(&current_node, node)))
        .collect_vec();

        assert_eq!(
            calc_next_nodes(&current_node, &create_test_path_nodes()),
            expected_nodes_and_costs
        );
    }

    #[test]
    #[should_panic]
    fn to_start() {
        calc_next_nodes(
            &Node::Rest { line_index: 3 },
            &[Node::Rest { line_index: 4 }, Node::Start],
        );
    }
}

/// Calculates the transition difficulty from one node to another based on the
/// average fret difference and fret span.
fn calculate_node_difficulty(current_node: &Node, next_node: &Node) -> NodeDifficulty {
    let current_avg_fret = match current_node {
        Node::Playable {
            scored_beat_fingering,
            ..
        } => scored_beat_fingering.avg_non_zero_fret,
        _ => None,
    };

    let (next_avg_fret, next_fret_span) = match next_node {
        Node::Start => unreachable!("Start should never be a future node."),
        Node::Rest { .. } => (None, 0.0),
        Node::Playable {
            scored_beat_fingering,
            ..
        } => (
            scored_beat_fingering.avg_non_zero_fret,
            scored_beat_fingering.non_zero_fret_span as f64,
        ),
    };

    let avg_fret_difference = match (current_avg_fret, next_avg_fret) {
        (Some(current_avg_fret_num), Some(next_avg_fret_num)) => {
            (next_avg_fret_num - current_avg_fret_num).abs()
        }
        _ => 0.0,
    };

    // The cast to i32 (NodeDifficulty) cannot overflow: every fret term is bounded by
    // Guitar::MAX_NUM_FRETS (30), so the weighted sum stays far inside i32, and the inputs are
    // finite because calc_avg_non_zero_fret yields None (scored as 0.0) for an all-open beat.
    ((avg_fret_difference * 100.0)
        + (next_fret_span * 10.0)
        + (next_avg_fret.unwrap_or(OrderedFloat(0.0))).into_inner()) as NodeDifficulty
}
#[cfg(test)]
mod test_calculate_node_difficulty {
    use super::*;

    #[test]
    fn simple_no_diff() {
        let current_node = Node::Playable {
            line_index: 0,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.5)),
                non_zero_fret_span: 0,
            }),
        };
        let next_node = Node::Playable {
            line_index: 1,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.5)),
                non_zero_fret_span: 0,
            }),
        };

        assert_eq!(calculate_node_difficulty(&current_node, &next_node), 3);
    }
    #[test]
    fn simple_from_start() {
        let next_node = Node::Playable {
            line_index: 1,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.5)),
                non_zero_fret_span: 0,
            }),
        };

        assert_eq!(calculate_node_difficulty(&Node::Start, &next_node), 3);
    }
    #[test]
    fn simple_from_rest() {
        let next_node = Node::Playable {
            line_index: 1,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.5)),
                non_zero_fret_span: 0,
            }),
        };

        assert_eq!(
            calculate_node_difficulty(&Node::Rest { line_index: 0 }, &next_node),
            3
        );
    }
    #[test]
    fn simple_to_rest() {
        let current_node = Node::Playable {
            line_index: 0,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.5)),
                non_zero_fret_span: 0,
            }),
        };

        assert_eq!(
            calculate_node_difficulty(&current_node, &Node::Rest { line_index: 1 }),
            0
        );
    }
    #[test]
    fn simple_avg_fret_diff() {
        let current_node = Node::Playable {
            line_index: 0,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.0)),
                non_zero_fret_span: 0,
            }),
        };
        let next_node = Node::Playable {
            line_index: 1,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(1.6)),
                non_zero_fret_span: 0,
            }),
        };

        assert_eq!(calculate_node_difficulty(&current_node, &next_node), 141);
    }
    #[test]
    fn simple_fret_span() {
        let current_node = Node::Playable {
            line_index: 0,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(4.133333)),
                non_zero_fret_span: 0,
            }),
        };
        let next_node = Node::Playable {
            line_index: 1,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(4.133333)),
                non_zero_fret_span: 3,
            }),
        };

        assert_eq!(calculate_node_difficulty(&current_node, &next_node), 34);
    }
    #[test]
    fn compound() {
        let current_node = Node::Playable {
            line_index: 0,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(5.0)),
                non_zero_fret_span: 0,
            }),
        };
        let next_node = Node::Playable {
            line_index: 1,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(2.0)),
                non_zero_fret_span: 5,
            }),
        };

        assert_eq!(calculate_node_difficulty(&current_node, &next_node), 352);
    }
    #[test]
    fn complex() {
        let current_node = Node::Playable {
            line_index: 0,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(7.3333333)),
                non_zero_fret_span: 0,
            }),
        };
        let next_node = Node::Playable {
            line_index: 1,
            scored_beat_fingering: Rc::new(ScoredBeatFingering {
                beat_fingering: vec![],
                avg_non_zero_fret: Some(OrderedFloat(3.6666666)),
                non_zero_fret_span: 4,
            }),
        };

        assert_eq!(calculate_node_difficulty(&current_node, &next_node), 410);
    }
}

fn process_path(
    path_nodes: Vec<Node>,
    path_difficulty: i32,
    measure_break_indices: &[usize],
) -> Arrangement {
    let mut lines: Vec<Line<BeatVec<PitchFingering>>> = path_nodes
        .iter()
        .filter(|node| node != &&Node::Start)
        .map(|node| match node {
            Node::Start => unreachable!("Start node should have been filtered out."),
            Node::Rest { .. } => Line::Rest,
            Node::Playable {
                scored_beat_fingering,
                ..
            } => Line::Playable(scored_beat_fingering.beat_fingering.clone()),
        })
        .collect_vec();
    // Re-inject measure breaks. `measure_break_indices` is built by `enumerate().filter()`
    // upstream, so it is already ascending; inserting low to high lands each break at its
    // original post-skip slot without shifting an earlier one.
    for &measure_break_index in measure_break_indices {
        lines.insert(measure_break_index, Line::MeasureBreak);
    }

    let max_fret_span: u8 = path_nodes
        .iter()
        .filter(|node| node != &&Node::Start)
        .filter_map(|node| match node {
            Node::Start => unreachable!("Start node should have been filtered out."),
            Node::Rest { .. } => None,
            Node::Playable {
                scored_beat_fingering,
                ..
            } => Some(scored_beat_fingering.non_zero_fret_span),
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
        let placeholder_scored_beat_fingering = ScoredBeatFingering {
            beat_fingering: vec![PitchFingering {
                pitch: Pitch::C4,
                string_number: StringNumber::new(1).unwrap(),
                fret: 3,
            }],
            avg_non_zero_fret: Some(OrderedFloat(3.0)),
            non_zero_fret_span: 0,
        };

        let path_nodes = vec![
            Node::Start,
            Node::Playable {
                line_index: 0,
                scored_beat_fingering: Rc::new(placeholder_scored_beat_fingering.clone()),
            },
        ];

        let arrangement = process_path(path_nodes, 123, &[]);

        let expected_arrangement = Arrangement {
            lines: vec![Playable(placeholder_scored_beat_fingering.beat_fingering)],
            difficulty: 123,
            max_fret_span: 0,
        };

        assert_eq!(arrangement, expected_arrangement);
    }
    #[test]
    fn complex() {
        let placeholder_scored_beat_fingering = ScoredBeatFingering {
            beat_fingering: vec![PitchFingering {
                pitch: Pitch::C4,
                string_number: StringNumber::new(1).unwrap(),
                fret: 3,
            }],
            avg_non_zero_fret: Some(OrderedFloat(3.0)),
            non_zero_fret_span: 4,
        };

        let path_nodes = vec![
            Node::Start,
            Node::Playable {
                line_index: 0,
                scored_beat_fingering: Rc::new(placeholder_scored_beat_fingering.clone()),
            },
            Node::Playable {
                line_index: 1,
                scored_beat_fingering: Rc::new(placeholder_scored_beat_fingering.clone()),
            },
            Node::Rest { line_index: 2 },
            Node::Playable {
                line_index: 3,
                scored_beat_fingering: Rc::new(placeholder_scored_beat_fingering.clone()),
            },
            Node::Playable {
                line_index: 4,
                scored_beat_fingering: Rc::new(placeholder_scored_beat_fingering.clone()),
            },
        ];

        let arrangement = process_path(path_nodes, 321, &[0, 2, 5, 7]);

        let expected_arrangement = Arrangement {
            lines: vec![
                MeasureBreak,
                Playable(placeholder_scored_beat_fingering.clone().beat_fingering),
                MeasureBreak,
                Playable(placeholder_scored_beat_fingering.clone().beat_fingering),
                Rest,
                MeasureBreak,
                Playable(placeholder_scored_beat_fingering.clone().beat_fingering),
                MeasureBreak,
                Playable(placeholder_scored_beat_fingering.beat_fingering),
            ],
            difficulty: 321,
            max_fret_span: 4,
        };

        assert_eq!(arrangement, expected_arrangement);
    }
}

// `proptest` is a non-wasm dev-dependency (it does not compile for `wasm32`), so this module
// is gated off the wasm test build alongside it.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod proptest_invariants {
    use super::*;
    use crate::NumArrangements;
    use crate::guitar::{STD_6_STRING_TUNING_OPEN_PITCHES, create_string_tuning};
    use proptest::prelude::*;
    use std::collections::HashSet;

    fn any_pitch() -> impl Strategy<Value = Pitch> {
        // E2 (index 28) through C6 (index 72): a comfortable range for a std-tuned
        // 6-string guitar and ensures generate_pitch_fingerings returns >=1 candidate.
        (28usize..=72usize).prop_map(|idx| Pitch::from_repr(idx).expect("BUG: index in range"))
    }

    #[derive(Debug, Clone)]
    #[allow(dead_code)] // fields are surfaced via Debug when proptest shrinks a failing case
    struct ArrangementCase {
        input_lines: Vec<Line<BeatVec<Pitch>>>,
        num_arrangements: NumArrangements,
        measure_break_positions: Vec<usize>,
        rest_positions: Vec<usize>,
        playable_pitches_per_line: Vec<Vec<Pitch>>,
    }

    fn arb_case() -> impl Strategy<Value = ArrangementCase> {
        (
            prop::collection::vec(
                (
                    prop::collection::vec(any_pitch(), 1..=3),
                    any::<u8>(), // kind selector
                ),
                1..=6,
            ),
            1u8..=5u8,
        )
            .prop_map(|(line_specs, num_arrangements)| {
                let mut input_lines: Vec<Line<BeatVec<Pitch>>> =
                    Vec::with_capacity(line_specs.len());
                let mut measure_break_positions = Vec::new();
                let mut rest_positions = Vec::new();
                let mut playable_pitches_per_line = Vec::new();

                for (idx, (pitches, kind_byte)) in line_specs.into_iter().enumerate() {
                    match kind_byte % 8 {
                        0 => {
                            input_lines.push(Line::Rest);
                            rest_positions.push(idx);
                            playable_pitches_per_line.push(Vec::new());
                        }
                        1 => {
                            input_lines.push(Line::MeasureBreak);
                            measure_break_positions.push(idx);
                            playable_pitches_per_line.push(Vec::new());
                        }
                        _ => {
                            playable_pitches_per_line.push(pitches.clone());
                            input_lines.push(Line::Playable(pitches));
                        }
                    }
                }

                // Ensure at least one playable line so create_arrangements doesn't short-circuit.
                if !input_lines.iter().any(|l| matches!(l, Line::Playable(_))) {
                    input_lines[0] = Line::Playable(vec![Pitch::E4]);
                    if let Some(pos) = rest_positions.iter().position(|&p| p == 0) {
                        rest_positions.remove(pos);
                    }
                    if let Some(pos) = measure_break_positions.iter().position(|&p| p == 0) {
                        measure_break_positions.remove(pos);
                    }
                    playable_pitches_per_line[0] = vec![Pitch::E4];
                }

                ArrangementCase {
                    input_lines,
                    num_arrangements: NumArrangements::try_new(num_arrangements)
                        .expect("BUG: strategy generates 1..=5"),
                    measure_break_positions,
                    rest_positions,
                    playable_pitches_per_line,
                }
            })
    }

    fn std_guitar() -> Guitar {
        let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES)
            .expect("BUG: standard tuning is always valid");
        Guitar::new(tuning, 18, 0).expect("BUG: std guitar is always valid")
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        // Invariant 1: every input pitch is represented in each arrangement at the matching
        // line_index via one of its candidate fingerings.
        #[test]
        fn invariant_input_pitches_represented(case in arb_case()) {
            let guitar = std_guitar();
            let arrangements = create_arrangements(
                guitar.clone(), case.input_lines.clone(), case.num_arrangements, None,
            ).map_err(|e| TestCaseError::reject(format!("create_arrangements rejected input: {e}")))?;

            // Map input line_index (skipping leading non-playable lines) to expected pitches.
            let first_playable = case.input_lines
                .iter()
                .position(|l| matches!(l, Line::Playable(_)))
                .unwrap_or(0);
            let effective_lines: Vec<&Line<BeatVec<Pitch>>> = case.input_lines
                .iter()
                .skip(first_playable)
                .collect();

            for arrangement in &arrangements {
                prop_assert_eq!(arrangement.lines.len(), effective_lines.len());
                for (idx, (input_line, output_line)) in
                    effective_lines.iter().zip(arrangement.lines.iter()).enumerate()
                {
                    match (input_line, output_line) {
                        (Line::Playable(input_pitches), Line::Playable(fingerings)) => {
                            prop_assert_eq!(
                                fingerings.len(), input_pitches.len(),
                                "line {} fingering count mismatch", idx
                            );
                            let output_pitches: HashSet<Pitch> =
                                fingerings.iter().map(|f| f.pitch).collect();
                            let expected_pitches: HashSet<Pitch> =
                                input_pitches.iter().copied().collect();
                            prop_assert_eq!(
                                output_pitches, expected_pitches,
                                "line {} pitch mismatch", idx
                            );
                        }
                        (Line::Rest, Line::Rest) | (Line::MeasureBreak, Line::MeasureBreak) => {}
                        _ => prop_assert!(false, "line {} variant mismatch", idx),
                    }
                }
            }
        }

        // Invariant 2: no two fingerings in the same beat share a string_number.
        #[test]
        fn invariant_no_duplicate_strings(case in arb_case()) {
            let guitar = std_guitar();
            let arrangements = create_arrangements(
                guitar, case.input_lines, case.num_arrangements, None,
            ).map_err(|e| TestCaseError::reject(format!("create_arrangements rejected input: {e}")))?;

            for arrangement in &arrangements {
                for line in &arrangement.lines {
                    if let Line::Playable(fingerings) = line {
                        let mut seen = HashSet::new();
                        for f in fingerings {
                            prop_assert!(
                                seen.insert(f.string_number),
                                "duplicate string_number in beat"
                            );
                        }
                    }
                }
            }
        }

        // Invariant 3: every fret is in [0, num_frets].
        #[test]
        fn invariant_fret_bounds(case in arb_case()) {
            let guitar = std_guitar();
            let playable_frets = guitar.playable_frets;
            let arrangements = create_arrangements(
                guitar, case.input_lines, case.num_arrangements, None,
            ).map_err(|e| TestCaseError::reject(format!("create_arrangements rejected input: {e}")))?;

            for arrangement in &arrangements {
                for line in &arrangement.lines {
                    if let Line::Playable(fingerings) = line {
                        for f in fingerings {
                            prop_assert!(f.fret <= playable_frets);
                        }
                    }
                }
            }
        }

        // Invariant 4: arrangements are sorted by ascending difficulty.
        #[test]
        fn invariant_sorted_by_difficulty(case in arb_case()) {
            let guitar = std_guitar();
            let arrangements = create_arrangements(
                guitar, case.input_lines, case.num_arrangements, None,
            ).map_err(|e| TestCaseError::reject(format!("create_arrangements rejected input: {e}")))?;

            for pair in arrangements.windows(2) {
                prop_assert!(pair[0].difficulty <= pair[1].difficulty);
            }
        }

        // Invariant 5: the number of arrangements returned is at most the requested max.
        #[test]
        fn invariant_count_bounded(case in arb_case()) {
            let guitar = std_guitar();
            let arrangements = create_arrangements(
                guitar, case.input_lines, case.num_arrangements, None,
            ).map_err(|e| TestCaseError::reject(format!("create_arrangements rejected input: {e}")))?;

            prop_assert!(arrangements.len() <= case.num_arrangements.get() as usize);
        }

        // Invariant 6: deterministic. Same input produces the same output twice.
        // Both calls go through the uncached `memoized_original_create_arrangements`. The
        // memoized `create_arrangements` would serve the second call from its cache and never
        // re-run the pathfinder, which would make the comparison trivially true.
        #[test]
        fn invariant_deterministic(case in arb_case()) {
            let guitar1 = std_guitar();
            let guitar2 = std_guitar();
            let first = memoized_original_create_arrangements(
                guitar1, case.input_lines.clone(), case.num_arrangements, None,
            );
            let second = memoized_original_create_arrangements(
                guitar2, case.input_lines, case.num_arrangements, None,
            );
            match (first, second) {
                (Ok(a), Ok(b)) => prop_assert_eq!(a, b),
                (Err(a), Err(b)) => prop_assert_eq!(a, b),
                _ => prop_assert!(false, "determinism violated: outcomes differ"),
            }
        }
    }
}
