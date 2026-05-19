# 2.0.0 WASM Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the 2.0.0 breaking WASM surface redesign described in `.scratch/2.0.0-wasm-surface/PRD.md`, replacing the serde-shaped `Vec<Composition>` boundary with an opaque `ArrangementSet` handle, typed via tsify-next, with structured errors and a per-arrangement render method.

**Architecture:** Two free `#[wasm_bindgen]` functions (`generate_arrangements`, `get_tuning_names`) plus a `#[wasm_bindgen]` opaque-handle struct (`ArrangementSet`) with getters and a `render` method. All other boundary types (`TabInput`, `NormalizedBeat`, `TabError`, `ParseError`, `TuningName`) cross the boundary via tsify-next + serde-wasm-bindgen. A new `src/error.rs` module hosts `ParseError` and `TabError`; the parser imports `ParseError` for its internal `Result` type so the same struct serves both internal use and the wire.

**Tech Stack:** Rust (edition 2021), wasm-bindgen 0.2.104, tsify-next 0.5 (new dep), serde-wasm-bindgen 0.6, memoize 0.6, anyhow, pathfinding 4.14, strum 0.28. Tests: built-in `cargo test`, proptest 1, criterion 0.8.

**Spec:** `.scratch/2.0.0-wasm-surface/PRD.md`. The plan references the spec for higher-level rationale; this document is the executable steps.

**Spec deviations recorded:**
- The spec said "Dropped: `memoized_original_create_arrangements`, `memoized_original_parse_lines`". Verified during plan-writing that `benches/benchmarks.rs` depends on both to compare memoized vs un-memoized performance. Plan keeps them public. They are tiny re-exports; the demo never sees them.

---

## Task 0: Land in-flight rename work, then branch

The working tree currently holds CONTEXT.md-aligned internal renames (`BeatFingeringCombo` to `ScoredBeatFingering`, `Node::Note` to `Node::Playable`, `Guitar.num_frets` to `Guitar.playable_frets`, `sonorous` to `beat` in renderer.rs, partial `Composition` to `RenderedTab` rename in lib.rs, `version` bumped to `2.0.0`). These are consistent with the 2.0.0 direction but are not strictly the 2.0.0 surface change.

**Files:**
- Inspect: `src/arrangement.rs`, `src/guitar.rs`, `src/lib.rs`, `src/renderer.rs`, `types.md`, `Cargo.toml`

- [ ] **Step 1: Inspect current diff**

```bash
git status
git diff HEAD -- src/arrangement.rs src/guitar.rs src/lib.rs src/renderer.rs types.md Cargo.toml
```

Expected: the renames listed above plus the partial `Composition` to `RenderedTab` rename in `src/lib.rs` and the `version = "2.0.0"` bump.

- [ ] **Step 2: Run the test suite to confirm green starting state**

```bash
cargo test
```

Expected: all tests pass. If they don't, stop and resolve before continuing — the 2.0.0 work assumes a green baseline.

- [ ] **Step 3: Stage and commit the in-flight rename work**

```bash
git add src/arrangement.rs src/guitar.rs src/lib.rs src/renderer.rs types.md Cargo.toml
git commit -m "refactor: align internal names with CONTEXT.md"
```

The `Cargo.toml` version bump and the partial `Composition` to `RenderedTab` rename ride along; the partial rename gets superseded in later tasks but staying mid-rename in the working tree makes git history messier.

- [ ] **Step 4: Stage and commit the untracked CONTEXT.md and ADR**

```bash
git add CONTEXT.md docs/adr/0001-arrangement-set-opaque-handle.md
git commit -m "docs: domain glossary and ADR for 2.0.0 WASM surface"
```

- [ ] **Step 5: Stage and commit the spec and plan**

```bash
git add .scratch/2.0.0-wasm-surface/
git commit -m "docs: 2.0.0 WASM surface PRD and implementation plan"
```

- [ ] **Step 6: Create a feature branch for the implementation work**

```bash
git checkout -b refactor/wasm-surface-2.0.0
```

---

## Task 1: Add tsify-next dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add tsify-next under `[dependencies]`**

In `Cargo.toml`, insert this line (alphabetically, after `strum_macros`):

```toml
tsify-next = { version = "0.5", features = ["js"] }
```

- [ ] **Step 2: Verify the dep resolves**

```bash
cargo check
```

Expected: PASS. No warnings about unused dep yet (it has no users).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add tsify-next dependency for typed WASM bindings"
```

---

## Task 2: Create error module with ParseError and TabError

**Files:**
- Create: `src/error.rs`
- Modify: `src/lib.rs` (register the module)
- Test: `src/error.rs` (inline `#[cfg(test)] mod` block)

- [ ] **Step 1: Write the failing test for ParseError Display**

Create `src/error.rs` with the following content:

```rust
//! Error types crossing the WASM boundary.
//!
//! `ParseError` is used both internally by the parser and as a leaf of `TabError::Parse`.
//! `TabError` is the tagged enum the WASM boundary throws on failure.

use serde::Serialize;
use tsify_next::Tsify;

/// One unparseable substring in the input, with its 1-indexed line number.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct ParseError {
    pub line: u32,
    pub text: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Input '{}' on line {} could not be parsed into a pitch.",
            self.text, self.line
        )
    }
}

/// Top-level error variant for the WASM boundary.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TabError {
    Parse { errors: Vec<ParseError> },
    Guitar { message: String },
    Arrangement { message: String },
    InvalidInput { field: String, message: String },
}

impl std::fmt::Display for TabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TabError::Parse { errors } => {
                let joined = errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
                write!(f, "{joined}")
            }
            TabError::Guitar { message } => write!(f, "{message}"),
            TabError::Arrangement { message } => write!(f, "{message}"),
            TabError::InvalidInput { field, message } => {
                write!(f, "invalid input for `{field}`: {message}")
            }
        }
    }
}

impl std::error::Error for TabError {}

#[cfg(test)]
mod test_parse_error_display {
    use super::*;

    #[test]
    fn reproduces_legacy_message_format() {
        let err = ParseError {
            line: 4,
            text: "BB.2".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "Input 'BB.2' on line 4 could not be parsed into a pitch."
        );
    }
}

#[cfg(test)]
mod test_tab_error_display {
    use super::*;

    #[test]
    fn parse_variant_joins_errors_with_newlines() {
        let err = TabError::Parse {
            errors: vec![
                ParseError { line: 1, text: "xyz".to_owned() },
                ParseError { line: 4, text: "BB.2".to_owned() },
            ],
        };
        assert_eq!(
            err.to_string(),
            "Input 'xyz' on line 1 could not be parsed into a pitch.\nInput 'BB.2' on line 4 could not be parsed into a pitch."
        );
    }

    #[test]
    fn invalid_input_includes_field_name() {
        let err = TabError::InvalidInput {
            field: "numArrangements".to_owned(),
            message: "must be between 1 and 20 inclusive, got 0".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "invalid input for `numArrangements`: must be between 1 and 20 inclusive, got 0"
        );
    }
}
```

