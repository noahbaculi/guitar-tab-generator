# Empty / all-rest input returns Ok(set), not an error

In 2.0.0, `generate_arrangements` returns `Ok(set)` when the input parses cleanly but contains zero `Playable` lines (only rests, only measure breaks, or fully blank). `set.len == num_arrangements`, every `set.render(i, ...)` returns the empty string, and `set.normalized_input()` echoes the input lines as `Rest` / `MeasureBreak` variants. Pinned by `empty_input_returns_set_with_requested_count` in `src/lib.rs`.

This ADR also covers the related but distinct case where `max_fret_span_filter` rejects every candidate arrangement. The two paths share a motivation (a benign zero state should not look like a parse failure) but they surface differently on the handle.

## Considered Options

- **Return a typed `TabError` variant for "no playable pitches".** Argues that requesting arrangements of nothing has no meaningful answer. Rejected: empty input is the dominant state during interactive editing. Treating every keystroke between deletion and the next pitch as an error forces every UI to handle a parse-style failure for what is really just an in-progress state.
- **Return `TabError::Parse` with a synthetic error.** Same complaint as above plus the lie of pretending the input failed to parse when it parsed fine.
- **Return `Ok(set)` with the requested arrangement count and empty renders.** Picked for the empty-input case. The shape callers handle for "rendered tab" is the same shape they handle for "nothing to render yet."
- **For the filter-drops-everything case, return `Ok(set)` with zero arrangements.** Picked. The filter argument is documented to relax the requested count when no fingering passes the span ceiling; surfacing this as `set.len == 0` lets the UI render a "no arrangements within span" hint without diverging from the success path.

## Consequences

Two distinct zero-arrangement states exist on `ArrangementSet`. Callers that need to distinguish them check different fields:

| Case | `set.len` | `set.isEmpty` | `set.render(i, ...)` | `set.normalizedInput` |
|---|---|---|---|---|
| Empty / all-rest input | `num_arrangements` | `false` | `""` for every `i` | echoes input as `Rest` / `MeasureBreak` beats |
| `maxFretSpanFilter` dropped every candidate | `0` | `true` | `index` out of bounds -> `TabError::IndexOutOfBounds` | echoes the playable input the filter rejected |

- Interactive UIs (the in-repo demo, the noahbaculi.com app) handle "no playable beats yet" with the same `set` shape as a normal render. No error-pane bounce per keystroke.
- To detect empty / all-rest input, callers check `set.render(0, ...).is_empty()` (or walk `set.normalizedInput` for any `Playable` variant). `set.isEmpty` will not flip on this path.
- To detect the filter-drops-everything case, callers check `set.isEmpty` (equivalently `set.len === 0`). `set.normalizedInput` still holds the playable input the filter rejected, so the UI can render the source while explaining why no arrangement appears.
- The `first_playable_index` fallback in `generate_arrangements` (which decides where `normalized_input` starts) falls back to 0 for empty inputs. This is intentional and `empty_input_returns_set_with_requested_count` pins it.
- Tests pin both behaviours: `empty_input_returns_set_with_requested_count` in `src/lib.rs` and `arrangement_set_is_empty_when_filter_drops_every_candidate` in `tests/integration_public_surface.rs`.
