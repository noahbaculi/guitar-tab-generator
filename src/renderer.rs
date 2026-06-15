use crate::{
    arrangement::{BeatVec, Line},
    guitar::{Guitar, PitchFingering},
};
use itertools::Itertools;
use std::collections::VecDeque;
use std::fmt::Write;

/// Widest fret column the renderer lays down (two-digit frets such as `12`).
///
/// Assumes `Guitar::MAX_NUM_FRETS` keeps fret renders to two digits. A three-digit fret
/// would break the `min_render_width` slack below.
pub(crate) const MAX_FRET_RENDER_WIDTH: usize = 2;

/// Holds the two-digit assumption above to a compile-time check: a `MAX_NUM_FRETS` of 100 or
/// more would render three-digit frets and silently misalign the `min_render_width` slack.
const _: () = assert!(Guitar::MAX_NUM_FRETS < 100);

/// Minimum `width` [`render_tab`] needs to lay out one beat at the given `padding`.
///
/// A row carries a `padding`-wide dash margin on each side plus `MAX_FRET_RENDER_WIDTH`
/// for the fret, plus one column of slack so the wrap loop admits exactly one beat at the
/// minimum (`2 * padding + MAX_FRET_RENDER_WIDTH + 1`). `ArrangementSet::render` rejects
/// smaller widths with `TabError::RenderWidthTooSmall`.
pub(crate) const fn min_render_width(padding: u8) -> u16 {
    2 * padding as u16 + MAX_FRET_RENDER_WIDTH as u16 + 1
}

/// Renders an `Arrangement`'s lines as an ASCII guitar tab.
///
/// The `width` parameter controls the character width of each row group (rows wrap to a
/// new group when they reach `width`), and `padding` controls the number of dashes between
/// beats. If `playback` is supplied, an indicator `▼`/`▲` is drawn above and below the
/// beat column corresponding to the 0-indexed beat (counting `Playable` and `Rest` lines,
/// skipping `MeasureBreak`s).
///
/// Returns an empty string if `arrangement_lines` is empty or the guitar has no strings.
#[must_use]
pub fn render_tab(
    arrangement_lines: &[Line<BeatVec<PitchFingering>>],
    guitar: &Guitar,
    width: u16,
    padding: u8,
    playback: Option<u16>,
) -> String {
    if arrangement_lines.is_empty() {
        return String::new();
    }
    let num_strings = guitar.string_ranges.len();
    if num_strings == 0 {
        return String::new();
    }

    let line_index_of_playback: Option<usize> = match playback {
        None => None,
        Some(playback_beat_index) => {
            line_index_of_beat_index(arrangement_lines, playback_beat_index as usize)
        }
    };

    let columns = arrangement_lines
        .iter()
        .map(|line| render_line(line, num_strings))
        .collect_vec();

    let beat_column_renders = transpose(columns);

    let (rows_by_string, playback_indicator_position) =
        render_string_groups(beat_column_renders, width, padding, line_index_of_playback);

    render_string_output(&rows_by_string, playback_indicator_position)
}
#[cfg(test)]
mod test_render_tab {
    use crate::{pitch::Pitch, string_number::StringNumber};

    use super::*;

    fn get_arrangement_lines() -> Vec<Line<BeatVec<PitchFingering>>> {
        vec![
            Line::Playable(vec![PitchFingering {
                pitch: Pitch::E4,
                string_number: StringNumber::new(1).unwrap(),
                fret: 0,
            }]),
            Line::Playable(vec![PitchFingering {
                pitch: Pitch::DSharpEFlat4,
                string_number: StringNumber::new(2).unwrap(),
                fret: 4,
            }]),
            Line::Playable(vec![PitchFingering {
                pitch: Pitch::E4,
                string_number: StringNumber::new(1).unwrap(),
                fret: 0,
            }]),
            Line::Rest,
            Line::MeasureBreak,
            Line::Playable(vec![PitchFingering {
                pitch: Pitch::DSharpEFlat4,
                string_number: StringNumber::new(1).unwrap(),
                fret: 4,
            }]),
            Line::Playable(vec![PitchFingering {
                pitch: Pitch::A5,
                string_number: StringNumber::new(1).unwrap(),
                fret: 12,
            }]),
        ]
    }

