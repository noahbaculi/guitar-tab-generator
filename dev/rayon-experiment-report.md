# Rayon Parallelism Experiment Report

Benchmarks run on: Apple M-series (darwin 25.3.0)

## Benchmark Inputs

- **Fur Elise excerpt**: ~85 lines, single-pitch notes and occasional 2-note chords
- **Scaling**: `bench_create_single_composition_scaling` varies input from 5–85 lines

## Results

| Benchmark | Baseline | Site A (path_node_groups) | Site B (validate_fingerings) | Site C (process_path + render_tab) |
|-----------|----------|--------------------------|------------------------------|-------------------------------------|
| `fur_elise_1_arrangement` | 187.13 µs | 281.97 µs (+50%) | 315.08 µs (+68%) | 313.28 µs (+67%) |
| `fur_elise_3_arrangements` | 19.806 ms | 20.683 ms (+4.8%, noise) | 19.749 ms (no change) | 19.986 ms (no change) |
| `fur_elise_5_arrangements` | 19.962 ms | 19.914 ms (no change) | 19.670 ms (no change) | 20.084 ms (no change) |
| `bench_create_single_composition_scaling/5` | 10.016 µs | 10.296 µs (+3%, noise) | 10.081 µs (+0.7%, noise) | 10.371 µs (+3.5%, noise) |
| `bench_create_single_composition_scaling/45` | 34.184 µs | 35.290 µs (+3%, noise) | 33.790 µs (-1.1%, noise) | 33.996 µs (-0.5%, noise) |
| `bench_create_single_composition_scaling/85` | 58.631 µs | 59.349 µs (+1.7%, noise) | 57.569 µs (-1.8%, noise) | 57.750 µs (-1.5%, noise) |

## Analysis

### Site A: `path_node_groups` construction (`arrangement.rs:306-325`)
**Result: Net regression — Site A should be reverted.**

The `fur_elise_1_arrangement` benchmark regressed by **+50%** (187 µs → 282 µs). The Rayon thread pool spin-up and work distribution overhead completely dominates the actual computation for this workload. Each beat's `generate_fingering_combos` call is too lightweight to amortize the parallelism cost.

For the ms-range benchmarks (3+ arrangements), no statistically significant improvement was detected. The bottleneck for those benchmarks is the `yen()` pathfinding step, which is inherently sequential — parallelizing `path_node_groups` construction has no effect on the dominant cost.

### Site B: `validate_fingerings` (`arrangement.rs:531-559`)
**Result: Net regression — Site B should also be reverted.**

`fur_elise_1_arrangement` regressed to **315 µs**, 68% worse than the baseline and even slower than Site A alone (+50%). The `generate_pitch_fingerings` call per pitch is lightweight (a range lookup), so Rayon's thread dispatch overhead once again dominates.

The `bench_create_single_composition_scaling` values appear slightly better vs Site A, but those benchmarks hit the memoize cache after the first call — they measure cache-hit overhead + `render_tab`, not the computation. On any cold-path invocation (first call or cache miss), performance is significantly worse.

The refactored `process_beat` helper function is a clean extraction regardless of parallelism; the `par_iter()` usage itself is not beneficial here.

### Site C: `process_path` + `render_tab` (`arrangement.rs:349`, `lib.rs:94`)
**Result: No improvement — Site C should be reverted.**

The collection is 1–20 items. `fur_elise_1_arrangement` shows no additional regression vs Site B (313 µs vs 315 µs — within noise), confirming that parallelizing a single-item list does nothing useful. For 3 and 5 arrangements, also no statistically significant change.

This confirms the plan's prediction: overhead > gain at ≤20 items.

## Conclusion

**All three sites should be reverted.** None provided a net performance improvement; all regressed or showed no change.

### Why Rayon didn't help here

The guitar-tab-generator pipeline is **dominated by the sequential `yen()` pathfinding step**, which accounts for essentially all of the multi-arrangement benchmark time (~20 ms per run). Since `yen()` cannot be parallelized (each path depends on the previous), the pipeline has no parallel headroom at the macro level.

The three candidate sites (`path_node_groups`, `validate_fingerings`, `process_path`/`render_tab`) are all pre- or post-`yen()` and individually fast (µs-range per element). Rayon's thread pool spin-up and work distribution overhead (~50–100 µs per `par_iter()` invocation on Apple M-series) exceeds the actual computation time at these sites.

### Key findings

| Site | Overhead | Computation | Result |
|------|----------|-------------|--------|
| A (`path_node_groups`) | High | Fast (µs/beat) | +50% regression on cold path |
| B (`validate_fingerings`) | High | Very fast (range lookup) | +68% regression cumulative |
| C (`process_path` + `render_tab`) | High | Fast (1–20 items) | No change |

### When Rayon would start to pay off

Parallelism would help if:
- **Input size grows dramatically** (hundreds of beats per composition with complex chord combos)
- **WASM threading support** (`wasm-bindgen-rayon`) is added, enabling parallel execution in browser workers
- **The per-element work increases** (e.g., if chord fingering expansion becomes more computationally expensive)

At present input sizes (5–85 lines of music), the sequential implementation is optimal.

### Recommendation

- Revert all Rayon changes from `src/arrangement.rs` and `src/lib.rs`
- Keep the `process_beat` helper function extracted in Site B as a clean refactor
- Remove the Rayon dependency from `Cargo.toml`
- Track this finding in the README to avoid re-investigating the same approach
