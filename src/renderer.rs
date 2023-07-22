use crate::{
    arrangement::{Arrangement, BeatVec, Line},
    guitar::{Guitar, PitchFingering},
};
use itertools::Itertools;
use std::collections::VecDeque;

#[allow(unused_variables)]
pub fn render_tab(
    arrangement: Arrangement,
    guitar: Guitar,
    width: u16,
    padding: u8,
    playback_beat_num: Option<u16>,
) -> String {
    let num_strings = guitar.string_ranges.len();
    let columns = arrangement
        .lines
        .iter()
        .map(|line| render_line(line, num_strings))
        .collect_vec();

    let beat_column_renders = transpose(columns);

    let string_group_renders = render_string_group(beat_column_renders, width, padding);

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

fn render_string_group(
    beat_column_renders: Vec<Vec<String>>,
    width: u16,
    padding: u8,
) -> Vec<String> {
    let padding_render = "-".repeat(padding as usize);

    const MAX_FRET_RENDER_WIDTH: usize = 2;
    let mut strings_rows: Vec<Vec<String>> = vec![];

    dbg!(&beat_column_renders);

    for string_beat_columns in beat_column_renders {
        let mut remaining_string_beat_columns = VecDeque::from(string_beat_columns);
        let mut string_rows: Vec<String> = vec![];

        while !remaining_string_beat_columns.is_empty() {
            let mut string_row = String::with_capacity(width as usize);
            string_row.push_str(&padding_render);
            while string_row.len() < (width as usize - padding as usize - MAX_FRET_RENDER_WIDTH) {
                let next_string_item = remaining_string_beat_columns.pop_front();
                match next_string_item {
                    Some(string_item) => string_row.push_str(&string_item),
                    None => break,
                }
                string_row.push_str(&padding_render);
            }
            let remaining_characters = width as usize - string_row.len();
            string_row.push_str(&"-".repeat(remaining_characters));

            string_rows.push(string_row);
        }

        strings_rows.push(string_rows);

        // dbg!(&string_rows);
    }
    // dbg!(&strings_rows);

    // let num_row_groups = strings_rows[0][0].len() - 1;
    // for row_group_index in 0..num_row_groups {
    //     for string_rows in &strings_rows {
    //         println!(
    //             "{:?}",
    //             string_rows
    //                 .get(row_group_index.clone())
    //                 .unwrap_or(&"???".to_owned())
    //         );
    //     }
    //     println!("")
    // }

    vec!["Hi".to_owned()]
}
