#![allow(unused)]

use anyhow::{anyhow, Result};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use guitar_tab_generator::{
    arrangement::{create_arrangements, BeatVec, Line},
    guitar::{create_string_tuning, Guitar, STD_6_STRING_TUNING_OPEN_PITCHES},
    parser::parse_lines,
    pitch::Pitch,
    renderer::render_tab,
    string_number::StringNumber,
};
use itertools::Itertools;
use std::{collections::BTreeMap, time::Duration};

pub fn guitar_creation(c: &mut Criterion) {
    let six_string_tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES);

    let three_string_tuning = create_string_tuning(&[Pitch::E4, Pitch::B3, Pitch::G3]);
    let twelve_string_tuning = create_string_tuning(&[
        Pitch::E4,
        Pitch::B3,
        Pitch::G3,
        Pitch::D3,
        Pitch::A2,
        Pitch::E2,
        Pitch::E2,
        Pitch::E2,
        Pitch::E2,
        Pitch::E2,
        Pitch::E2,
        Pitch::E2,
    ]);

    const STANDARD_NUM_FRETS: u8 = 18;

    c.bench_function("create_standard_guitar", |b| {
        b.iter(|| {
            Guitar::new(
                black_box(six_string_tuning.clone()),
                black_box(STANDARD_NUM_FRETS),
                black_box(0),
            )
        })
    });
    c.bench_function("create_few_fret_guitar", |b| {
        b.iter(|| {
            Guitar::new(
                black_box(six_string_tuning.clone()),
                black_box(3),
                black_box(0),
            )
        })
    });
    c.bench_function("create_few_string_guitar", |b| {
        b.iter(|| {
            Guitar::new(
                black_box(three_string_tuning.clone()),
                black_box(STANDARD_NUM_FRETS),
                black_box(0),
            )
        })
    });
}

fn fur_elise_input() -> &'static str {
    "E4
    Eb4
    E4
    Eb4
    E4
    B3
    D4
    C4

    A2A3
    E3
    A3
    C3
    E3
    A3

    E3B3
    E3
    Ab3
    E3
    Ab3
    B3

    A2C4
    E3
    A3
    E3

    E4
    Eb4
    E4
    Eb4
    E4
    B3
    D4
    C4

    A2A3
    E3
    A3
    C3
    E3
    A3

    E3B3
    E3
    Ab3
    E3
    C4
    B3
    A3

    C4
    C4
    C4
    C4
    F4
    E4
    E4
    D4

    Bb4
    A4
    A4
    G4
    F4
    E4
    D4
    C4

    Bb3
    Bb3
    A3
    G3
    A3
    Bb3
    C4

    D4
    Eb4
    Eb4
    E4
    F4
    A3
    C4

    D4
    B3
    C4"
}

fn fur_elise_lines() -> Result<Vec<Line<BeatVec<Pitch>>>> {
    guitar_tab_generator::parser::parse_lines(fur_elise_input().to_owned())
}

fn bench_arrangement_creation(c: &mut Criterion) {
    let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES);

    // c.bench_function("fur_elise_1_arrangement", |b| {
    //     b.iter(|| {
    //         create_arrangements(
    //             black_box(Guitar::new(tuning.clone(), 18, 0).unwrap()),
    //             black_box(fur_elise_lines().unwrap()),
    //             black_box(1),
    //         )
    //     })
    // });
    // c.bench_function("fur_elise_3_arrangements", |b| {
    //     b.iter(|| {
    //         create_arrangements(
    //             black_box(Guitar::new(tuning.clone(), 18, 0).unwrap()),
    //             black_box(fur_elise_lines().unwrap()),
    //             black_box(3),
    //         )
    //     })
    // });
    c.bench_function("fur_elise_5_arrangements", |b| {
        b.iter(|| {
            create_arrangements(
                black_box(Guitar::new(tuning.clone(), 18, 0).unwrap()),
                black_box(fur_elise_lines().unwrap()),
                black_box(5),
            )
        })
    });
}

fn bench_arrangement_scaling(c: &mut Criterion) {
    let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES);

    let mut group = c.benchmark_group("arrangement_scaling");
    for num in (0..=22) {
        // group.throughput(Throughput::Bytes(*size as u64));
        group
            .sample_size(15)
            .warm_up_time(Duration::from_secs_f32(2.0));
        group.bench_with_input(BenchmarkId::from_parameter(num), &num, |b, &num| {
            b.iter(|| {
                create_arrangements(
                    black_box(Guitar::new(tuning.clone(), 18, 0).unwrap()),
                    black_box(fur_elise_lines().unwrap()),
                    black_box(num),
                )
            });
        });
    }
    group.finish();
}

fn bench_parse_lines(c: &mut Criterion) {
    let fur_elise_input = fur_elise_input();

    c.bench_function("parse_lines", |b| {
        b.iter(|| guitar_tab_generator::parser::parse_lines(fur_elise_input.to_owned()))
    });
}

fn bench_create_string_tuning_offset(c: &mut Criterion) {
    c.bench_function("create_string_tuning_offset", |b| {
        b.iter(|| {
            guitar_tab_generator::parser::create_string_tuning_offset(
                guitar_tab_generator::parser::parse_tuning(black_box("random")),
            )
        })
    });
}

fn bench_create_single_composition_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_create_single_composition_scaling");
    for input_lines_num in (5..=85).step_by(10) {
        let input_text = fur_elise_input().lines().take(input_lines_num).join("\n");
        // dbg!(&input_text);

        // group
        //     .sample_size(20)
        //     .warm_up_time(Duration::from_secs_f32(3.0));
        group.bench_with_input(
            BenchmarkId::from_parameter(input_lines_num),
            &input_lines_num,
            |b, &playback_index| {
                b.iter(|| {
                    guitar_tab_generator::create_guitar_compositions(
                        black_box(input_text.clone()),
                        black_box("standard"),
                        black_box(18),
                        black_box(0),
                        black_box(1),
                        black_box(40),
                        black_box(1),
                        black_box(None),
                    );
                });
            },
        );
    }
    group.finish();
}

fn bench_render_tab(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_tab");

    let arrangements = create_arrangements(
        Guitar::default(),
        parse_lines(fur_elise_input().to_owned()).unwrap(),
        1,
    )
    .unwrap();

    for playback_index in (0..=30).step_by(10) {
        // group
        //     .sample_size(20)
        //     .warm_up_time(Duration::from_secs_f32(3.0));
        group.bench_with_input(
            BenchmarkId::from_parameter(playback_index),
            &playback_index,
            |b, &playback_index| {
                b.iter(|| {
                    render_tab(
                        black_box(&arrangements[0].lines),
                        black_box(&Guitar::default()),
                        black_box(20),
                        black_box(2),
                        black_box(Some(playback_index)),
                    );
                });
            },
        );
    }
    group.finish();
}

criterion_group! {
    name=benches;
    config = Criterion::default().noise_threshold(0.02).sample_size(25);
    targets =
        // guitar_creation,
        // arrangement_creation,
        // arrangement_scaling,
        // parse_lines,
        // create_string_tuning_offset,
        bench_create_single_composition_scaling,
        bench_render_tab
}
criterion_main!(benches);