    #[test]
    fn single_row_group() {
        let arrangement_lines = get_arrangement_lines();
        let width = 20;
        let padding = 1;
        let playback = Some(3);

        let output = render_tab(
            &arrangement_lines,
            &Guitar::default(),
            width,
            padding,
            playback,
        );

        let expected_output = concat!(
            "       ▼\n",
            "-0---0---|-4-12-----\n",
            "---4-----|----------\n",
            "---------|----------\n",
            "---------|----------\n",
            "---------|----------\n",
            "---------|----------\n",
            "       ▲\n"
        )
        .to_owned();

        println!("Output :\n{output}");
        println!("expected output :\n{expected_output}");

        assert_eq!(output, expected_output);
    }
    #[test]
    fn two_row_groups_no_playback() {
        let arrangement_lines = get_arrangement_lines();
        let width = 14;
        let padding = 1;
        let playback = None;

        let output = render_tab(
            &arrangement_lines,
            &Guitar::default(),
            width,
            padding,
            playback,
        );

        let expected_output = concat!(
            "\n",
            "-0---0---|----\n",
            "---4-----|----\n",
            "---------|----\n",
            "---------|----\n",
            "---------|----\n",
            "---------|----\n",
            "\n\n\n",
            "-4-12---------\n",
            "--------------\n",
            "--------------\n",
            "--------------\n",
            "--------------\n",
            "--------------\n\n"
        )
        .to_owned();

        println!("Output :\n{output}");
        println!("expected output :\n{expected_output}");

        assert_eq!(output, expected_output);
    }

    #[test]
    fn width_below_minimum_does_not_panic() {
        // Regression: a width smaller than `min_render_width(padding)` must neither underflow
        // the column math nor stall the wrap loop. `ArrangementSet::render` rejects such widths
        // up front. `render_tab` itself stays total via the saturating floor plus the
        // one-beat-per-row progress floor (`content_cap`).
        let arrangement_lines = get_arrangement_lines();
        let output = render_tab(&arrangement_lines, &Guitar::default(), 1, 0, None);
        assert!(!output.is_empty());
    }

    #[test]
    fn zero_string_guitar_does_not_panic() {
        // A guitar with no strings is constructible via the low-level API. Rendering any
        // non-empty line sequence against it must return an empty string, not panic in
        // `render_string_output`.
        let no_pitches: [Pitch; 0] = [];
        let guitar = Guitar::new(
            crate::guitar::create_string_tuning(&no_pitches).unwrap(),
            0,
            0,
        )
        .unwrap();
        let lines = vec![Line::Rest, Line::MeasureBreak];
        assert_eq!(render_tab(&lines, &guitar, 20, 1, None), "");
    }

    #[test]
    fn mismatched_guitar_string_count_does_not_panic() {
        // An arrangement built on a wider guitar, rendered against a narrower one, must drop the
        // out-of-range fingerings instead of panicking on the per-string slice index.
        // `get_arrangement_lines` places notes on strings 1 and 2. A 1-string render guitar keeps
        // string 1 and skips string 2.
        let arrangement_lines = get_arrangement_lines();
        let one_string = Guitar::new(
            crate::guitar::create_string_tuning(&[Pitch::E4]).unwrap(),
            12,
            0,
        )
        .unwrap();
        let output = render_tab(&arrangement_lines, &one_string, 20, 1, None);
        // The render guitar has one string, so string 2's fingering is out of range and dropped:
        // exactly one string row is laid out, carrying string 1's frets (0, _, 0, |, 4, 12).
        // `playback` is None, so the indicator rows are blank. Filtering blanks leaves the string
        // rows only. A regression that kept string 2 would yield two rows and fail this.
        let string_rows: Vec<&str> = output
            .lines()
            .filter(|row| !row.trim().is_empty())
            .collect();
        assert_eq!(string_rows, vec!["-0---0---|-4-12-----"]);
    }
}

