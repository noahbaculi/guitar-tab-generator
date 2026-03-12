# Rayon Parallelism Experiment Report

Benchmarks run on: Apple M-series (darwin 25.3.0)

## Benchmark Inputs

- **Fur Elise excerpt**: ~85 lines, single-pitch notes and occasional 2-note chords
- **Scaling**: `bench_create_single_composition_scaling` varies input from 5–85 lines

## Results

| Benchmark | Baseline | Site A (path_node_groups) | Site B (validate_fingerings) | Site C (process_path + render_tab) |
|-----------|----------|--------------------------|------------------------------|-------------------------------------|
| `fur_elise_1_arrangement` | 187.13 µs | — | — | — |
| `fur_elise_3_arrangements` | 19.806 ms | — | — | — |
| `fur_elise_5_arrangements` | 19.962 ms | — | — | — |
| `bench_create_single_composition_scaling/5` | 10.016 µs | — | — | — |
| `bench_create_single_composition_scaling/45` | 34.184 µs | — | — | — |
| `bench_create_single_composition_scaling/85` | 58.631 µs | — | — | — |

## Analysis

### Site A: `path_node_groups` construction (`arrangement.rs:306-325`)
_TBD after benchmarking_

### Site B: `validate_fingerings` (`arrangement.rs:531-559`)
_TBD after benchmarking_

### Site C: `process_path` + `render_tab` (`arrangement.rs:349`, `lib.rs:94`)
_TBD after benchmarking_

## Conclusion

_TBD — which sites to keep, revert, or explore further_