In `src/lib.rs`, register the new module by adding this line near the other `mod` declarations (just after `pub(crate) mod arrangement;`):

```rust
pub(crate) mod error;
```

And add the `pub use` line near the other `pub use` lines:

```rust
pub use error::{ParseError, TabError};
```

- [ ] **Step 2: Run tests to verify they pass**

```bash
cargo test error::
```

Expected: PASS. Three tests: `reproduces_legacy_message_format`, `parse_variant_joins_errors_with_newlines`, `invalid_input_includes_field_name`.

- [ ] **Step 3: Run the full suite**

```bash
cargo test
```

Expected: PASS. All existing tests untouched.

- [ ] **Step 4: Commit**

```bash
git add src/error.rs src/lib.rs
git commit -m "feat(error): introduce ParseError and TabError types"
```

---

## Task 3: Refactor parser to use structured ParseError

The parser currently joins error strings inside `parse_pitch` and returns `Result<_, Arc<anyhow::Error>>` from `parse_lines`. After this task, `parse_pitch` returns `Vec<ParseError>` on the error path, and `parse_lines` returns `Result<_, Arc<Vec<ParseError>>>`.

**Files:**
- Modify: `src/parser.rs`
- Modify: `examples/advanced.rs` (it currently `Arc::try_unwrap(e).unwrap()` on the old anyhow error type)

- [ ] **Step 1: Update the test expectation in `test_parse_lines::reports_line_and_content_for_unparseable_input`**

In `src/parser.rs`, replace the existing test body:

```rust
#[test]
fn reports_line_and_content_for_unparseable_input() {
    let input = "A3xyz\nE2\n\nG4BB.2\n-\nE4".to_owned();

    let errors = parse_lines(input).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].line, 1);
    assert_eq!(errors[0].text, "xyz");
    assert_eq!(errors[1].line, 4);
    assert_eq!(errors[1].text, "BB.2");
}
```

(Old assertion compared the joined string; new one walks the structured Vec.)

- [ ] **Step 2: Update tests in `test_parse_pitch` that assert on error message text**

In `src/parser.rs`, find `test_parse_pitch::invalid_typo`, `invalid_pitch`, and `invalid_random`, and rewrite each to assert on the structured `Vec<ParseError>` instead of formatted strings. Replacement bodies:

```rust
#[test]
fn invalid_typo() {
    let errors = parse_pitch(&test_pitch_regex(), 12, "ZA2G#444B3").unwrap_err();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].line, 13);
    assert_eq!(errors[0].text, "Z");
    assert_eq!(errors[1].line, 13);
    assert_eq!(errors[1].text, "44");
}
#[test]
fn invalid_pitch() {
    let errors = parse_pitch(&test_pitch_regex(), 28, "Fb3").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].line, 29);
    assert_eq!(errors[0].text, "Fb3");
}
#[test]
fn invalid_random() {
    let errors = parse_pitch(&test_pitch_regex(), 0, "baS3Q-hNr").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].line, 1);
    assert_eq!(errors[0].text, "baS3Q-hNr");
}
```

Also update `test_parse_line::reports_error_for_unparseable_text` to assert on the structured form:

```rust
#[test]
fn reports_error_for_unparseable_text() {
    let errors = parse_line(&test_pitch_regex(), 4, "  Invalid Text  ").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].line, 5);
    assert_eq!(errors[0].text, "InvalidText");
}
```

- [ ] **Step 3: Run the tests, confirm they fail**

```bash
cargo test parser::
```

Expected: FAIL (compile errors and/or assertion failures — the implementation still produces `anyhow::Error`).

- [ ] **Step 4: Refactor `parse_pitch` to return `Vec<ParseError>`**

In `src/parser.rs`, replace the entire `parse_pitch` function body. The new signature returns `Vec<ParseError>` on the error side:

```rust
/// Parses input line to extract valid musical pitches, returning structured errors for any
/// substring that cannot be parsed.
fn parse_pitch(
    regex: &Regex,
    input_index: usize,
    input_line: &str,
) -> Result<Line<Vec<Pitch>>, Vec<crate::error::ParseError>> {
    let mut matched_mask = vec![false; input_line.len()];
    let mut matched_pitches: Vec<Pitch> = Vec::new();

    for regex_match in regex.find_iter(input_line) {
        if let Ok(pitch) = Pitch::from_str(regex_match.as_str()) {
            matched_pitches.push(pitch);
            for slot in matched_mask
                .iter_mut()
                .take(regex_match.end())
                .skip(regex_match.start())
            {
                *slot = true;
            }
        }
    }

    let unmatched_indices: Vec<usize> = matched_mask
        .iter()
        .enumerate()
        .filter_map(|(idx, matched)| if *matched { None } else { Some(idx) })
        .collect();

    if !unmatched_indices.is_empty() {
        let line_number = (input_index + 1) as u32;
        let consecutive_indices = consecutive_slices(&unmatched_indices);
        let errors: Vec<crate::error::ParseError> = consecutive_indices
            .into_iter()
            .map(|unmatched_input_indices| {
                let first_idx = *unmatched_input_indices.first().unwrap();
                let last_idx = *unmatched_input_indices.last().unwrap();
                let unmatched_input = &input_line[first_idx..=last_idx];
                crate::error::ParseError {
                    line: line_number,
                    text: unmatched_input.to_owned(),
                }
            })
            .collect();
        return Err(errors);
    }

    Ok(Line::Playable(matched_pitches))
}
```

- [ ] **Step 5: Propagate the new error type through `parse_line` and `parse_lines`**

In `src/parser.rs`, change `parse_line` to:

```rust
fn parse_line(
    regex: &Regex,
    input_index: usize,
    mut input_line: &str,
) -> Result<Line<Vec<Pitch>>, Vec<crate::error::ParseError>> {
    input_line = remove_comments(input_line);
    let line_content: String = remove_whitespace(input_line);

    if let Some(rest) = parse_rest(&line_content) {
        return Ok(rest);
    }
    if let Some(measure_break) = parse_measure_break(&line_content) {
        return Ok(measure_break);
    }
    parse_pitch(regex, input_index, &line_content)
}
```

Change `parse_lines` to return `Arc<Vec<ParseError>>` on the error side:

```rust
#[memoize(Capacity: 10)]
pub fn parse_lines(
    input: String,
) -> Result<Vec<Line<BeatVec<Pitch>>>, Arc<Vec<crate::error::ParseError>>> {
    let pitch_regex = RegexBuilder::new(PITCH_PATTERN)
        .case_insensitive(true)
        .build()
        .expect("BUG: Regex pattern should be valid");

    let (parsed_lines, errors): (Vec<Line<BeatVec<Pitch>>>, Vec<Vec<crate::error::ParseError>>) =
        input
            .lines()
            .enumerate()
            .map(|(input_index, input_line)| parse_line(&pitch_regex, input_index, input_line))
            .partition_map(|result| match result {
                Ok(line) => itertools::Either::Left(line),
                Err(errs) => itertools::Either::Right(errs),
            });

    let flat_errors: Vec<crate::error::ParseError> = errors.into_iter().flatten().collect();
    if !flat_errors.is_empty() {
        return Err(Arc::new(flat_errors));
    }

    Ok(parsed_lines)
}
```

Note: the existing `use anyhow::{anyhow, Result};` in `src/parser.rs` is no longer needed if these are the only callers of `anyhow!`. Verify with `cargo check`; remove the unused import if the compiler flags it. Other callers in the same file (e.g., `create_string_tuning_offset`) may still need `Result`, so keep the import as-is unless the compiler complains.

- [ ] **Step 6: Run the parser tests to verify they pass**

```bash
cargo test parser::
```

Expected: PASS. All parser unit tests including the rewritten error assertions.

- [ ] **Step 7: Update `examples/advanced.rs` to use the new error type**

In `examples/advanced.rs`, line 30, replace:

```rust
let lines: Vec<Line<Vec<Pitch>>> = match parse_lines(input) {
    Ok(input_lines) => input_lines,
    Err(e) => return Err(std::sync::Arc::try_unwrap(e).unwrap()),
};
```

with:

```rust
let lines: Vec<Line<Vec<Pitch>>> = match parse_lines(input) {
    Ok(input_lines) => input_lines,
    Err(errs) => {
        let joined = errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
        return Err(anyhow::anyhow!(joined));
    }
};
```

- [ ] **Step 8: Verify the example still builds**

```bash
cargo build --example advanced
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add src/parser.rs examples/advanced.rs
git commit -m "refactor(parser): return structured ParseError instead of joined strings"
```

---

## Task 4: Add boundary input types (TabInput, NormalizedBeat)

These types only land at the boundary; nothing inside the crate consumes them until Task 5.

**Files:**
- Modify: `src/lib.rs`
- Test: `src/lib.rs` (inline)

- [ ] **Step 1: Write a serialization round-trip test**

Add this test module to `src/lib.rs` (near the bottom):

```rust
#[cfg(test)]
mod test_boundary_types {
    use super::*;

    #[test]
    fn tab_input_deserializes_from_camelcase_json() {
        let json = r#"{
            "input": "E2\nA2",
            "tuningName": "standard",
            "guitarNumFrets": 18,
            "guitarCapo": 0,
            "numArrangements": 1,
            "maxFretSpanFilter": null
        }"#;
        let input: TabInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.input, "E2\nA2");
        assert_eq!(input.tuning_name, "standard");
        assert_eq!(input.guitar_num_frets, 18);
        assert_eq!(input.num_arrangements, 1);
        assert!(input.max_fret_span_filter.is_none());
    }

    #[test]
    fn normalized_beat_serializes_with_kind_discriminant() {
        let playable = NormalizedBeat::Playable {
            pitches: vec!["E2".to_owned()],
        };
        let json = serde_json::to_string(&playable).unwrap();
        assert_eq!(json, r#"{"kind":"playable","pitches":["E2"]}"#);

        let rest = NormalizedBeat::Rest;
        let json = serde_json::to_string(&rest).unwrap();
        assert_eq!(json, r#"{"kind":"rest"}"#);

        let mb = NormalizedBeat::MeasureBreak;
        let json = serde_json::to_string(&mb).unwrap();
        assert_eq!(json, r#"{"kind":"measureBreak"}"#);
    }
}
```

Add `serde_json` as a dev-dependency in `Cargo.toml` (under `[dev-dependencies]`):

```toml
serde_json = "1"
```

- [ ] **Step 2: Run tests to confirm failure**

```bash
cargo test test_boundary_types
```

Expected: FAIL (compile error: `TabInput` and `NormalizedBeat` not yet defined).

- [ ] **Step 3: Add `TabInput` and `NormalizedBeat` to `src/lib.rs`**

In `src/lib.rs`, add these definitions near the top of the file (after the imports, before the existing `CompositionInput` struct):

```rust
use tsify_next::Tsify;

/// Configuration bundle for one tab-generation request.
///
/// Crosses the WASM boundary via `tsify_next`; JS sees a camelCase interface generated
/// alongside the `.wasm`. `num_arrangements` must be in `1..=20`; the value is validated
/// at the boundary and a `TabError::InvalidInput` is thrown when out of range.
#[derive(Debug, Clone, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct TabInput {
    pub input: String,
    pub tuning_name: String,
    pub guitar_num_frets: u8,
    pub guitar_capo: u8,
    pub num_arrangements: u8,
    pub max_fret_span_filter: Option<u8>,
}

/// One beat in the normalized input echoed back from `ArrangementSet::normalized_input`.
///
/// Serialized as a discriminated union tagged by `kind`, so JS code can `switch (b.kind)`
/// instead of comparing strings.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NormalizedBeat {
    Playable { pitches: Vec<String> },
    Rest,
    MeasureBreak,
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test test_boundary_types
```

Expected: PASS.

- [ ] **Step 5: Run the full suite**

```bash
cargo test
```

Expected: PASS. The existing `wasm_create_guitar_compositions` and `wrapper_create_arrangements` still work; the new types are additive.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs
git commit -m "feat(lib): add TabInput and NormalizedBeat boundary types"
```

---

## Task 5: Add ArrangementSet handle struct

Add the opaque handle but with stub method bodies that delegate to the same paths the existing `wrapper_create_arrangements` uses. The handle holds Rust-side state and exposes typed getters and a `render` method. Test the handle directly through the Rust API; the WASM entry point gets added in Task 6.

**Files:**
- Modify: `src/arrangement.rs` (expose `difficulty` via an accessor)
- Modify: `src/lib.rs`
- Test: `src/lib.rs` (inline)

- [ ] **Step 1: Expose `Arrangement::difficulty` via an accessor method**

`Arrangement.difficulty` is currently a module-private field. Add a public method matching the existing `max_fret_span()` pattern so `ArrangementSet::difficulty` can delegate to it.

In `src/arrangement.rs`, inside `impl Arrangement { ... }` (just after the existing `max_fret_span` method), add:

```rust
/// The difficulty score of this arrangement. Lower is easier. Equal to the sum of
/// transition difficulties along the chosen path through the fingering graph.
#[must_use]
pub fn difficulty(&self) -> i32 {
    self.difficulty
}
```

- [ ] **Step 2: Write tests for `ArrangementSet` methods (failing)**

Add to the `test_boundary_types` module in `src/lib.rs`:

```rust
#[test]
fn arrangement_set_len_matches_num_arrangements() {
    let set = arrangement_set_fixture(2);
    assert_eq!(set.len(), 2);
}