fn line_index_of_beat_index(
    lines: &[Line<BeatVec<PitchFingering>>],
    playback_beat_index: usize,
) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| matches!(line, Line::Playable(_) | Line::Rest))
        .map(|(index, _)| index)
        .nth(playback_beat_index)
}
#[cfg(test)]
mod test_line_index_of_beat_index {
    use super::*;
    use crate::{pitch::Pitch, string_number::StringNumber};

    #[test]
    fn empty_lines() {
        let lines: Vec<Line<BeatVec<PitchFingering>>> = vec![];
        assert_eq!(line_index_of_beat_index(&lines, 12), None);
    }
    #[test]
    fn only_measure_breaks() {
        let lines: Vec<Line<BeatVec<PitchFingering>>> =
            vec![Line::MeasureBreak, Line::MeasureBreak, Line::MeasureBreak];
        assert_eq!(line_index_of_beat_index(&lines, 12), None);
    }

    fn get_lines() -> Vec<Line<BeatVec<PitchFingering>>> {
        vec![
            Line::Playable(vec![PitchFingering {
                string_number: StringNumber::new(1).unwrap(),
                fret: 6,
                pitch: Pitch::E4,
            }]),
            Line::Playable(vec![PitchFingering {
                string_number: StringNumber::new(1).unwrap(),
                fret: 6,
                pitch: Pitch::E4,
            }]),
            Line::Rest,
            Line::Playable(vec![PitchFingering {
                string_number: StringNumber::new(1).unwrap(),
                fret: 6,
                pitch: Pitch::E4,
            }]),
            Line::MeasureBreak,
            Line::Playable(vec![PitchFingering {
                string_number: StringNumber::new(1).unwrap(),
                fret: 6,
                pitch: Pitch::E4,
            }]),
            Line::Playable(vec![PitchFingering {
                string_number: StringNumber::new(1).unwrap(),
                fret: 6,
                pitch: Pitch::E4,
            }]),
            Line::Playable(vec![PitchFingering {
                string_number: StringNumber::new(1).unwrap(),
                fret: 6,
                pitch: Pitch::E4,
            }]),
        ]
    }
    #[test]
    fn include_playable() {
        let lines = get_lines();
        assert_eq!(line_index_of_beat_index(&lines, 2), Some(2));
    }
    #[test]
    fn include_rest() {
        let lines = get_lines();
        assert_eq!(line_index_of_beat_index(&lines, 3), Some(3));
    }
    #[test]
    fn exclude_measure_break() {
        let lines = get_lines();
        assert_eq!(line_index_of_beat_index(&lines, 4), Some(5));
    }
}

/// Renders Line as a vector of strings representing the fret positions on a guitar.
///
/// Stays total when the line and the render guitar disagree: an empty `Playable` beat renders as
/// a rest, and a fingering whose string number exceeds `num_strings` is skipped. Both require a
/// hand-built line/guitar mismatch (the parser and `create_arrangements` never produce them), but
/// `render_tab` is public, so it must not panic on them.
fn render_line(line: &Line<BeatVec<PitchFingering>>, num_strings: usize) -> Vec<String> {
    let pitch_fingerings = match line {
        Line::MeasureBreak => return vec!["|".to_owned(); num_strings],
        Line::Rest => return vec!["-".to_owned(); num_strings],
        Line::Playable(pitch_fingerings) => pitch_fingerings.iter().sorted().collect_vec(),
    };
    if pitch_fingerings.is_empty() {
        return vec!["-".to_owned(); num_strings];
    }
    let fret_width_max = calc_fret_width_max(&pitch_fingerings);

    // Instantiate vec with rest dashes for all strings with the max fret width
    let mut playable_render = vec!["-".repeat(fret_width_max); num_strings];

    // Add the rendered frets for the strings that are played. Skip a fingering whose string number
    // is beyond the render guitar: the arrangement was built on a guitar with more strings.
    for fingering in pitch_fingerings {
        let string_index = fingering.string_number.get() as usize - 1;
        if let Some(slot) = playable_render.get_mut(string_index) {
            *slot = render_fret(fingering.fret, fret_width_max);
        }
    }

    playable_render
}
#[cfg(test)]
mod test_render_line {
    use super::*;
    use crate::{pitch::Pitch, string_number::StringNumber};

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
    fn playable_string_number_beyond_render_guitar_is_skipped() {
        // String 2 is beyond a 1-string render guitar. The in-range fingering renders. The
        // out-of-range one is dropped instead of panicking on the slice index.
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
        assert_eq!(render_line(&Line::Playable(pitch_fingerings), 1), vec!["9"]);
    }
    #[test]
    fn playable_empty_beat_renders_as_rest() {
        // A hand-built empty `Playable` beat has no fingerings, so it renders as a rest row
        // rather than reaching `calc_fret_width_max`'s non-empty contract.
        assert_eq!(
            render_line(&Line::Playable(vec![]), 4),
            vec!["-".to_owned(); 4]
        );
    }
}

