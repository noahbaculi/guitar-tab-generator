# Rayon Parallelism Experiment Report

Benchmarks run on: Apple M-series (darwin 25.3.0)

## Benchmark Inputs

- **Fur Elise excerpt**: ~85 lines, single-pitch notes and occasional 2-note chords
- **Scaling**: `bench_create_single_composition_scaling` varies input from 5–85 lines

## Results

| Benchmark | Baseline | Site A (path_node_groups) | Site B (validate_fingerings) | Site C (process_path + render_tab) |
|-----------|----------|--------------------------|------------------------------|-------------------------------------|
| `fur_elise_1_arrangement` | 187.13 µs | 281.97 µs (+50%) | — | — |
| `fur_elise_3_arrangements` | 19.806 ms | 20.683 ms (+4.8%, noise) | — | — |
| `fur_elise_5_arrangements` | 19.962 ms | 19.914 ms (no change) | — | — |
| `bench_create_single_composition_scaling/5` | 10.016 µs | 10.296 µs (+3%, noise) | — | — |
| `bench_create_single_composition_scaling/45` | 34.184 µs | 35.290 µs (+3%, noise) | — | — |
| `bench_create_single_composition_scaling/85` | 58.631 µs | 59.349 µs (+1.7%, noise) | — | — |

## Analysis

### Site A: `path_node_groups` construction (`arrangement.rs:306-325`)
**Result: Net regression — Site A should be reverted.**

The `fur_elise_1_arrangement` benchmark regressed by **+50%** (187 µs → 282 µs). The Rayon thread pool spin-up and work distribution overhead completely dominates the actual computation for this workload. Each beat's `generate_fingering_combos` call is too lightweight to amortize the parallelism cost.

For the ms-range benchmarks (3+ arrangements), no statistically significant improvement was detected. The bottleneck for those benchmarks is the `yen()` pathfinding step, which is inherently sequential — parallelizing `path_node_groups` construction has no effect on the dominant cost.

### Site B: `validate_fingerings` (`arrangement.rs:531-559`)
_TBD after benchmarking_

### Site C: `process_path` + `render_tab` (`arrangement.rs:349`, `lib.rs:94`)
_TBD after benchmarking_

## Conclusion

_TBD — which sites to keep, revert, or explore further_
