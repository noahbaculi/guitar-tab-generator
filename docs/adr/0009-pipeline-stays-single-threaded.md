# 0009: Pipeline stays single-threaded; Rayon parallelism rejected

Status: accepted
Date: 2026-06-05

## Context

The arrangement pipeline's cost is dominated by one step: the `yen()`
k-shortest-paths search inside `create_arrangements` (`src/arrangement.rs`).
A single-arrangement run of the Fur Elise benchmark (~85 lines) is ~187 µs.
Asking for 3 or 5 arrangements jumps to ~20 ms, essentially all of it inside
`yen()`. The surrounding stages (node-group construction, fingering
validation, path processing, rendering) are individually µs-range per element.

Whether data parallelism could speed the pipeline up was an open question. A
`rayon-experimentation` branch (`origin/rayon-experimentation`, final report at
commit `acbb97d`, `dev/rayon-experiment-report.md`) added Rayon at three
candidate sites and benchmarked each against the baseline. This ADR records the
result so the question stays answered without re-running the experiment.

## Decision

Keep the pipeline single-threaded. Do not add Rayon. The experiment branch is
not merged.

Three sites were parallelized with `par_iter()` and measured on Apple M-series:

| Site | Function | Cold-path result (`fur_elise_1_arrangement`) |
|------|----------|----------------------------------------------|
| A | `path_node_groups` construction | 187 µs -> 282 µs (+50%) |
| B | `validate_fingerings` | -> 315 µs (+68% cumulative) |
| C | `process_path` + `render_tab` | -> 313 µs (no change vs B) |

The multi-arrangement benchmarks (3 and 5 arrangements, ~20 ms) showed no
statistically significant change at any site, because their cost is the
sequential `yen()` step that no candidate site touches.

The cause is structural, not a tuning problem. `yen()` is inherently
sequential: each of the k paths is found relative to the ones before it, so the
pipeline has no parallel headroom at the step that actually costs. The three
candidate sites are all pre- or post-`yen()` and individually fast (µs per
element), so Rayon's thread-pool spin-up and work-distribution overhead
(~50-100 µs per `par_iter()` invocation on this hardware) exceeds the work being
parallelized. The `bench_create_single_composition_scaling` cases that looked
flat or slightly faster were measuring memoize cache hits plus `render_tab`, not
the cold computation.

## Consequences

- No Rayon dependency. Keeping it out also avoids a
  `#[cfg(not(target_arch = "wasm32"))]` split, since Rayon's threads do not
  cross the WASM boundary without `wasm-bindgen-rayon` and a browser worker
  pool.
- The `process_beat` helper extracted during Site B is a clean refactor on its
  own and could be lifted from the branch independently of the `par_iter()`
  call. The parallelism is what's rejected, not the extraction.
- Re-investigating parallelism is only worth it if the workload shape changes:
  input grows to hundreds of beats with dense chords, per-element fingering work
  gets much heavier, or WASM threading (`wasm-bindgen-rayon` plus a worker pool)
  lands and the target becomes browser parallelism rather than wall-clock on one
  core.
- The real performance lever, if one is ever needed, is `yen()` itself or the
  graph feeding it (a tighter difficulty heuristic to prune the search, smaller
  node-group fan-out, or an earlier search cutoff), not data parallelism over
  the surrounding stages.
