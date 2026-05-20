# Empty / all-rest input returns Ok(set), not an error

In 2.0.0, `build_arrangement_set` returns `Ok(set)` when the input parses cleanly but contains zero `Playable` lines (only rests, only measure breaks, or fully blank). `set.len == num_arrangements`, every `set.render(i, ...)` returns the empty string, and `set.normalized_input()` echoes the input lines as `Rest` / `MeasureBreak` variants. Pinned by `empty_input_returns_set_with_requested_count` in `src/lib.rs`.

## Considered Options

- **Return `TabError::InvalidInput { field: "input", message: "Input contains no playable pitches" }`.** Argues that requesting arrangements of nothing has no meaningful answer. Rejected: empty input is the dominant state during interactive editing. Treating every keystroke between deletion and the next pitch as an error forces every UI to handle a parse-style failure for what is really just an in-progress state.
- **Return `TabError::Parse` with a synthetic error.** Same complaint as above plus the lie of pretending the input failed to parse when it parsed fine.
- **Return `Ok(set)` with the requested arrangement count and empty renders.** Picked. The shape callers handle for "rendered tab" is the same shape they handle for "nothing to render yet."

## Consequences

- Interactive UIs (the in-repo demo, the noahbaculi.com app) handle "no playable beats yet" with the same `set` shape as a normal render. No error-pane bounce per keystroke.
- Callers that need to distinguish "empty" from "rendered" check `set.isEmpty` or `set.len == 0` and look at the render outputs.
- The `first_playable_index` fallback in `build_arrangement_set` (which decides where `normalized_input` starts) falls back to 0 for empty inputs. This is intentional and `empty_input_returns_set_with_requested_count` pins it.