#[test]
fn arrangement_set_normalized_input_is_tagged_variants() {
    let set = arrangement_set_fixture(1);
    let beats = set.normalized_input();
    assert!(matches!(beats[0], NormalizedBeat::Playable { .. }));
}

#[test]
fn arrangement_set_render_returns_string_for_in_bounds_index() {
    let set = arrangement_set_fixture(1);
    let tab = set.render(0, 30, 2, None).unwrap();
    assert!(!tab.is_empty());
}

#[test]
fn arrangement_set_render_rejects_out_of_bounds_index() {
    let set = arrangement_set_fixture(1);
    let err = set.render(99, 30, 2, None).unwrap_err();
    match err {
        TabError::InvalidInput { field, .. } => assert_eq!(field, "index"),
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[test]
fn arrangement_set_max_fret_span_returns_value_for_in_bounds_index() {
    let set = arrangement_set_fixture(1);
    let span = set.max_fret_span(0).unwrap();
    assert!(span < 25);
}

#[test]
fn arrangement_set_difficulty_returns_value_for_in_bounds_index() {
    let set = arrangement_set_fixture(1);
    let _difficulty = set.difficulty(0).unwrap();
}

fn arrangement_set_fixture(num_arrangements: u8) -> ArrangementSet {
    let tab_input = TabInput {
        input: "E2\nA2\nD3".to_owned(),
        tuning_name: "standard".to_owned(),
        guitar_num_frets: 20,
        guitar_capo: 0,
        num_arrangements,
        max_fret_span_filter: None,
    };
    build_arrangement_set(tab_input).unwrap()
}
```

- [ ] **Step 3: Run tests to confirm failure**

```bash
cargo test test_boundary_types
```

Expected: FAIL — `ArrangementSet`, `build_arrangement_set`, and method names are not yet defined.

- [ ] **Step 4: Implement `ArrangementSet` and `build_arrangement_set`**

In `src/lib.rs`, add after the `NormalizedBeat` definition:

```rust
/// Opaque handle holding the result of one `generate_arrangements` call.
///
/// Owns the arrangements, the guitar configuration, and the normalized input shared across
/// arrangements. Per-arrangement metadata (`difficulty`, `max_fret_span`) and the rendered
/// tab string are reached by index through methods on the handle.
#[wasm_bindgen]
pub struct ArrangementSet {
    arrangements: Vec<arrangement::Arrangement>,
    guitar: Guitar,
    normalized_input: Vec<NormalizedBeat>,
}

#[wasm_bindgen]
impl ArrangementSet {
    /// Number of arrangements in the set. Equal to the requested `num_arrangements`, possibly
    /// reduced by `max_fret_span_filter` when filtering would otherwise drop below the count.
    #[wasm_bindgen(getter)]
    pub fn len(&self) -> usize {
        self.arrangements.len()
    }

    /// Returns true when `len() == 0`.
    #[wasm_bindgen(getter, js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.arrangements.is_empty()
    }

    /// The per-beat input echoed back as a sequence of tagged `NormalizedBeat` variants.
    /// Shared across all arrangements; lives once on the set.
    #[wasm_bindgen(getter, js_name = "normalizedInput")]
    pub fn normalized_input(&self) -> Vec<NormalizedBeat> {
        self.normalized_input.clone()
    }

    /// Largest non-zero fret span across any beat in the arrangement at `index`.
    #[wasm_bindgen(js_name = "maxFretSpan")]
    pub fn max_fret_span(&self, index: usize) -> Result<u8, TabError> {
        self.arrangements
            .get(index)
            .map(|a| a.max_fret_span())
            .ok_or_else(|| out_of_bounds_error(index, self.arrangements.len()))
    }

    /// Difficulty score for the arrangement at `index`. Lower is easier.
    pub fn difficulty(&self, index: usize) -> Result<i32, TabError> {
        self.arrangements
            .get(index)
            .map(|a| a.difficulty())
            .ok_or_else(|| out_of_bounds_error(index, self.arrangements.len()))
    }

    /// Renders the arrangement at `index` at the supplied `width`, `padding`, and optional
    /// `playback` beat indicator. Cheap to call repeatedly with different render parameters
    /// — pathfinding does not re-run.
    pub fn render(
        &self,
        index: usize,
        width: u16,
        padding: u8,
        playback: Option<u16>,
    ) -> Result<String, TabError> {
        let arrangement = self
            .arrangements
            .get(index)
            .ok_or_else(|| out_of_bounds_error(index, self.arrangements.len()))?;
        Ok(renderer::render_tab(
            &arrangement.lines,
            &self.guitar,
            width,
            padding,
            playback,
        ))
    }
}

fn out_of_bounds_error(index: usize, len: usize) -> TabError {
    TabError::InvalidInput {
        field: "index".to_owned(),
        message: format!("index {index} is out of bounds for set of length {len}"),
    }
}

/// Builds an `ArrangementSet` from a `TabInput`. The Rust-side entry; the WASM entry point
/// added in the next task wraps this for the boundary.
pub fn build_arrangement_set(tab_input: TabInput) -> Result<ArrangementSet, TabError> {
    if !(1..=20).contains(&tab_input.num_arrangements) {
        return Err(TabError::InvalidInput {
            field: "numArrangements".to_owned(),
            message: format!(
                "must be between 1 and 20 inclusive, got {}",
                tab_input.num_arrangements
            ),
        });
    }

    let input_lines = parser::parse_lines(tab_input.input.clone()).map_err(|errs| {
        TabError::Parse {
            errors: std::sync::Arc::try_unwrap(errs).unwrap_or_else(|arc| (*arc).clone()),
        }
    })?;

    let first_playable_index = input_lines
        .iter()
        .position(|line| matches!(line, arrangement::Line::Playable(_)))
        .unwrap_or(0);

    let normalized_input: Vec<NormalizedBeat> = input_lines
        .iter()
        .skip(first_playable_index)
        .map(|line| match line {
            arrangement::Line::Playable(pitches) => NormalizedBeat::Playable {
                pitches: pitches.iter().map(|p| p.plain_text().to_owned()).collect(),
            },
            arrangement::Line::Rest => NormalizedBeat::Rest,
            arrangement::Line::MeasureBreak => NormalizedBeat::MeasureBreak,
        })
        .collect();

    let tuning = parser::create_string_tuning_offset(parser::parse_tuning(&tab_input.tuning_name));
    let guitar = Guitar::new(tuning, tab_input.guitar_num_frets, tab_input.guitar_capo)
        .map_err(|e| TabError::Guitar { message: e.to_string() })?;

    let arrangements = arrangement::create_arrangements(
        guitar.clone(),
        input_lines,
        tab_input.num_arrangements,
    )
    .map_err(|e| TabError::Arrangement { message: e.to_string() })?;

    Ok(ArrangementSet {
        arrangements,
        guitar,
        normalized_input,
    })
}
```

(`build_arrangement_set` deliberately calls `arrangement::create_arrangements` with the existing three-argument signature for now — Task 7 widens it to take the filter.)

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test test_boundary_types
```

Expected: PASS — all six `arrangement_set_*` tests and the two boundary type tests.

- [ ] **Step 6: Run the full suite**

```bash
cargo test
```

Expected: PASS. Legacy tests still green.

- [ ] **Step 7: Commit**

```bash
git add src/arrangement.rs src/lib.rs
git commit -m "feat(lib): add ArrangementSet opaque handle with render method"
```

---

## Task 6: Add generate_arrangements WASM entry point

Wrap `build_arrangement_set` with the `#[wasm_bindgen]` attribute and the camelCase `js_name`.

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Add the WASM entry point**

In `src/lib.rs`, add after `build_arrangement_set`:

```rust
/// WASM-facing entry point. Generates an `ArrangementSet` from a `TabInput`.
///
/// # Errors
///
/// Returns `TabError::InvalidInput` when `num_arrangements` is outside `1..=20`, `TabError::Parse`
/// when the input contains unparseable substrings, `TabError::Guitar` on invalid tuning or capo or
/// fret count, and `TabError::Arrangement` when no valid fingering exists for a pitch.
#[wasm_bindgen(js_name = "generateArrangements")]
#[cfg(not(tarpaulin_include))]
pub fn generate_arrangements(input: TabInput) -> Result<ArrangementSet, TabError> {
    build_arrangement_set(input)
}
```

(Splitting the boundary `generate_arrangements` from the Rust-facing `build_arrangement_set` lets the test suite cover the heavy logic without going through wasm-bindgen's serialization layer; `#[cfg(not(tarpaulin_include))]` matches the existing pattern in the file for excluding pure-WASM glue from coverage.)

- [ ] **Step 2: Verify the build still passes**

```bash
cargo check --target wasm32-unknown-unknown
```

Note: this requires `rustup target add wasm32-unknown-unknown` if not already installed. If the target is unavailable in CI, this step can be skipped — `cargo check` on the host triple still validates the type system, and `wasm-pack build` in Task 11 is the canonical WASM build verification.

```bash
cargo check
```

Expected: PASS.

- [ ] **Step 3: Run the full test suite**

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/lib.rs
git commit -m "feat(lib): add generateArrangements WASM entry point"
```

---

## Task 7: Add max_fret_span_filter end-to-end

Add the filter parameter to `arrangement::create_arrangements`, default it to `None` at all existing call sites, and wire it through `build_arrangement_set` from `TabInput.max_fret_span_filter`.

**Files:**
- Modify: `src/arrangement.rs`
- Modify: `src/lib.rs`
- Modify: `examples/advanced.rs` (call site passes `None`)
- Modify: `benches/benchmarks.rs` (call sites pass `None`)

- [ ] **Step 1: Write a failing test for the filter behavior**

Add to `src/arrangement.rs` (inside the existing `mod test_create_arrangements` block, or a new sibling block — the existing module contains the proptests and unit tests for the function):

```rust
#[test]
fn max_fret_span_filter_drops_high_span_arrangements() {
    let tuning = crate::guitar::create_string_tuning(
        &crate::guitar::STD_6_STRING_TUNING_OPEN_PITCHES,
    )
    .unwrap();
    let guitar = crate::guitar::Guitar::new(tuning, 20, 0).unwrap();
    let lines = crate::parser::parse_lines("E2\nA4".to_owned()).unwrap();

    // Without a filter, expect 5 arrangements (some with wide stretches).
    let unfiltered = create_arrangements(guitar.clone(), lines.clone(), 5, None).unwrap();
    assert!(unfiltered.iter().any(|a| a.max_fret_span() > 0));

    // With filter = Some(0), only arrangements that never stretch survive.
    let filtered = create_arrangements(guitar.clone(), lines, 5, Some(0)).unwrap();
    assert!(filtered.iter().all(|a| a.max_fret_span() == 0));
    assert!(filtered.len() <= 5);
}
```

- [ ] **Step 2: Run the test to confirm failure**

```bash
cargo test arrangement::test_create_arrangements::max_fret_span_filter
```

Expected: FAIL — `create_arrangements` signature does not yet accept the filter argument.

- [ ] **Step 3: Widen `create_arrangements` signature**

In `src/arrangement.rs`, find `pub fn create_arrangements(...)` and change its signature to add the new parameter as the last argument:

```rust
pub fn create_arrangements(
    guitar: Guitar,
    input_lines: Vec<Line<BeatVec<Pitch>>>,
    num_arrangements: u8,
    max_fret_span_filter: Option<u8>,
) -> Result<Vec<Arrangement>, Arc<anyhow::Error>>
```

At the end of the existing body, after the `arrangements` `Vec` is built but before it is returned, insert the post-filter:

```rust
if let Some(max_span) = max_fret_span_filter {
    arrangements.retain(|a| a.max_fret_span() <= max_span);
}

Ok(arrangements)
```

(The pathfinding yen call still runs at the requested `num_arrangements`. The retain trims; the count can drop below `num_arrangements` when the filter rejects too many.)

If the `memoized_original_create_arrangements` re-export is used by the bench, the `#[memoize]`-generated escape hatch keeps the new signature — verify the bench still builds in Step 6 below.

- [ ] **Step 4: Update all internal callers of `create_arrangements`**

Search for `create_arrangements(` in the crate:

```bash
grep -rn "create_arrangements(" src/ benches/ examples/ tests/
```

Update each call site (excluding the function definition itself) to pass `None` as the new fourth argument unless the caller is `build_arrangement_set` in `src/lib.rs`, where it becomes `tab_input.max_fret_span_filter`.

In `src/lib.rs`, change the call inside `build_arrangement_set`:

```rust
let arrangements = arrangement::create_arrangements(
    guitar.clone(),
    input_lines,
    tab_input.num_arrangements,
    tab_input.max_fret_span_filter,
)
.map_err(|e| TabError::Arrangement { message: e.to_string() })?;
```

In `examples/advanced.rs`, line 49, change:

```rust
let arrangements = match create_arrangements(guitar.clone(), lines, num_arrangements) {
```

to:

```rust
let arrangements = match create_arrangements(guitar.clone(), lines, num_arrangements, None) {
```

In `benches/benchmarks.rs`, update every `create_arrangements(` and `memoized_original_create_arrangements(` call to pass an additional `None` final argument. (Run `grep` above to find them; expect ~6 call sites.)

- [ ] **Step 5: Update the proptest call sites**

In `src/arrangement.rs`, inside `mod test_create_arrangements`, find every `create_arrangements(` invocation in the proptest bodies (and any other unit tests) and append `, None` to each.

- [ ] **Step 6: Run the new test, plus the full suite**

```bash
cargo test arrangement::test_create_arrangements::max_fret_span_filter
cargo test
cargo build --benches
cargo build --examples
```

Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
git add src/arrangement.rs src/lib.rs examples/advanced.rs benches/benchmarks.rs
git commit -m "feat(arrangement): add max_fret_span_filter parameter"
```

---

## Task 8: Convert get_tuning_names to a typed return

Currently `get_tuning_names` returns `Result<JsValue, JsError>` carrying a `Vec<String>`. The new shape returns `Vec<TuningName>` directly, which tsify renders as a typed string-literal union on the JS side.

**Files:**
- Modify: `src/parser.rs`

- [ ] **Step 1: Add `Tsify` and `Serialize` to `TuningName`**

In `src/parser.rs`, find the `TuningName` enum and update its derives + attributes:

```rust
use serde::Serialize;
use tsify_next::Tsify;

#[derive(Debug, EnumString, VariantNames, Serialize, Tsify)]
#[strum(ascii_case_insensitive)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum TuningName {
    OpenG,
    OpenD,
    C6,
    #[strum(serialize = "dsus4", serialize = "dadgad")]
    Dsus4,
    DropD,
    DropC,
    OpenC,
    DropB,
    OpenE,
}
```

(The `#[serde(rename_all = "camelCase")]` keeps wire names matching the existing JS expectations like `"openG"`. Note: this changes the wire form from the existing strum `VariantNames` output, which uses the Rust identifier case. If JS senders pass `"OpenG"`, they will continue to work because `parse_tuning` calls `TuningName::from_str` which is `ascii_case_insensitive`. The serialization output, however, changes from `"OpenG"` to `"openG"` — the demo will need to handle both or pick one. **Verify with the demo maintainer before merging.** If the demo only consumes the list and renders it as-is, this is a wire-format breaking change that has to be documented in the CHANGELOG.)

- [ ] **Step 2: Replace `get_tuning_names` body**

In `src/parser.rs`, replace the existing `get_tuning_names`:

```rust
/// Returns the supported `TuningName` variants, typed for JS consumption via tsify.
#[wasm_bindgen(js_name = "getTuningNames")]
#[cfg(not(tarpaulin_include))]
pub fn get_tuning_names() -> Vec<TuningName> {
    TuningName::VARIANTS
        .iter()
        .map(|&v| TuningName::from_str(v).expect("BUG: VARIANTS yields parseable strings"))
        .collect()
}
```

- [ ] **Step 3: Build and verify**

```bash
cargo check
cargo test
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/parser.rs
git commit -m "feat(parser): expose getTuningNames with typed TuningName return"
```

---

## Task 9: Migrate the legacy boundary test

The existing `mod test_wrapper_create_arrangements` in `src/lib.rs` asserts on byte-exact tab output through the old API. Rewrite it as `mod test_generate_arrangements_and_render` against `build_arrangement_set` (so it does not depend on WASM glue). Drop the parts that depend on the legacy `Composition` struct shape.

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Replace the existing `mod test_wrapper_create_arrangements`**

In `src/lib.rs`, find the entire `mod test_wrapper_create_arrangements { ... }` block and replace it with:

```rust
#[cfg(test)]
mod test_generate_arrangements_and_render {
    use super::*;

    #[test]
    fn valid_input() {
        let tab_input = TabInput {
            input: "E2\nA2\nD3\n\nG3\nB3\n---\nE4".to_owned(),
            tuning_name: "standard".to_owned(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 1,
            max_fret_span_filter: None,
        };
        let set = build_arrangement_set(tab_input).unwrap();

        assert_eq!(set.len(), 1);
        assert_eq!(set.max_fret_span(0).unwrap(), 0);

        let tab = set.render(0, 30, 2, Some(3)).unwrap();
        assert_eq!(
            tab,
            "           \u{25bc}\n--------------------|--0------\n-----------------0--|---------\n--------------0-----|---------\n--------0-----------|---------\n-----0--------------|---------\n--0-----------------|---------\n           \u{25b2}\n"
        );

        let beats = set.normalized_input();
        assert_eq!(beats.len(), 8);
        assert!(matches!(beats[0], NormalizedBeat::Playable { ref pitches } if pitches == &["E2".to_owned()]));
        assert!(matches!(beats[3], NormalizedBeat::Rest));
        assert!(matches!(beats[6], NormalizedBeat::MeasureBreak));
    }

    #[test]
    fn empty_input_returns_set_with_requested_count() {
        let tab_input = TabInput {
            input: "\n\n\n---\n \n".to_owned(),
            tuning_name: "standard".to_owned(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 2,
            max_fret_span_filter: None,
        };
        let set = build_arrangement_set(tab_input).unwrap();
        assert_eq!(set.len(), 2);
        assert_eq!(set.render(0, 30, 2, Some(3)).unwrap(), "");
        assert_eq!(set.render(1, 30, 2, Some(3)).unwrap(), "");
    }

    #[test]
    fn invalid_input_returns_parse_error() {
        let tab_input = TabInput {
            input: "E2\nA2\nD3\n???\nG3\nB3\nE4".to_owned(),
            tuning_name: "standard".to_owned(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 1,
            max_fret_span_filter: None,
        };
        let err = build_arrangement_set(tab_input).unwrap_err();
        match err {
            TabError::Parse { errors } => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].line, 4);
                assert_eq!(errors[0].text, "???");
            }
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn num_arrangements_zero_is_invalid() {
        let tab_input = TabInput {
            input: "E2".to_owned(),
            tuning_name: "standard".to_owned(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 0,
            max_fret_span_filter: None,
        };
        let err = build_arrangement_set(tab_input).unwrap_err();
        match err {
            TabError::InvalidInput { field, .. } => assert_eq!(field, "numArrangements"),
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn num_arrangements_above_cap_is_invalid() {
        let tab_input = TabInput {
            input: "E2".to_owned(),
            tuning_name: "standard".to_owned(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 21,
            max_fret_span_filter: None,
        };
        let err = build_arrangement_set(tab_input).unwrap_err();
        match err {
            TabError::InvalidInput { field, .. } => assert_eq!(field, "numArrangements"),
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn render_at_two_widths_produces_different_outputs() {
        let tab_input = TabInput {
            input: "E2\nA2\nD3".to_owned(),
            tuning_name: "standard".to_owned(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 1,
            max_fret_span_filter: None,
        };
        let set = build_arrangement_set(tab_input).unwrap();
        let narrow = set.render(0, 12, 1, None).unwrap();
        let wide = set.render(0, 100, 1, None).unwrap();
        assert_ne!(narrow, wide);
    }
}
```

- [ ] **Step 2: Run the migrated tests**

```bash
cargo test test_generate_arrangements_and_render
```

Expected: PASS — all six tests. The byte-exact tab assertion in `valid_input` must match the legacy assertion; if it does not, double-check that no `render_tab` semantics changed.

- [ ] **Step 3: Run the full suite**

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/lib.rs
git commit -m "test(lib): migrate legacy boundary test to ArrangementSet API"
```

---

## Task 10: Remove the legacy surface

Now that the new surface is complete and tested, delete the old API. After this task, `CompositionInput`, the legacy `RenderedTab` struct (the one that carried `tab`/`normalized_input`/`max_fret_span`), `wrapper_create_arrangements`, and `wasm_create_guitar_compositions` no longer exist.

**Files:**
- Modify: `src/lib.rs`
- Modify: `examples/basic.rs`
- Modify: `benches/benchmarks.rs`

- [ ] **Step 1: Delete the legacy types and functions from `src/lib.rs`**

In `src/lib.rs`:
- Delete the `pub struct CompositionInput { ... }` definition (and its doc comment).
- Delete the `pub struct RenderedTab { ... }` definition (the legacy one with `tab`, `normalized_input`, `max_fret_span` fields — note the type name was renamed from `Composition` in the in-flight work and is being deleted entirely here, not kept).
- Delete the `pub fn wasm_create_guitar_compositions(...)` function.
- Delete the `pub fn wrapper_create_arrangements(...)` function.
- Remove `use std::rc::Rc;` if it is no longer used. Verify with `cargo check`.

- [ ] **Step 2: Rewrite `examples/basic.rs` to use the new API**

Replace `examples/basic.rs` entirely with:

```rust
extern crate guitar_tab_generator;

use guitar_tab_generator::{build_arrangement_set, TabInput};

/// Basic usage example using `build_arrangement_set` and `render`.
fn main() {
    let tab_input = TabInput {
        input: "E4
        Eb4

        E4
        Eb4
        E4
        B3
        D4
        C4
        -
        A2A3
        E3
        A3
        C3
        E3
        A3"
        .to_owned(),
        tuning_name: "standard".to_owned(),
        guitar_num_frets: 18,
        guitar_capo: 0,
        num_arrangements: 1,
        max_fret_span_filter: None,
    };

    let set = build_arrangement_set(tab_input).unwrap();
    let tab = set.render(0, 55, 2, Some(12)).unwrap();
    println!("Tab:\n{tab}");
}
```

- [ ] **Step 3: Update `benches/benchmarks.rs` to use the new API**

In `benches/benchmarks.rs`, change the import line that references the legacy types:

```rust
use guitar_tab_generator::{
    build_arrangement_set, create_arrangements, create_string_tuning,
    memoized_original_create_arrangements, memoized_original_parse_lines, parse_lines,
    render_tab, BeatVec, Guitar, Line, Pitch, StringNumber, TabInput,
    STD_6_STRING_TUNING_OPEN_PITCHES,
};
```

Find every `CompositionInput { ... }` literal and rewrite each as `TabInput { ... }`. The field renames:
- `pitches: ...` becomes `input: ...`
- Remove `width: ...`, `padding: ...`, `playback_index: ...` (now on `render`)
- Add `max_fret_span_filter: None`

Find every `guitar_tab_generator::wrapper_create_arrangements(...)` call and replace with `guitar_tab_generator::build_arrangement_set(...)`. The two top-level benchmarks that benchmarked the full pipeline now benchmark `build_arrangement_set` + a single `set.render(0, width, padding, playback)` to preserve apples-to-apples comparison with the previous numbers. The render parameters that used to live on the input bundle move to the render call.

Concretely, in the section that builds the input bundle:

```rust
let input = guitar_tab_generator::TabInput {
    input: fur_elise_input.to_owned(),
    tuning_name: "standard".to_owned(),
    guitar_num_frets: 18,
    guitar_capo: 0,
    num_arrangements: 1,
    max_fret_span_filter: None,
};
```

And in the timed section:

```rust
let _ =
    guitar_tab_generator::build_arrangement_set(black_box(input.clone()));
```

(If the benchmark also rendered the result, add `set.render(0, 60, 1, None)` inside the same closure after the set is built so the rendered output stays in scope.)

- [ ] **Step 4: Verify everything still builds**

```bash
cargo build --all-targets
```

Expected: PASS.

- [ ] **Step 5: Run the full test suite**

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs examples/basic.rs benches/benchmarks.rs
git commit -m "feat(lib)!: remove legacy CompositionInput and wrapper_create_arrangements

BREAKING CHANGE: 2.0.0 boundary is generate_arrangements + ArrangementSet.
Migration: replace wrapper_create_arrangements(input) with
build_arrangement_set(input) and call set.render(i, width, padding, playback)
per arrangement. See .scratch/2.0.0-wasm-surface/PRD.md for the full delta."
```

---

## Task 11: Documentation sweep

**Files:**
- Modify: `types.md`
- Modify: `README.md`
- Create or modify: `CHANGELOG.md`

- [ ] **Step 1: Rewrite `types.md` to reflect the new pipeline**

The file currently describes a `RenderedTabInput` to `Vec<Composition>` pipeline. Rewrite the top to reflect `TabInput` to `ArrangementSet`. Use this as the replacement first section (keep the lower "Types Up Close" section but rename `Composition` to the new shape):

```markdown
# Types and Data Flow

Input: `TabInput`. Output: `ArrangementSet` (opaque handle).

## Pipeline

\`\`\`
                        TabInput
                        ────────
  input: String │ tuning_name: String │ guitar_num_frets, guitar_capo: u8
          │                  │                        │
          ▼                  ▼                        │
    parse_lines         parse_tuning                  │
          │                  │                        │
          │                  ▼                        │
          │             [i8; 6]                       │
          │                  │                        │
          │                  ▼                        │
          │      create_string_tuning_offset          │
          │                  │                        │
          │                  ▼                        │
          │      BTreeMap<StringNumber, Pitch>        │
          │                  │                        │
          │                  └────────┬───────────────┘
          │                           ▼
          │                      Guitar::new
          │                           │
          │                           ▼
          │                        Guitar
          │                           │
          ▼                           │              num_arrangements: u8
Vec<Line<BeatVec<Pitch>>>             │              max_fret_span_filter: Option<u8>
          │                           │                        │
          └──────────────┬────────────┴────────────────────────┘
                         ▼
                 create_arrangements
                         │
                         ▼
                Vec<Arrangement>
                ─────────────────────────────────────────────────
                 lines        : Vec<Line<BeatVec<PitchFingering>>>
                 difficulty   : i32
                 max_fret_span: u8
                         │
                         ▼
               ArrangementSet (handle)
               ────────────────────────
                arrangements      : Vec<Arrangement>
                guitar            : Guitar
                normalized_input  : Vec<NormalizedBeat>

  per-arrangement reach: set.render(i, width, padding, playback) → String
                         set.max_fret_span(i) → u8
                         set.difficulty(i) → i32
\`\`\`
```

Replace literal backticks `\`\`\`` with real triple backticks when writing the file — they are shown escaped in this plan because the plan itself is a Markdown document.

- [ ] **Step 2: Update `README.md` Future Improvements section**

In `README.md`, find the `## Future Improvements` section and remove the bullet:

```
- [ ] Add filter for max_fret_span in `arrangements`
```

It is now shipped via `TabInput.max_fret_span_filter`.

- [ ] **Step 3: Add a CHANGELOG entry**

Create `CHANGELOG.md` if it does not exist; otherwise append a new section at the top:

```markdown
# Changelog

## 2.0.0 — 2026-MM-DD

### Breaking changes

- `wasm_create_guitar_compositions(input)` replaced by `generateArrangements(input)` (JS) / `generate_arrangements(input)` (Rust).
- `get_tuning_names()` replaced by `getTuningNames()`; returns typed `TuningName[]` instead of `string[]`.
- Output shape: `Composition[]` replaced by a single `ArrangementSet` handle. Access pattern: `for (let i = 0; i < set.len; i++) set.render(i, width, padding, playback)`.
- Input field `pitches` renamed to `input`.
- Input field renames flow to the JS side as `numArrangements`, `tuningName`, `guitarNumFrets`, `guitarCapo`, `maxFretSpanFilter`.
- Render parameters (`width`, `padding`, `playback`) moved from input bundle to `ArrangementSet.render(...)`.
- `normalized_input` sentinels (`"REST"`, `"MEASURE_BREAK"`) replaced by tagged variants (`{ kind: "rest" }`, `{ kind: "measureBreak" }`).
- Errors are typed: throws `TabError` with `kind` discriminator. `Parse` carries `errors[].line` and `errors[].text` for inline editor highlights.
- `TuningName` wire serialization switched from PascalCase (`"OpenG"`) to camelCase (`"openG"`). Input parsing remains case-insensitive.

### Added

- `TabInput.maxFretSpanFilter: Option<u8>` filters arrangements by maximum non-zero fret span.

### Internal

- Adopted `tsify-next` for typed TypeScript bindings.
- Parser returns structured `Vec<ParseError>` internally; the wire format reuses the same struct via `crate::error::ParseError`.
- Released `CONTEXT.md` (domain glossary) and `docs/adr/0001-arrangement-set-opaque-handle.md` (ADR for the opaque-handle pattern).
```

Replace `2026-MM-DD` with the merge date.

- [ ] **Step 4: Verify docs render and the project still builds**

```bash
cargo build --all-targets
cargo test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add types.md README.md CHANGELOG.md
git commit -m "docs: update types.md, README, and CHANGELOG for 2.0.0 surface"
```

---

## Task 12: Build the WASM artifact and smoke-test

This is the final verification step. Build the WASM binary the same way the demo consumes it, then check the generated `.d.ts` for the new types.

**Files:**
- No source files. Build output lands in `pkg/`.

- [ ] **Step 1: Build the WASM artifact**

```bash
wasm-pack build --target web --out-dir pkg/wasm_guitar_tab_generator
```

Expected: build succeeds. (If `wasm-pack` is not installed: `cargo install wasm-pack`.)

- [ ] **Step 2: Inspect the generated TypeScript definitions**

```bash
cat pkg/wasm_guitar_tab_generator/guitar_tab_generator.d.ts
```

Expected to see (among other things):
- `export function generateArrangements(input: TabInput): ArrangementSet;`
- `export function getTuningNames(): TuningName[];`
- `export class ArrangementSet { ... render(...) ... }` with all method signatures.
- `interface TabInput { input: string; tuningName: string; ... }`
- `type NormalizedBeat = { kind: "playable"; pitches: string[] } | { kind: "rest" } | { kind: "measureBreak" };`
- `type TabError = { kind: "parse"; errors: ParseError[] } | { kind: "guitar"; message: string } | ...;`

If any of those are missing or shaped wrong, the corresponding tsify derive in the source needs review.

- [ ] **Step 3: Check binary size hasn't regressed catastrophically**

```bash
ls -l pkg/wasm_guitar_tab_generator/guitar_tab_generator_bg.wasm
```

Compare against the size before this work (the prior `pkg/` checkout, if available). The tsify addition is small; expect at most a few KB of growth in the optimized binary.

- [ ] **Step 4: Final commit (if `pkg/` is checked in to the repo)**

```bash
git add pkg/
git commit -m "build: rebuild WASM artifacts for 2.0.0"
```

If `pkg/` is not checked in (verify with `git check-ignore pkg/`), skip the commit.

---

## Self-review summary

After completing all tasks, the surface looks like this on the JS side:

```ts
import init, { generateArrangements, getTuningNames, ArrangementSet, TabInput, TabError } from "./pkg/wasm_guitar_tab_generator/guitar_tab_generator.js";

await init();

const tunings = getTuningNames(); // TuningName[]

try {
  const set = generateArrangements({
    input: "E2\nA2",
    tuningName: "standard",
    guitarNumFrets: 18,
    guitarCapo: 0,
    numArrangements: 5,
    maxFretSpanFilter: null,
  });

  const beats = set.normalizedInput;
  for (let i = 0; i < set.len; i++) {
    const tab = set.render(i, 80, 2, null);
    const span = set.maxFretSpan(i);
    const difficulty = set.difficulty(i);
    console.log(tab, span, difficulty);
  }
  set.free();
} catch (e) {
  const err = e as TabError;
  if (err.kind === "parse") {
    err.errors.forEach(({ line, text }) => console.error(`line ${line}: "${text}"`));
  }
}
```

All spec requirements covered:
- TabInput, ArrangementSet, NormalizedBeat, TabError, ParseError: Task 4, 5, 2.
- generate_arrangements, getTuningNames, ArrangementSet methods: Task 6, 8, 5.
- Structured ParseError refactor: Task 3.
- max_fret_span_filter: Task 7.
- Boundary validation of num_arrangements: Task 5.
- Test migration: Task 9.
- Legacy surface removal: Task 10.
- Docs sweep: Task 11.
- WASM artifact smoke test: Task 12.