/// Creates a string with the fret number padded with dashes to match the maximum width.
///
/// Through the public render path `fret <= Guitar::MAX_NUM_FRETS`, which the file-level
/// `assert!(Guitar::MAX_NUM_FRETS < 100)` keeps under 100, so the three-digit branch and the
/// panic below are unreachable there. The function stays total for any `u8`, so direct callers
/// and the unit tests can still exercise wider inputs.
///
/// # Panics
///
/// Panics if the width of the fret string representation is greater than `fret_width_max`.
fn render_fret(fret: u8, fret_width_max: usize) -> String {
    let fret_width = if fret < 10 {
        1
    } else if fret < 100 {
        2
    } else {
        3
    };
    assert!(
        fret_width_max >= fret_width,
        "fret_width_max ({fret_width_max}) is less than fret_width ({fret_width})"
    );
    let filler_width = fret_width_max - fret_width;
    let mut out = String::with_capacity(fret_width_max);
    for _ in 0..filler_width {
        out.push('-');
    }
    write!(&mut out, "{fret}").expect("BUG: writing to a String cannot fail");
    out
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

/// Calculates the maximum width of the string representations of fret numbers in a given array of pitch fingerings.
fn calc_fret_width_max(pitch_fingerings: &[&PitchFingering]) -> usize {
    pitch_fingerings
        .iter()
        .map(|fingering| {
            let fret = fingering.fret;
            if fret < 10 {
                1
            } else if fret < 100 {
                2
            } else {
                3
            }
        })
        .max()
        .expect("BUG: Playable line pitch fingerings should not be empty")
}
#[cfg(test)]
mod test_calc_fret_width_max {
    use crate::{pitch::Pitch, string_number::StringNumber};

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
    assert!(
        !v.is_empty(),
        "BUG: transpose called with empty input -- caller should filter empty arrangements"
    );
    let len = v[0].len();
    let mut iters: Vec<_> = v.into_iter().map(|n| n.into_iter()).collect();
    (0..len)
        .map(|_| {
            iters
                .iter_mut()
                .map(|n| {
                    n.next()
                        .expect("BUG: all inner vecs must have equal length for transpose")
                })
                .collect::<Vec<T>>()
        })
        .collect()
}
#[cfg(test)]
mod test_transpose {
    use super::*;

    #[test]
    fn transposes_2x2_matrix() {
        let input_matrix = vec![vec!["A", "B"], vec!["C", "D"]];
        let expected_output = vec![vec!["A", "C"], vec!["B", "D"]];
        assert_eq!(transpose(input_matrix), expected_output);
    }
    #[test]
    fn transposes_3x2_matrix() {
        let input_matrix = vec![vec!["A", "B"], vec!["C", "D"], vec!["E", "F"]];
        let expected_output = vec![vec!["A", "C", "E"], vec!["B", "D", "F"]];
        assert_eq!(transpose(input_matrix), expected_output);
    }
    #[test]
    fn transposes_2x3_matrix() {
        let input_matrix = vec![vec!["A", "B", "C"], vec!["D", "E", "F"]];
        let expected_output = vec![vec!["A", "D"], vec!["B", "E"], vec!["C", "F"]];
        assert_eq!(transpose(input_matrix), expected_output);
    }
    #[test]
    #[should_panic]
    fn empty_input() {
        let input_matrix: Vec<Vec<&str>> = Vec::new();
        transpose(input_matrix);
    }
}

#[derive(Debug, PartialEq)]
struct PlaybackIndicatorPosition {
    row_group_index: usize,
    column_index: usize,
}

fn render_string_groups(
    beat_column_renders: Vec<Vec<String>>,
    width: u16,
    padding: u8,
    playback_column_index: Option<usize>,
) -> (Vec<Vec<String>>, Option<PlaybackIndicatorPosition>) {
    let padding_render = "-".repeat(padding as usize);
    let content_cap = (width as usize)
        .saturating_sub(padding as usize)
        .saturating_sub(MAX_FRET_RENDER_WIDTH)
        .max(padding as usize + 1);

    let mut rows_by_string: Vec<Vec<String>> = vec![];

    let mut playback_indicator_position: Option<PlaybackIndicatorPosition> = None;

    for string_beat_columns in beat_column_renders {
        let num_render_columns = string_beat_columns.len();
        let mut remaining_string_beat_columns = VecDeque::from(string_beat_columns);
        let mut single_string_rows: Vec<String> = vec![];

        while !remaining_string_beat_columns.is_empty() {
            let mut row = String::with_capacity(width as usize);
            row.push_str(&padding_render);
            while row.len() < content_cap {
                let next_string_item = remaining_string_beat_columns.pop_front();
                match next_string_item {
                    None => {
                        break;
                    }
                    Some(string_item) => {
                        match playback_column_index {
                            Some(idx)
                                if num_render_columns - remaining_string_beat_columns.len() - 1
                                    == idx =>
                            {
                                // Offset the playback indicator by one
                                // character if the frets are two characters wide
                                let wide_fret_playback_offset = match string_item.len() {
                                    2 => 1,
                                    _ => 0,
                                };

                                playback_indicator_position = Some(PlaybackIndicatorPosition {
                                    row_group_index: single_string_rows.len(),
                                    column_index: row.len() + wide_fret_playback_offset,
                                });
                            }
                            _ => {}
                        }
                        row.push_str(&string_item)
                    }
                }

                row.push_str(&padding_render);
            }
            let remaining_characters = (width as usize).saturating_sub(row.len());
            for _ in 0..remaining_characters {
                row.push('-');
            }

            single_string_rows.push(row);
        }

        rows_by_string.push(single_string_rows);
    }

    (rows_by_string, playback_indicator_position)
}
#[cfg(test)]
mod test_render_string_groups {
    use super::*;

    fn get_beat_column_renders() -> Vec<Vec<String>> {
        vec![
            vec![
                "0".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                "--".to_owned(),
                "|".to_owned(),
                "0".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                "--".to_owned(),
                "|".to_owned(),
            ],
            vec![
                "-".to_owned(),
                "1".to_owned(),
                "-".to_owned(),
                "--".to_owned(),
                "|".to_owned(),
                "-".to_owned(),
                "1".to_owned(),
                "-".to_owned(),
                "--".to_owned(),
                "|".to_owned(),
            ],
            vec![
                "-".to_owned(),
                "-".to_owned(),
                "2".to_owned(),
                "--".to_owned(),
                "|".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                "2".to_owned(),
                "--".to_owned(),
                "|".to_owned(),
            ],
            vec![
                "-".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                "30".to_owned(),
                "|".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                "30".to_owned(),
                "|".to_owned(),
            ],
        ]
    }

    #[test]
    fn single_row_group() {
        let beat_column_renders = get_beat_column_renders();
        let width = 25;
        let padding = 1;
        let playback_column_index = Some(1);

        let expected_string_groups = vec![
            vec!["-0--------|-0--------|---".to_owned()],
            vec!["---1------|---1------|---".to_owned()],
            vec!["-----2----|-----2----|---".to_owned()],
            vec!["-------30-|-------30-|---".to_owned()],
        ];
        let expected_playback_indicator_position = Some(PlaybackIndicatorPosition {
            row_group_index: 0,
            column_index: 3,
        });

        assert_eq!(
            render_string_groups(beat_column_renders, width, padding, playback_column_index),
            (expected_string_groups, expected_playback_indicator_position)
        );
    }
    #[test]
    fn single_row_group_playback_second_char_of_wide_fret() {
        let beat_column_renders = get_beat_column_renders();
        let width = 25;
        let padding = 1;
        let playback_column_index = Some(3);

        let expected_string_groups = vec![
            vec!["-0--------|-0--------|---".to_owned()],
            vec!["---1------|---1------|---".to_owned()],
            vec!["-----2----|-----2----|---".to_owned()],
            vec!["-------30-|-------30-|---".to_owned()],
        ];
        let expected_playback_indicator_position = Some(PlaybackIndicatorPosition {
            row_group_index: 0,
            column_index: 8,
        });

        assert_eq!(
            render_string_groups(beat_column_renders, width, padding, playback_column_index),
            (expected_string_groups, expected_playback_indicator_position)
        );
    }
    #[test]
    fn two_row_group() {
        let beat_column_renders = get_beat_column_renders();
        let width = 14;
        let padding = 1;
        let playback_column_index = Some(7);

        let expected_string_groups = vec![
            vec!["-0--------|---".to_owned(), "-0--------|---".to_owned()],
            vec!["---1------|---".to_owned(), "---1------|---".to_owned()],
            vec!["-----2----|---".to_owned(), "-----2----|---".to_owned()],
            vec!["-------30-|---".to_owned(), "-------30-|---".to_owned()],
        ];
        let expected_playback_indicator_position = Some(PlaybackIndicatorPosition {
            row_group_index: 1,
            column_index: 5,
        });

        assert_eq!(
            render_string_groups(beat_column_renders, width, padding, playback_column_index),
            (expected_string_groups, expected_playback_indicator_position)
        );
    }
    #[test]
    fn no_playback_column_index() {
        let (_, playback_indicator_position) =
            render_string_groups(get_beat_column_renders(), 20, 1, None);

        assert_eq!(playback_indicator_position, None);
    }
    #[test]
    fn too_large_playback_column_index() {
        let (_, playback_indicator_position) =
            render_string_groups(get_beat_column_renders(), 20, 1, Some(100_000));

        assert_eq!(playback_indicator_position, None);
    }
}

/// Writes one playback-indicator line into `out`, terminated by `'\n'`.
///
/// Emits `column_index` spaces followed by `symbol` when the indicator falls on
/// `row_group_index`, and nothing (just the newline) otherwise. Writing in place
/// avoids the `" ".repeat(..)` temp string the indent used to allocate.
fn push_playback_line(
    out: &mut String,
    symbol: &str,
    row_group_index: usize,
    playback_indicator_position: Option<&PlaybackIndicatorPosition>,
) {
    if let Some(pos) =
        playback_indicator_position.filter(|pos| pos.row_group_index == row_group_index)
    {
        for _ in 0..pos.column_index {
            out.push(' ');
        }
        out.push_str(symbol);
    }
    out.push('\n');
}

fn render_string_output(
    rows_by_string: &[Vec<String>],
    playback_indicator_position: Option<PlaybackIndicatorPosition>,
) -> String {
    let num_strings = rows_by_string.len();
    let first_string_rows = rows_by_string
        .first()
        .expect("BUG: every arrangement has at least one string");
    let num_row_groups = first_string_rows.len();

    // Rows are ASCII (dashes, digits, pipes), so byte length is the visible width.
    let row_width = first_string_rows.first().map_or(0, String::len);
    let mut out = String::with_capacity(num_row_groups * (num_strings + 3) * (row_width + 1));
    let pos = playback_indicator_position.as_ref();

    for row_group_index in 0..num_row_groups {
        push_playback_line(&mut out, "▼", row_group_index, pos);

        for single_string_rows in rows_by_string {
            out.push_str(
                single_string_rows
                    .get(row_group_index)
                    .map(String::as_str)
                    .expect("BUG: every string has the same row-group count"),
            );
            out.push('\n');
        }

        push_playback_line(&mut out, "▲", row_group_index, pos);
        out.push('\n'); // blank line between row groups
    }

    // Each line above was written with a trailing '\n'. join("\n") separates
    // lines without a trailing newline, so drop the final one to match.
    out.pop();
    out
}

#[cfg(test)]
mod test_render_string_output {
    use super::*;

    #[test]
    fn single_row_group() {
        let string_rows = vec![
            vec!["-0--------|-0--------|---".to_owned()],
            vec!["---1------|---1------|---".to_owned()],
            vec!["-----2----|-----2----|---".to_owned()],
            vec!["-------30-|-------30-|---".to_owned()],
        ];
        let playback_indicator_position = Some(PlaybackIndicatorPosition {
            row_group_index: 0,
            column_index: 3,
        });

        let expected_output = concat!(
            "   ▼\n",
            "-0--------|-0--------|---\n",
            "---1------|---1------|---\n",
            "-----2----|-----2----|---\n",
            "-------30-|-------30-|---\n",
            "   ▲\n"
        )
        .to_owned();

        assert_eq!(
            render_string_output(&string_rows, playback_indicator_position),
            expected_output
        );
    }
    #[test]
    fn single_row_group_no_playback() {
        let string_rows = vec![
            vec!["-0--------|-0--------|---".to_owned()],
            vec!["---1------|---1------|---".to_owned()],
            vec!["-----2----|-----2----|---".to_owned()],
            vec!["-------30-|-------30-|---".to_owned()],
        ];
        let playback_indicator_position = None;

        let output = render_string_output(&string_rows, playback_indicator_position);

        let expected_output = concat!(
            "\n",
            "-0--------|-0--------|---\n",
            "---1------|---1------|---\n",
            "-----2----|-----2----|---\n",
            "-------30-|-------30-|---\n",
            "\n"
        )
        .to_owned();

        println!("Output :\n{output}");
        println!("expected output :\n{expected_output}");

        assert_eq!(output, expected_output);
    }
    #[test]
    fn multiple_row_groups_playback_second_char_of_wide_fret() {
        let string_rows = vec![
            vec!["-0--------|---".to_owned(), "-0--------|---".to_owned()],
            vec!["---1------|---".to_owned(), "---1------|---".to_owned()],
            vec!["-----2----|---".to_owned(), "-----2----|---".to_owned()],
            vec!["-------30-|---".to_owned(), "-------30-|---".to_owned()],
        ];
        let playback_indicator_position = Some(PlaybackIndicatorPosition {
            row_group_index: 1,
            column_index: 8,
        });

        let output = render_string_output(&string_rows, playback_indicator_position);

        let expected_output = concat!(
            "\n",
            "-0--------|---\n",
            "---1------|---\n",
            "-----2----|---\n",
            "-------30-|---\n\n\n",
            "        ▼\n",
            "-0--------|---\n",
            "---1------|---\n",
            "-----2----|---\n",
            "-------30-|---\n",
            "        ▲\n",
        )
        .to_owned();

        println!("Output :\n{output}");
        println!("expected output :\n{expected_output}");

        assert_eq!(output, expected_output);
    }
    #[test]
    fn empty_row_groups_returns_empty_string() {
        // One string with zero row groups: the loop body never runs, so the
        // trailing-newline pop() must leave an empty string, matching join over
        // an empty Vec.
        let string_rows: Vec<Vec<String>> = vec![vec![]];

        assert_eq!(render_string_output(&string_rows, None), "");
    }
}
