use crate::{arrangement::Line, pitch::Pitch};
use anyhow::{anyhow, Result};
use itertools::Itertools;
use regex::Regex;
use std::{collections::HashSet, str::FromStr};
use strum_macros::EnumString;

#[derive(EnumString, Debug, PartialEq)]
enum Color {
    Red,
    Green { range: usize },
    Blue(usize),
    Yellow,
}

pub fn parse_arrangements(input: String) -> String {
    let _x = input
        .lines()
        .enumerate()
        .map(|(input_index, input_line)| parse_line(input_index, input_line))
        .collect_vec();
    dbg!(&_x);

    "Hi".to_owned()
}

fn parse_line(input_index: usize, mut input_line: &str) -> Result<Line<Vec<Pitch>>> {
    // println!("--------------------------------");
    input_line = remove_comments(input_line);

    if let Some(rest) = parse_rest(input_line) {
        return Ok(rest);
    }
    if let Some(measure_break) = parse_measure_break(input_line) {
        return Ok(measure_break);
    }
    parse_pitch(input_index, input_line)
}

fn remove_comments(input_line: &str) -> &str {
    input_line.split("//").next().unwrap_or(input_line)
}

fn parse_rest(input_line: &str) -> Option<Line<Vec<Pitch>>> {
    if input_line.is_empty() {
        return Some(Line::Rest);
    }
    None
}

fn parse_measure_break(input_line: &str) -> Option<Line<Vec<Pitch>>> {
    let unique_chars: HashSet<char> = input_line.chars().collect();
    if unique_chars == HashSet::<char>::from(['-'])
        || unique_chars == HashSet::<char>::from(['–'])
        || unique_chars == HashSet::<char>::from(['—'])
    {
        return Some(Line::MeasureBreak);
    }
    None
}

fn parse_pitch(input_index: usize, input_line: &str) -> Result<Line<Vec<Pitch>>> {
    let pattern = r"(?P<three_char_pitch>[a-gA-G][#|♯|b|♭][0-9])|(?P<two_char_pitch>[a-gA-G][0-9])";
    let re = Regex::new(pattern).unwrap();
    let (each_matched_indices, matched_pitches): (Vec<Vec<usize>>, Vec<Pitch>) = re
        .find_iter(input_line)
        .filter_map(|regex_match| {
            if let Ok(pitch) = Pitch::from_str(regex_match.as_str()) {
                return Some(((regex_match.start()..regex_match.end()).collect(), pitch));
            }
            None
        })
        .unzip();

    let matched_indices: HashSet<usize> = each_matched_indices.into_iter().flatten().collect();
    let input_indices: HashSet<usize> = (0..input_line.len()).collect();

    let unmatched_indices: Vec<usize> = input_indices
        .difference(&matched_indices)
        .sorted()
        .cloned()
        .collect();

    if !unmatched_indices.is_empty() {
        let line_number = input_index + 1;
        let consecutive_indices = consecutive_slices(&unmatched_indices);
        let error_msg = consecutive_indices
            .into_iter()
            .sorted()
            .map(|unmatched_input_indices| {
                let first_idx = *unmatched_input_indices.first().unwrap();
                let last_idx = *unmatched_input_indices.last().unwrap();
                format!(
                    "Input '{}' on line {} could not be parsed into a pitch.",
                    &input_line[first_idx..=last_idx],
                    line_number
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        return Err(anyhow!(error_msg));
    }

    Ok(Line::Playable(matched_pitches))
}

/// Returns a vector of consecutive slices of the input numbers.
///
/// This function does not sort the input vector and the consecutive slices are grouped together based
/// on the order of the input numbers as received.
/// Each returned slice is a reference to a subarray of `usize` elements from the original data array.
fn consecutive_slices(numbers: &[usize]) -> Vec<&[usize]> {
    let mut slice_start = 0;
    let mut result = Vec::new();
    for i in 1..numbers.len() {
        if numbers[i - 1] + 1 != numbers[i] {
            result.push(&numbers[slice_start..i]);
            slice_start = i;
        }
    }
    if !numbers.is_empty() {
        result.push(&numbers[slice_start..]);
    }
    result
}
#[cfg(test)]
mod test_consecutive_slices {
    use super::*;

    #[test]
    fn simple() {
        let flat_nums = vec![1, 2, 3, 4];
        let consecutive_nums = vec![vec![1, 2, 3, 4]];

        assert_eq!(consecutive_slices(&flat_nums), consecutive_nums);
    }
    #[test]
    fn complex() {
        let flat_nums = vec![1, 2, 3, 4, 113, 115, 116, 6, 7, 8];
        let consecutive_nums = vec![vec![1, 2, 3, 4], vec![113], vec![115, 116], vec![6, 7, 8]];

        assert_eq!(consecutive_slices(&flat_nums), consecutive_nums);
    }
    #[test]
    fn no_consecutive() {
        let flat_nums = vec![95, 65, 74, 96, 68, 29, 34, 32];
        let consecutive_nums = vec![
            vec![95],
            vec![65],
            vec![74],
            vec![96],
            vec![68],
            vec![29],
            vec![34],
            vec![32],
        ];

        assert_eq!(consecutive_slices(&flat_nums), consecutive_nums);
    }
}
