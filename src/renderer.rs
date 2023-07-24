use crate::{
    arrangement::{Arrangement, BeatVec, Line},
    guitar::{Guitar, PitchFingering},
};
use itertools::Itertools;
use std::collections::VecDeque;

pub fn render_tab(
    arrangement: Arrangement,
    guitar: Guitar,
    width: u16,
    padding: u8,
    playback: Option<u16>,
) -> String {
    let num_strings = guitar.string_ranges.len();

    let line_index_of_playback: Option<usize> = match playback {
        None => None,
        Some(playback_sonorous_column_num) => {
            line_index_of_sonorous_index(&arrangement.lines, playback_sonorous_column_num as usize)
        }
    };
    // dbg!(&line_index_of_playback);

    // match playback_beat_num {
    //     None => None,
    //     Some(playback_beat_num) => Some(
    //         arrangement
    //             .lines
    //             .iter()
    //             .filter(|line| matches!(line, Line::Playable(_) | Line::Rest))
    //             .enumerate()
    //             .inspect(|(index, line)| {
    //                 println!("{} - {:?}", index, line);
    //             })
    //             .position(|(index, _)| index == playback_beat_num.into())
    //             .unwrap(),
    //     ),
    // };
    // dbg!(&arrangement.lines);
    // dbg!(&playback_column_index);

    let columns = arrangement
        .lines
        .iter()
        .map(|line| render_line(line, num_strings))
        .collect_vec();

    let beat_column_renders = transpose(columns);

    let (strings_rows, playback_indicator_position) =
        render_string_groups(beat_column_renders, width, padding, line_index_of_playback);

    // dbg!(&playback_indicator_position);

    render_complete(&strings_rows, playback_indicator_position)
}

fn line_index_of_sonorous_index(
    lines: &[Line<BeatVec<PitchFingering>>],
    playback_sonorous_column_num: usize,
) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| matches!(line, Line::Playable(_) | Line::Rest))
        .map(|(index, _)| index)
        .nth(playback_sonorous_column_num)
}
#[cfg(test)]
mod test_line_index_of_sonorous_index {
    use super::*;
    use crate::{pitch::Pitch, string_number::StringNumber};

    #[test]
    fn empty_lines() {
        let lines: Vec<Line<BeatVec<PitchFingering>>> = vec![];
        assert_eq!(line_index_of_sonorous_index(&lines, 12), None);
    }
    #[test]
    fn only_measure_breaks() {
        let lines: Vec<Line<BeatVec<PitchFingering>>> =
            vec![Line::MeasureBreak, Line::MeasureBreak, Line::MeasureBreak];
        assert_eq!(line_index_of_sonorous_index(&lines, 12), None);
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
        assert_eq!(line_index_of_sonorous_index(&lines, 2), Some(2));
    }
    #[test]
    fn include_rest() {
        let lines = get_lines();
        assert_eq!(line_index_of_sonorous_index(&lines, 3), Some(3));
    }
    #[test]
    fn exclude_measure_break() {
        let lines = get_lines();
        assert_eq!(line_index_of_sonorous_index(&lines, 4), Some(5));
    }
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
    assert!(
        fret_width_max >= fret_width,
        "fret_width_max ({fret_width_max}) is less than fret_width ({fret_width})"
    );
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
#[cfg(test)]
mod test_transpose {
    use super::*;

    #[test]
    fn test_transpose_2x2() {
        let input_matrix = vec![vec!["A", "B"], vec!["C", "D"]];
        let expected_output = vec![vec!["A", "C"], vec!["B", "D"]];
        assert_eq!(transpose(input_matrix), expected_output);
    }
    #[test]
    fn test_transpose_3x2() {
        let input_matrix = vec![vec!["A", "B"], vec!["C", "D"], vec!["E", "F"]];
        let expected_output = vec![vec!["A", "C", "E"], vec!["B", "D", "F"]];
        assert_eq!(transpose(input_matrix), expected_output);
    }
    #[test]
    fn test_transpose_2x3() {
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

    const MAX_FRET_RENDER_WIDTH: usize = 2;
    let mut strings_rows: Vec<Vec<String>> = vec![];

    let mut playback_indicator_position: Option<PlaybackIndicatorPosition> = None;

    for string_beat_columns in beat_column_renders {
        let num_render_columns = string_beat_columns.len();
        let mut remaining_string_beat_columns = VecDeque::from(string_beat_columns);
        let mut string_rows: Vec<String> = vec![];

        while !remaining_string_beat_columns.is_empty() {
            let mut string_row = String::with_capacity(width as usize);
            string_row.push_str(&padding_render);
            while string_row.len() < (width as usize - padding as usize - MAX_FRET_RENDER_WIDTH) {
                let next_string_item = remaining_string_beat_columns.pop_front();
                match next_string_item {
                    None => {
                        break;
                    }
                    Some(string_item) => {
                        match playback_column_index {
                            None => {}
                            Some(idx) => {
                                if num_render_columns - remaining_string_beat_columns.len() - 1
                                    == idx
                                {
                                    // Offset the playback indicator by one
                                    // character if the frets are two characters wide
                                    let wide_fret_playback_offset = match string_item.len() {
                                        2 => 1,
                                        _ => 0,
                                    };

                                    playback_indicator_position = Some(PlaybackIndicatorPosition {
                                        row_group_index: string_rows.len(),
                                        column_index: string_row.len() + wide_fret_playback_offset,
                                    });
                                }
                            }
                        }
                        string_row.push_str(&string_item)
                    }
                }

                string_row.push_str(&padding_render);
            }
            let remaining_characters = width as usize - string_row.len();
            string_row.push_str(&"-".repeat(remaining_characters));

            string_rows.push(string_row);
        }

        strings_rows.push(string_rows);
    }

    (strings_rows, playback_indicator_position)
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

fn render_complete(
    strings_rows: &[Vec<String>],
    playback_indicator_position: Option<PlaybackIndicatorPosition>,
) -> String {
    let mut output_lines: Vec<String> = vec![];

    let num_row_groups = strings_rows[0].len();

    for row_group_index in 0..num_row_groups {
        let upper_playback_row_render = match playback_indicator_position {
            None => "".to_owned(),
            Some(ref pos) => match row_group_index == pos.row_group_index {
                false => "".to_owned(),
                true => " ".repeat(pos.column_index) + "▼",
            },
        };
        output_lines.push(upper_playback_row_render);

        for string_rows in strings_rows {
            output_lines.push(
                string_rows
                    .get(row_group_index)
                    .unwrap_or(&"???".to_owned())
                    .to_string(),
            );
        }
        let lower_playback_row_render = match playback_indicator_position {
            None => "".to_owned(),
            Some(ref pos) => match row_group_index == pos.row_group_index {
                false => "".to_owned(),
                true => " ".repeat(pos.column_index) + "▲",
            },
        };

        output_lines.push(lower_playback_row_render);
        output_lines.push("".to_owned());
    }

    // println!("{}", &output_lines.join("\n"));
    output_lines.join("\n")
}
