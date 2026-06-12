#![allow(unused)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use guitar_tab_generator::{
    BeatVec, DifficultyWeights, Guitar, Line, NumArrangements, Pitch, StringNumber, TabInput,
    create_arrangements, create_string_tuning, generate_arrangements, parse_lines, render_tab,
};
// `__bench_internals` exposes the memoize escape hatches plus the now-private
// tuning offset helpers so criterion benches can still measure them; see the
// module docstring in src/lib.rs for the stability caveat.
use guitar_tab_generator::__bench_internals::{
    create_string_tuning_offset, memoized_original_create_arrangements,
    memoized_original_parse_lines, parse_tuning,
};

fn std_tuning() -> [Pitch; 6] {
    [
        Pitch::E4,
        Pitch::B3,
        Pitch::G3,
        Pitch::D3,
        Pitch::A2,
        Pitch::E2,
    ]
}
use itertools::Itertools;
use std::{collections::BTreeMap, hint::black_box, time::Duration};

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
fn fur_elise_lines() -> Vec<Line<BeatVec<Pitch>>> {
    parse_lines(fur_elise_input().to_owned()).unwrap()
}

fn bench_parse_lines(c: &mut Criterion) {
    let fur_elise_input = fur_elise_input();

    c.bench_function("parse_lines", |b| {
        b.iter(|| memoized_original_parse_lines(black_box(fur_elise_input).to_owned()))
    });
}

fn bench_create_string_tuning_offset(c: &mut Criterion) {
    c.bench_function("create_string_tuning_offset", |b| {
        b.iter(|| {
            create_string_tuning_offset(
                parse_tuning(black_box("openG")).expect("valid tuning name"),
            )
        })
    });
}

fn guitar_creation(c: &mut Criterion) {
    let six_string_tuning = create_string_tuning(&std_tuning()).unwrap();

    let three_string_tuning = create_string_tuning(&[Pitch::E4, Pitch::B3, Pitch::G3]).unwrap();
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
    ])
    .unwrap();

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

fn bench_arrangement_creation(c: &mut Criterion) {
    let tuning = create_string_tuning(&std_tuning()).unwrap();

    c.bench_function("fur_elise_1_arrangement", |b| {
        b.iter(|| {
            memoized_original_create_arrangements(
                black_box(Guitar::new(tuning.clone(), 18, 0).unwrap()),
                black_box(fur_elise_lines()),
                black_box(NumArrangements::try_new(1).unwrap()),
                black_box(DifficultyWeights::standard()),
                black_box(None),
            )
        })
    });
    c.bench_function("fur_elise_3_arrangements", |b| {
        b.iter(|| {
            memoized_original_create_arrangements(
                black_box(Guitar::new(tuning.clone(), 18, 0).unwrap()),
                black_box(fur_elise_lines()),
                black_box(NumArrangements::try_new(3).unwrap()),
                black_box(DifficultyWeights::standard()),
                black_box(None),
            )
        })
    });
    c.bench_function("fur_elise_5_arrangements", |b| {
        b.iter(|| {
            memoized_original_create_arrangements(
                black_box(Guitar::new(tuning.clone(), 18, 0).unwrap()),
                black_box(fur_elise_lines()),
                black_box(NumArrangements::try_new(5).unwrap()),
                black_box(DifficultyWeights::standard()),
                black_box(None),
            )
        })
    });
}

fn bench_arrangement_scaling(c: &mut Criterion) {
    let tuning = create_string_tuning(&std_tuning()).unwrap();

    let mut group = c.benchmark_group("bench_arrangement_scaling");
    // 1..=MAX: NumArrangements rejects 0 and >MAX at construction, so the validation-error
    // timings the prior 0..=22 loop produced are unreachable.
    for num in 1u8..=NumArrangements::MAX {
        group
            .sample_size(15)
            .warm_up_time(Duration::from_secs_f32(2.0));
        let num_arrangements = NumArrangements::try_new(num).unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(num), &num, |b, _| {
            b.iter(|| {
                memoized_original_create_arrangements(
                    black_box(Guitar::new(tuning.clone(), 18, 0).unwrap()),
                    black_box(fur_elise_lines()),
                    black_box(num_arrangements),
                    black_box(DifficultyWeights::standard()),
                    black_box(None),
                )
            });
        });
    }
    group.finish();
}

fn bench_render_tab(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_tab");

    let arrangements = create_arrangements(
        Guitar::default(),
        parse_lines(fur_elise_input().to_owned()).unwrap(),
        NumArrangements::try_new(1).unwrap(),
        DifficultyWeights::standard(),
        None,
    )
    .unwrap();

    for playback_index in (0..=30).step_by(10) {
        group.bench_with_input(
            BenchmarkId::from_parameter(playback_index),
            &playback_index,
            |b, &playback_index| {
                b.iter(|| {
                    black_box(render_tab(
                        black_box(arrangements[0].lines()),
                        black_box(&Guitar::default()),
                        black_box(20),
                        black_box(2),
                        black_box(Some(playback_index)),
                    ));
                });
            },
        );
    }
    group.finish();
}

fn bench_create_single_composition_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_create_single_composition_scaling");
    for input_lines_num in (5..=85).step_by(10) {
        let input = guitar_tab_generator::TabInput::new(
            fur_elise_input().lines().take(input_lines_num).join("\n"),
            "standard",
            18,
            0,
            1,
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(input_lines_num),
            &input_lines_num,
            |b, _| {
                b.iter(|| {
                    let set = guitar_tab_generator::generate_arrangements(black_box(input.clone()));
                    if let Ok(set) = set {
                        let _ = black_box(set.render(0, 40, 2, Some(12)));
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_create_single_composition_large_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_create_single_composition_large_scaling");
    for fur_elise_repetitions in (1..=10).step_by(1) {
        let input = guitar_tab_generator::TabInput::new(
            fur_elise_input().repeat(fur_elise_repetitions),
            "standard",
            18,
            0,
            1,
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(fur_elise_repetitions),
            &fur_elise_repetitions,
            |b, _| {
                b.iter(|| {
                    let set = guitar_tab_generator::generate_arrangements(black_box(input.clone()));
                    if let Ok(set) = set {
                        let _ = black_box(set.render(0, 40, 2, Some(12)));
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group! {
    name=benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(15);
    targets =
        bench_parse_lines,
        bench_create_string_tuning_offset,
        guitar_creation,
        bench_arrangement_creation,
        bench_arrangement_scaling,
        bench_create_single_composition_scaling,
        bench_create_single_composition_large_scaling,
        bench_render_tab
}
criterion_main!(benches);
