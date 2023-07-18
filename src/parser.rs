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

    // let _a = Color::from_str("Green").unwrap();
    // let _a = Pitch::from_str("Bb2220");
    // dbg!(&_a);

    "Hi".to_owned()
}

fn parse_line(input_index: usize, input_line: &str) -> Result<Line<Vec<Pitch>>> {
    // println!("--------------------------------");
    if let Some(rest) = parse_rest(input_line) {
        return Ok(rest);
    }
    if let Some(measure_break) = parse_measure_break(input_line) {
        return Ok(measure_break);
    }
    parse_pitch(input_index, input_line)
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

fn consecutive_slices(data: &[usize]) -> Vec<&[usize]> {
    let mut slice_start = 0;
    let mut result = Vec::new();
    for i in 1..data.len() {
        if data[i - 1] + 1 != data[i] {
            result.push(&data[slice_start..i]);
            slice_start = i;
        }
    }
    if !data.is_empty() {
        result.push(&data[slice_start..]);
    }
    result
}
