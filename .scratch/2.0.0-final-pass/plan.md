# 2.0.0 Final Pass Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure `TabError` as a flat tagged union, remove `anyhow` from public Rust signatures, add the missing capo-vs-fret-count cross-check, and drop the empty-string-as-standard tuning quirk. This is the last breaking-change window before 2.0.0 ships.

**Architecture:** All public Rust error returns become `Result<_, TabError>`. `TabError` flattens to 11 specific variants (one per concrete failure mode); the umbrella `Guitar`/`Arrangement`/`InvalidInput` variants are removed. `Pitch::plus_offset` shrinks to `Option<Pitch>` because the value has no contextual data to populate a typed error. `UnplayablePitch` moves from a private struct in `arrangement.rs` to a public Tsify-typed struct in `error.rs`.

**Tech Stack:** Rust (edition 2021), wasm-bindgen 0.2, tsify-next 0.5, serde 1, memoize 0.6, pathfinding 4. Tests: built-in `cargo test`, proptest 1, criterion 0.8.

**Spec:** `.scratch/2.0.0-final-pass/PRD.md` (committed as `980cd8a`).

**Out of scope (do not touch):**
- `parse_lines`, `render_tab`, `create_arrangements`, `create_string_tuning`, `Line`, `BeatVec`, `PitchVec` stay re-exported from the crate root. Surface trim is deferred to a future major.
- Performance work. Covered by `.scratch/post-2.0.0-internals/PRD.md`.

---

## File Structure

| File | Change kind | Responsibility |
|---|---|---|
| `src/error.rs` | Modify | Define flat `TabError` + public `UnplayablePitch`, Display/Error impls |
| `src/string_number.rs` | Modify | `StringNumber::new` returns `Result<Self, TabError>` |
| `src/pitch.rs` | Modify | `Pitch::plus_offset` returns `Option<Pitch>` |
| `src/guitar.rs` | Modify | All public + helper returns become `Result<_, TabError>`; add `CapoExceedsFrets` cross-check |
| `src/arrangement.rs` | Modify | Internal `UnplayablePitch` removed; `create_arrangements` returns `Result<_, Arc<TabError>>`; `validate_fingerings` and `generate_fingering_combos` return `TabError` |
| `src/parser.rs` | Modify | `parse_tuning` returns `TuningNameUnknown`; empty string no longer accepted |
| `src/lib.rs` | Modify | `NumArrangements::try_new` returns `NumArrangementsOutOfRange`; `out_of_bounds_error` returns `IndexOutOfBounds`; drop the `.map_err(\|e\| TabError::Guitar { message: e.to_string() })` wrappers |
| `examples/advanced.rs` | Modify | Adapt `Err` arms to new variants; structure unchanged |
| `tests/integration_public_surface.rs` | Modify | Adapt `Err` arms + add per-variant boundary tests |
| `Cargo.toml` | Modify | Remove `anyhow` if `cargo tree` confirms no direct use |
| `docs/adr/0007-flat-taberror-variants.md` | Create | Record the flat-vs-grouped decision |
| `MIGRATION.md` | Modify | Add the "2.0.0 final pass" subsection |
| `CHANGELOG.md` | Modify | Add bullets for each change |
| `CONTEXT.md` | Modify | Add `UnplayablePitch` entry |

---

## Task 0: Baseline check

**Files:** none (verification only).

- [ ] **Step 1: Confirm working tree is clean and on `v2.0.0`**

```bash
git status
git rev-parse --abbrev-ref HEAD
```

Expected: working tree clean, branch `v2.0.0`.

- [ ] **Step 2: Run the full test suite to confirm green baseline**

```bash
cargo test
```

Expected: every test passes. If any fail, stop and resolve before touching anything; the migration assumes a green starting state.

- [ ] **Step 3: Capture the current `anyhow` footprint**

```bash
cargo tree -i anyhow --depth 1
```

Expected: `anyhow` listed as a direct dependency of `guitar-tab-generator`. Save the output mentally for Task 12 (we will confirm it shrinks after the migration).

---

## Task 1: Add new `TabError` variants and public `UnplayablePitch` (umbrellas stay)

**Files:**
- Modify: `src/error.rs`

The new variants land alongside the existing `Guitar`/`Arrangement`/`InvalidInput` umbrellas. Nothing constructs the new variants yet; this commit only widens the enum.

- [ ] **Step 1: Add `UnplayablePitch` and the new variants to `src/error.rs`**

Replace the contents of `src/error.rs` between the `pub struct ParseError { ... }` block and the existing `pub enum TabError { ... }` block with the following additions, then expand `TabError` to include all new variants (keeping the umbrella variants for now):

Insert this new struct immediately after the existing `ParseError` Display impl (around `src/error.rs:31`):

```rust
/// A pitch that could not be played on the configured guitar, with its 1-indexed line number.
///
/// Public payload of [`TabError::UnplayablePitches`]. Replaces the prose
/// "Pitch X on line N cannot be played on any strings of the configured guitar."
/// string that 1.x and the pre-final 2.0.0 surface returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct UnplayablePitch {
    pub value: String,
    pub line: u32,
}

impl std::fmt::Display for UnplayablePitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pitch {} on line {} cannot be played on any strings of the configured guitar.",
            self.value, self.line
        )
    }
}
```

In the existing `pub enum TabError { ... }` block (currently `src/error.rs:38-46`), add the following variants between `Parse` and `Guitar` (keep `Guitar`, `Arrangement`, `InvalidInput` in place):

```rust
    NumFretsTooHigh { num_frets: u8, max: u8 },
    CapoTooHigh { capo: u8, max: u8 },
    CapoExceedsFrets { capo: u8, num_frets: u8 },
    StringNumberOutOfRange { value: u8, max: u8 },
    OpenPitchOutOfRange { string: u8, semitones: i16 },
    FretRangeExceedsPitchRange { open_pitch: String, playable_frets: u8 },
    UnplayablePitches { pitches: Vec<UnplayablePitch> },
    NumArrangementsOutOfRange { value: u8, max: u8 },
    TuningNameUnknown { value: String },
    IndexOutOfBounds { index: usize, len: usize },
```

In the existing `impl std::fmt::Display for TabError` block (currently `src/error.rs:48-62`), add arms for each new variant. Replace the match body with:

```rust
        match self {
            TabError::Parse { errors } => {
                let joined = errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
                write!(f, "{joined}")
            }
            TabError::NumFretsTooHigh { num_frets, max } => {
                write!(f, "Too many frets ({num_frets}). The maximum is {max}.")
            }
            TabError::CapoTooHigh { capo, max } => {
                write!(f, "The capo fret ({capo}) is too high. The maximum is {max}.")
            }
            TabError::CapoExceedsFrets { capo, num_frets } => {
                write!(
                    f,
                    "The capo fret ({capo}) cannot exceed the number of frets ({num_frets})."
                )
            }
            TabError::StringNumberOutOfRange { value, max } => {
                if *value == 0 {
                    write!(
                        f,
                        "A guitar cannot have a string number of zero (0). Guitar string numbering commences at one (1)."
                    )
                } else {
                    write!(f, "The string number ({value}) is too high. The maximum is {max}.")
                }
            }
            TabError::OpenPitchOutOfRange { string, semitones } => {
                write!(
                    f,
                    "Capo offset of {semitones} semitones on string {string} would push the open pitch out of the supported range."
                )
            }
            TabError::FretRangeExceedsPitchRange { open_pitch, playable_frets } => {
                write!(
                    f,
                    "Too many frets ({playable_frets}) for string starting at pitch {open_pitch}. The highest playable pitch is B9."
                )
            }
            TabError::UnplayablePitches { pitches } => {
                let joined = pitches.iter().map(|p| p.to_string()).collect::<Vec<_>>().join("\n");
                write!(f, "{joined}")
            }
            TabError::NumArrangementsOutOfRange { value, max } => {
                write!(f, "must be between 1 and {max} inclusive, got {value}")
            }
            TabError::TuningNameUnknown { value } => {
                write!(
                    f,
                    "must be \"standard\" or one of the supported TuningName variants, got {value:?}"
                )
            }
            TabError::IndexOutOfBounds { index, len } => {
                write!(f, "index {index} is out of bounds for set of length {len}")
            }
            TabError::Guitar { message } => write!(f, "{message}"),
            TabError::Arrangement { message } => write!(f, "{message}"),
            TabError::InvalidInput { field, message } => {
                write!(f, "invalid input for `{field}`: {message}")
            }
        }
```

- [ ] **Step 2: Add Display unit tests for each new variant**

Add this test module at the end of `src/error.rs`:

```rust
#[cfg(test)]
mod test_new_variant_display {
    use super::*;

    #[test]
    fn num_frets_too_high() {
        let err = TabError::NumFretsTooHigh { num_frets: 31, max: 30 };
        assert_eq!(err.to_string(), "Too many frets (31). The maximum is 30.");
    }

    #[test]
    fn capo_too_high() {
        let err = TabError::CapoTooHigh { capo: 9, max: 8 };
        assert_eq!(err.to_string(), "The capo fret (9) is too high. The maximum is 8.");
    }

    #[test]
    fn capo_exceeds_frets() {
        let err = TabError::CapoExceedsFrets { capo: 8, num_frets: 2 };
        assert_eq!(
            err.to_string(),
            "The capo fret (8) cannot exceed the number of frets (2)."
        );
    }

    #[test]
    fn string_number_out_of_range_zero() {
        let err = TabError::StringNumberOutOfRange { value: 0, max: 12 };
        assert_eq!(
            err.to_string(),
            "A guitar cannot have a string number of zero (0). Guitar string numbering commences at one (1)."
        );
    }

    #[test]
    fn string_number_out_of_range_above_max() {
        let err = TabError::StringNumberOutOfRange { value: 13, max: 12 };
        assert_eq!(
            err.to_string(),
            "The string number (13) is too high. The maximum is 12."
        );
    }

    #[test]
    fn open_pitch_out_of_range() {
        let err = TabError::OpenPitchOutOfRange { string: 1, semitones: 8 };
        assert_eq!(
            err.to_string(),
            "Capo offset of 8 semitones on string 1 would push the open pitch out of the supported range."
        );
    }

    #[test]
    fn fret_range_exceeds_pitch_range() {
        let err = TabError::FretRangeExceedsPitchRange {
            open_pitch: "G9".to_owned(),
            playable_frets: 5,
        };
        assert_eq!(
            err.to_string(),
            "Too many frets (5) for string starting at pitch G9. The highest playable pitch is B9."
        );
    }

    #[test]
    fn unplayable_pitches_joins_with_newlines() {
        let err = TabError::UnplayablePitches {
            pitches: vec![
                UnplayablePitch { value: "A1".to_owned(), line: 1 },
                UnplayablePitch { value: "B1".to_owned(), line: 4 },
            ],
        };
        assert_eq!(
            err.to_string(),
            "Pitch A1 on line 1 cannot be played on any strings of the configured guitar.\n\
             Pitch B1 on line 4 cannot be played on any strings of the configured guitar."
        );
    }

    #[test]
    fn num_arrangements_out_of_range() {
        let err = TabError::NumArrangementsOutOfRange { value: 21, max: 20 };
        assert_eq!(err.to_string(), "must be between 1 and 20 inclusive, got 21");
    }

    #[test]
    fn tuning_name_unknown() {
        let err = TabError::TuningNameUnknown { value: "openZ".to_owned() };
        assert_eq!(
            err.to_string(),
            "must be \"standard\" or one of the supported TuningName variants, got \"openZ\""
        );
    }

    #[test]
    fn index_out_of_bounds() {
        let err = TabError::IndexOutOfBounds { index: 99, len: 3 };
        assert_eq!(err.to_string(), "index 99 is out of bounds for set of length 3");
    }
}
```

- [ ] **Step 3: Verify the additive change compiles and all tests still pass**

```bash
cargo test
```

Expected: PASS. New tests run; existing tests unchanged.

- [ ] **Step 4: Commit**

```bash
git add src/error.rs
git commit -m "feat(error): add structured TabError variants and public UnplayablePitch

Adds the eight new TabError variants plus the public UnplayablePitch struct.
The umbrella Guitar/Arrangement/InvalidInput variants are still present;
nothing constructs the new variants yet. Removal of the umbrellas follows
once all call sites cut over."
```

---

## Task 2: Migrate `StringNumber::new` to `TabError`

**Files:**
- Modify: `src/string_number.rs`

`StringNumber::new` is a leaf (no internal callers of its returned error besides anyhow auto-conversion in `create_string_tuning`). Cut over to `Result<Self, TabError>` first.

- [ ] **Step 1: Add a failing test for the typed return**

Add this test inside the existing `#[cfg(test)] mod test_create_string_number` block in `src/string_number.rs`:

```rust
    #[test]
    fn returns_typed_error_for_zero() {
        let err = StringNumber::new(0).unwrap_err();
        match err {
            crate::error::TabError::StringNumberOutOfRange { value, max } => {
                assert_eq!(value, 0);
                assert_eq!(max, 12);
            }
            other => panic!("expected StringNumberOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn returns_typed_error_for_above_max() {
        let err = StringNumber::new(13).unwrap_err();
        match err {
            crate::error::TabError::StringNumberOutOfRange { value, max } => {
                assert_eq!(value, 13);
                assert_eq!(max, 12);
            }
            other => panic!("expected StringNumberOutOfRange, got {other:?}"),
        }
    }
```

- [ ] **Step 2: Run the new tests; confirm they fail**

```bash
cargo test --lib string_number::test_create_string_number::returns_typed_error
```

Expected: FAIL with a type mismatch (`StringNumber::new` returns `anyhow::Error`).

- [ ] **Step 3: Change `StringNumber::new` to return `Result<Self, TabError>`**

Replace the top of `src/string_number.rs` (the `use` line and the `new` method) with:

```rust
use crate::error::TabError;
use std::fmt;

/// A validated guitar string number in the range `1..=12`.
///
/// String numbers follow guitar convention: string 1 is the highest-pitched (thinnest)
/// string, and higher numbers designate lower-pitched strings.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct StringNumber(u8);
impl StringNumber {
    /// Upper bound enforced by [`StringNumber::new`].
    pub const MAX: u8 = 12;

    /// Constructs a `StringNumber` after validating that `string_number` is in `1..=MAX`.
    ///
    /// # Errors
    ///
    /// Returns [`TabError::StringNumberOutOfRange`] if `string_number` is `0` or exceeds
    /// [`StringNumber::MAX`].
    pub fn new(string_number: u8) -> Result<Self, TabError> {
        match string_number {
            0 => Err(TabError::StringNumberOutOfRange { value: 0, max: Self::MAX }),
            1..=Self::MAX => Ok(StringNumber(string_number)),
            _ => Err(TabError::StringNumberOutOfRange { value: string_number, max: Self::MAX }),
        }
    }
```

(Leave the rest of the file unchanged. Note that `MAX = 12` is exposed via the `pub const` rather than a hidden `const` inside `new`, which is purely additive.)

Also update the existing `invalid` test in the same module to assert on `TabError` instead of `anyhow::Error`. Replace it with:

```rust
    #[test]
    fn invalid() {
        for n in [0u8, 13, 100, 255] {
            assert!(StringNumber::new(n).is_err(), "n={n} must be Err");
        }
    }
```

- [ ] **Step 4: Run the test suite; confirm green**

```bash
cargo test
```

Expected: PASS. The `anyhow::Error` -> `TabError` change ripples to `create_string_tuning` callers via the `?` operator's auto-`From<TabError> for anyhow::Error` (anyhow accepts any `std::error::Error`).

- [ ] **Step 5: Commit**

```bash
git add src/string_number.rs
git commit -m "refactor(string_number)!: return TabError instead of anyhow::Error

StringNumber::new now returns Result<Self, TabError> with the new
StringNumberOutOfRange variant. Also lifts the MAX constant to pub const
StringNumber::MAX so callers can introspect the bound."
```

---

## Task 3: Migrate `Pitch::plus_offset` to `Option<Pitch>`

**Files:**
- Modify: `src/pitch.rs`

`Pitch::plus_offset` has no contextual information to populate a `TabError` variant -- the meaningful context (which string, which capo) lives in the caller. Shrink it to `Option<Pitch>`.

- [ ] **Step 1: Add a test that asserts the new return type**

Add this test to `src/pitch.rs` (find an existing `#[cfg(test)]` block in the file and add it, or add a new module at the end):

```rust
#[cfg(test)]
mod test_plus_offset {
    use super::*;

    #[test]
    fn within_range_returns_some() {
        assert_eq!(Pitch::C0.plus_offset(2), Some(Pitch::D0));
        assert_eq!(Pitch::E4.plus_offset(0), Some(Pitch::E4));
    }

    #[test]
    fn negative_overflow_returns_none() {
        assert_eq!(Pitch::C0.plus_offset(-1), None);
    }

    #[test]
    fn positive_overflow_returns_none() {
        assert_eq!(Pitch::B9.plus_offset(1), None);
    }
}
```

- [ ] **Step 2: Run; confirm failure**

```bash
cargo test --lib pitch::test_plus_offset
```

Expected: FAIL (return type is `Result<Pitch>`, not `Option<Pitch>`).

- [ ] **Step 3: Change `plus_offset` to return `Option<Pitch>`**

Replace `src/pitch.rs:459-470` (the `plus_offset` method body) with:

```rust
    /// Returns the pitch `offset` semitones above (or below, for negative `offset`) this pitch,
    /// or `None` if the result would fall outside the supported `Pitch` range.
    #[must_use]
    pub fn plus_offset(&self, offset: i16) -> Option<Pitch> {
        let new_index = self.index() as i16 + offset;
        if new_index < 0 {
            return None;
        }
        Pitch::from_repr(new_index as usize)
    }
}
```

Also remove the now-unused `use anyhow::{anyhow, Result};` line at the top of `src/pitch.rs` (line 1). Verify with `grep -n anyhow src/pitch.rs` that no anyhow uses remain in that file.

- [ ] **Step 4: Update internal callers of `plus_offset`**

Two call sites need updating:

In `src/guitar.rs`, the `Guitar::new` capo loop currently uses `.context(...)?` on `plus_offset`. Replace `src/guitar.rs:124-132` with:

```rust
        let adjusted_tuning = tuning
            .into_iter()
            .map(|(string_num, pitch)| -> Result<_, TabError> {
                let adjusted = pitch.plus_offset(capo as i16).ok_or(
                    TabError::OpenPitchOutOfRange { string: string_num.get(), semitones: capo as i16 },
                )?;
                Ok((string_num, adjusted))
            })
            .collect::<Result<BTreeMap<_, _>, TabError>>()?;
```

This requires `use crate::error::TabError;` at the top of `src/guitar.rs` (added in Task 5; for now, qualify as `crate::error::TabError` or do nothing -- Step 5 below will check the build).

In `src/parser.rs::create_string_tuning_offset` (line ~152-162), `plus_offset` is called with `.expect("BUG: Tuning pitch offset should be valid")`. Change to:

```rust
        .map(|(std_tuning_pitch, offset)| {
            std_tuning_pitch
                .plus_offset(offset as i16)
                .expect("BUG: Tuning pitch offset should be valid")
        })
```

The `.expect` becomes `.expect()` on the `Option`'s `.unwrap_or_else` equivalent. Since the input is `[i8; 6]` against the standard 6-string tuning (E4 high to E2 low), `plus_offset` over the configured presets never falls out of range; the BUG label remains accurate. The signature change is purely `Result::expect` -> `Option::expect`, which is identical in call shape.

- [ ] **Step 5: Run the test suite; confirm green**

```bash
cargo test
```

Expected: PASS. If the build fails in `guitar.rs` because of the `crate::error::TabError` reference, that means Task 5 needs to land first. Skip Step 5 of this task and continue to Task 4 / Task 5; come back and run `cargo test` after Task 5 completes.

- [ ] **Step 6: Commit**

```bash
git add src/pitch.rs src/guitar.rs src/parser.rs
git commit -m "refactor(pitch)!: Pitch::plus_offset returns Option<Pitch>

The previous Result<Pitch, anyhow::Error> carried no context the caller
could not produce. Guitar::new now wraps None into the typed
OpenPitchOutOfRange variant with the string number and offset; the
bench-only create_string_tuning_offset path uses .expect with the same
BUG-condition rationale."
```

---

## Task 4: Migrate `check_fret_number`, `check_capo_number`, `create_string_range`, and add `CapoExceedsFrets` check

**Files:**
- Modify: `src/guitar.rs`

- [ ] **Step 1: Add failing tests for the typed variants and the new cross-check**

Add these tests to the existing `mod test_check_fret_number` and `mod test_check_capo_number` in `src/guitar.rs`, plus a new test for the cross-check:

In `mod test_check_fret_number`:

```rust
    #[test]
    fn invalid_returns_typed_error() {
        let err = check_fret_number(31).unwrap_err();
        match err {
            crate::error::TabError::NumFretsTooHigh { num_frets, max } => {
                assert_eq!(num_frets, 31);
                assert_eq!(max, 30);
            }
            other => panic!("expected NumFretsTooHigh, got {other:?}"),
        }
    }
```

In `mod test_check_capo_number`:

```rust
    #[test]
    fn invalid_returns_typed_error() {
        let err = check_capo_number(9).unwrap_err();
        match err {
            crate::error::TabError::CapoTooHigh { capo, max } => {
                assert_eq!(capo, 9);
                assert_eq!(max, 8);
            }
            other => panic!("expected CapoTooHigh, got {other:?}"),
        }
    }
```

In the existing `mod test_create_guitar`:

```rust
    #[test]
    fn capo_exceeds_num_frets_returns_typed_error() {
        let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES).unwrap();
        let err = Guitar::new(tuning, 2, 4).unwrap_err();
        match err {
            crate::error::TabError::CapoExceedsFrets { capo, num_frets } => {
                assert_eq!(capo, 4);
                assert_eq!(num_frets, 2);
            }
            other => panic!("expected CapoExceedsFrets, got {other:?}"),
        }
    }
```

In `mod test_create_string_range`, replace the existing `invalid` test (line ~668) with:

```rust
    #[test]
    fn invalid_returns_typed_error() {
        let err = create_string_range(&Pitch::G9, 5).unwrap_err();
        match err {
            crate::error::TabError::FretRangeExceedsPitchRange { open_pitch, playable_frets } => {
                assert_eq!(open_pitch, "G9");
                assert_eq!(playable_frets, 5);
            }
            other => panic!("expected FretRangeExceedsPitchRange, got {other:?}"),
        }
    }
```

- [ ] **Step 2: Run; confirm the new tests fail**

```bash
cargo test --lib guitar::test_check_fret_number guitar::test_check_capo_number guitar::test_create_guitar::capo_exceeds_num_frets guitar::test_create_string_range
```

Expected: FAIL (typed-error tests fail because helpers still return anyhow; the cross-check test panics on underflow in debug or asserts the wrong shape in release).

- [ ] **Step 3: Migrate the helpers and add the cross-check**

Replace the helper definitions in `src/guitar.rs`:

Replace `check_fret_number` (currently `src/guitar.rs:547-556`) with:

```rust
/// Validates that `num_frets` does not exceed [`Guitar::MAX_NUM_FRETS`].
fn check_fret_number(num_frets: u8) -> Result<(), TabError> {
    if num_frets > Guitar::MAX_NUM_FRETS {
        return Err(TabError::NumFretsTooHigh { num_frets, max: Guitar::MAX_NUM_FRETS });
    }
    Ok(())
}
```

Replace `check_capo_number` (currently `src/guitar.rs:577-585`) with:

```rust
/// Validates that `capo` does not exceed [`Guitar::MAX_CAPO`].
fn check_capo_number(capo: u8) -> Result<(), TabError> {
    if capo > Guitar::MAX_CAPO {
        return Err(TabError::CapoTooHigh { capo, max: Guitar::MAX_CAPO });
    }
    Ok(())
}
```

Replace `create_string_range` (currently `src/guitar.rs:615-636`) with:

```rust
/// Generates a vector of pitches representing the range of one string from its open pitch
/// up through `num_frets` semitones.
fn create_string_range(open_string_pitch: &Pitch, num_frets: u8) -> Result<Vec<Pitch>, TabError> {
    let lowest_pitch_index = Pitch::iter().position(|x| &x == open_string_pitch).unwrap();
    let needed = num_frets as usize + 1;

    let string_range: Vec<Pitch> = Pitch::iter()
        .skip(lowest_pitch_index)
        .take(needed)
        .collect();

    if string_range.len() == needed {
        Ok(string_range)
    } else {
        Err(TabError::FretRangeExceedsPitchRange {
            open_pitch: open_string_pitch.to_string(),
            playable_frets: num_frets,
        })
    }
}
```

Above the `impl Guitar` block (around `src/guitar.rs:107`), add the constants and replace `Guitar::new`'s signature + body:

```rust
impl Guitar {
    /// Upper bound on the fret count accepted by [`Guitar::new`].
    pub const MAX_NUM_FRETS: u8 = 30;
    /// Upper bound on the capo position accepted by [`Guitar::new`].
    pub const MAX_CAPO: u8 = 8;

    /// Constructs a validated `Guitar` from a tuning map, fret count, and capo position.
    ///
    /// The capo shifts every open-string pitch up by `capo` semitones and reduces the
    /// effective `num_frets` by the same amount.
    ///
    /// # Errors
    ///
    /// Returns a [`TabError`] variant for any of: fret count above [`Guitar::MAX_NUM_FRETS`],
    /// capo above [`Guitar::MAX_CAPO`], `capo > num_frets`, an open-string pitch shifted out
    /// of the supported `Pitch` range, or a string range that exceeds the highest pitch (`B9`).
    pub fn new(tuning: BTreeMap<StringNumber, Pitch>, num_frets: u8, capo: u8) -> Result<Self, TabError> {
        check_fret_number(num_frets)?;
        check_capo_number(capo)?;
        if capo > num_frets {
            return Err(TabError::CapoExceedsFrets { capo, num_frets });
        }
        let playable_frets = num_frets - capo;
        let adjusted_tuning = tuning
            .into_iter()
            .map(|(string_num, pitch)| -> Result<_, TabError> {
                let adjusted = pitch.plus_offset(capo as i16).ok_or(
                    TabError::OpenPitchOutOfRange { string: string_num.get(), semitones: capo as i16 },
                )?;
                Ok((string_num, adjusted))
            })
            .collect::<Result<BTreeMap<_, _>, TabError>>()?;

        let mut string_ranges: BTreeMap<StringNumber, Box<[Pitch]>> = BTreeMap::new();
        for (string_number, string_open_pitch) in adjusted_tuning.iter() {
            string_ranges.insert(
                *string_number,
                create_string_range(string_open_pitch, playable_frets)?.into_boxed_slice(),
            );
        }

        let range =
            string_ranges
                .iter()
                .fold(BTreeSet::new(), |mut all_pitches, string_pitches| {
                    all_pitches.extend(string_pitches.1);
                    all_pitches
                });

        Ok(Guitar {
            tuning: adjusted_tuning,
            playable_frets,
            range,
            string_ranges,
        })
    }
}
```

Add `use crate::error::TabError;` to the top of `src/guitar.rs` (alongside the existing uses). Remove `use anyhow::{anyhow, Context, Result};`.

The `create_string_tuning` function (currently `src/guitar.rs:81-87`) still returns `Result<BTreeMap<_, _>>` (anyhow). Update its signature and body to:

```rust
/// Builds a tuning map from a slice of open-string pitches, numbering them from string 1
/// (highest) to string N (lowest).
///
/// # Errors
///
/// Returns [`TabError::StringNumberOutOfRange`] if the input slice has more entries than
/// [`StringNumber::MAX`].
pub fn create_string_tuning(open_string_pitches: &[Pitch]) -> Result<BTreeMap<StringNumber, Pitch>, TabError> {
    open_string_pitches
        .iter()
        .enumerate()
        .map(|(i, p)| StringNumber::new((i + 1) as u8).map(|sn| (sn, *p)))
        .collect()
}
```

Also update `Guitar::default` (currently `src/guitar.rs:100-106`) to drop the `anyhow::Result` return type implication:

```rust
impl Default for Guitar {
    fn default() -> Guitar {
        let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES)
            .expect("BUG: standard tuning has 6 strings");
        Guitar::new(tuning, 18, 0).expect("BUG: Default guitar should be valid")
    }
}
```

(Body is unchanged; `.expect` works against `Result<_, TabError>` identically.)

Any remaining `valid_simple`, `valid_simple_capo`, `valid_normal` tests in `mod test_create_guitar` use `Result<(), anyhow::Error>` (via `-> Result<()>`). Change their return type to `Result<(), TabError>`:

```rust
    fn valid_simple() -> Result<(), TabError> {
```

(Same for the other three valid-case tests.) The `?` operator inside still works.

Same change applies to `mod test_create_string_range::valid` and `mod test_generate_pitch_fingering` tests that currently use `Result<(), anyhow::Error>`. Replace their return type with `Result<(), TabError>`.

- [ ] **Step 4: Run the test suite**

```bash
cargo test
```

Expected: PASS. The Task 3 commit may have left `src/guitar.rs` in a state where `TabError` was referenced before the `use` line; this task's `use crate::error::TabError;` addition fixes that.

- [ ] **Step 5: Commit**

```bash
git add src/guitar.rs
git commit -m "refactor(guitar)!: typed errors and CapoExceedsFrets cross-check

Guitar::new, create_string_tuning, check_fret_number, check_capo_number,
and create_string_range all return Result<_, TabError>. Adds the
CapoExceedsFrets cross-check before the playable_frets subtraction
(previously underflowed for capo > num_frets). Surfaces MAX_NUM_FRETS
and MAX_CAPO as pub const on Guitar."
```

---

## Task 5: Migrate `arrangement.rs` internals to `TabError`

**Files:**
- Modify: `src/arrangement.rs`

`UnplayablePitch` is now public in `error.rs` (Task 1). Drop the private duplicate in `arrangement.rs`. The two internal functions returning `Result<_, Arc<anyhow::Error>>` (`create_arrangements`, `generate_fingering_combos`) and `validate_fingerings` (`Result<_, anyhow::Error>`) all migrate to `TabError`.

- [ ] **Step 1: Add a test that asserts the new typed return**

Add this test in the existing `mod test_create_arrangements` in `src/arrangement.rs` (or its nearest equivalent; search for the test that pins the "Pitch A1 on line 1 cannot be played" message at around line 740):

```rust
    #[test]
    fn unreachable_pitch_returns_unplayable_pitches_variant() {
        let tuning = create_string_tuning(&STD_6_STRING_TUNING_OPEN_PITCHES).unwrap();
        let guitar = Guitar::new(tuning, 18, 0).unwrap();
        let lines = parse_lines("A1".to_owned()).unwrap();
        let n = crate::NumArrangements::try_new(1).unwrap();
        let err = create_arrangements(guitar, lines, n, None).unwrap_err();
        let inner = std::sync::Arc::try_unwrap(err).unwrap_or_else(|arc| (*arc).clone());
        match inner {
            crate::error::TabError::UnplayablePitches { pitches } => {
                assert_eq!(pitches.len(), 1);
                assert_eq!(pitches[0].value, "A1");
                assert_eq!(pitches[0].line, 1);
            }
            other => panic!("expected UnplayablePitches, got {other:?}"),
        }
    }
```

(Imports: bring `STD_6_STRING_TUNING_OPEN_PITCHES`, `create_string_tuning`, `parse_lines`, `Guitar` into scope through the test module's `use super::*;` plus `use crate::{...};` as needed. If the existing test for the pitch-A1 string match has imports already wired, add this test next to it.)

- [ ] **Step 2: Run; confirm failure**

```bash
cargo test --lib arrangement::test_create_arrangements::unreachable_pitch_returns_unplayable_pitches_variant
```

Expected: FAIL (`create_arrangements` returns `Arc<anyhow::Error>` today).

- [ ] **Step 3: Remove the private `UnplayablePitch` and migrate the internal functions**

Delete the private `UnplayablePitch` struct (currently `src/arrangement.rs:14-18`):

```rust
// DELETE:
#[derive(Debug)]
struct UnplayablePitch {
    value: String,
    line_number: u16,
}
```

Replace its uses with the public `crate::error::UnplayablePitch`. At the top of `src/arrangement.rs`, add:

```rust
use crate::error::{TabError, UnplayablePitch};
```

(And remove `use anyhow::{anyhow, Result};`. If `Result` from anyhow is used elsewhere as a bare alias, qualify or alias as `std::result::Result`.)

In `validate_fingerings` (currently `src/arrangement.rs:619-654`), update:

- Return type from `Result<Vec<Line<BeatVec<PitchVec<PitchFingering>>>>>` to `Result<Vec<Line<BeatVec<PitchVec<PitchFingering>>>>, TabError>`.
- Field name on the construction: `line_number` becomes `line` and changes type from `u16` to `u32`. Replace:

```rust
                            impossible_pitches.push(UnplayablePitch {
                                value: format!("{beat_pitch:?}"),
                                line_number: (beat_index as u16) + 1,
                            })
```

with:

```rust
                            impossible_pitches.push(UnplayablePitch {
                                value: format!("{beat_pitch:?}"),
                                line: (beat_index as u32) + 1,
                            })
```

- Replace the trailing `Err(anyhow!(error_msg))` block with:

```rust
    if !impossible_pitches.is_empty() {
        return Err(TabError::UnplayablePitches { pitches: impossible_pitches });
    }
```

(Drop the entire `let error_msg = ... join("\n");` block above it. The Display impl on `UnplayablePitches` reproduces the joined string.)

In `generate_fingering_combos` (currently `src/arrangement.rs:751-769`):

- Return type from `Result<Vec<BeatVec<PitchFingering>>, Arc<anyhow::Error>>` to `Result<Vec<BeatVec<PitchFingering>>, Arc<TabError>>`.
- The internal anyhow `bail!` (the only one is `"generate_fingering_combos called with empty input"`) is a BUG-condition guard. The spec keeps this as a typed error since the function is reached from `create_arrangements`. Wrap into a new generic variant? No -- it is unreachable from external callers. Instead, replace `Arc::new(anyhow!(...))` with `Arc::new(TabError::UnplayablePitches { pitches: vec![] })` only if reached. **Wait -- this is wrong: the BUG-condition guard should panic, not error.** Replace the body with:

```rust
fn generate_fingering_combos(
    beat_fingerings_per_pitch: &[Vec<PitchFingering>],
) -> Vec<BeatVec<PitchFingering>> {
    assert!(
        !beat_fingerings_per_pitch.is_empty(),
        "BUG: generate_fingering_combos called with empty input"
    );

    beat_fingerings_per_pitch
        .iter()
        .multi_cartesian_product()
        .map(|combo| combo.into_iter().copied().collect::<Vec<PitchFingering>>())
        .filter(|x| no_duplicate_strings(x))
        .collect()
}
```

(The function returns `Vec<...>` directly; the only caller in `create_arrangements` no longer needs `?` on this call. Update the call site accordingly.)

In `create_arrangements` (currently `src/arrangement.rs:308-end`):

- Return type from `Result<Vec<Arrangement>, Arc<anyhow::Error>>` to `Result<Vec<Arrangement>, Arc<TabError>>`.
- The `Result<BeatVec<Node>, Arc<anyhow::Error>>` annotation at `src/arrangement.rs:352` becomes `Result<BeatVec<Node>, Arc<TabError>>`.
- The `Err(Arc::new(anyhow!("No arrangements could be calculated.")))` at line 391 becomes a panic. This is a BUG condition reachable only if the pathfinding library returns no result for non-empty input (which the invariants prevent). Replace with:

```rust
            panic!("BUG: pathfinding produced no path despite non-empty playable input");
```

- The `validate_fingerings` call (around line 380, search for it) currently does:

```rust
        let fingerings_per_beat = validate_fingerings(&guitar, &playable_lines)?;
```

This works because `?` converts `TabError` -> `Arc<TabError>` via auto-from. But Rust does not auto-wrap into `Arc`. Replace with:

```rust
        let fingerings_per_beat = validate_fingerings(&guitar, &playable_lines).map_err(Arc::new)?;
```

The `generate_fingering_combos` call (around line 360) drops the `?`:

```rust
        // BEFORE:
        let beat_fingering_candidates = generate_fingering_combos(&beat_fingerings_per_pitch)?;
        // AFTER:
        let beat_fingering_candidates = generate_fingering_combos(&beat_fingerings_per_pitch);
```

Also update the proptests in the same file that pattern-match the error string. Search for `create_arrangements rejected input:` (around `src/arrangement.rs:1649, 1694, 1718, 1737, 1750`). The format string `format!("create_arrangements rejected input: {e}")` continues to work because `TabError` impls `Display` (Task 1). No change needed there.

The two integration tests pinning the legacy joined string (search for `cannot be played on any strings of the configured guitar` in the test bodies around lines 722 and 740-743) need to switch from string assertions to variant assertions. Replace:

```rust
        // OLD:
        let err = create_arrangements(...).unwrap_err();
        assert_eq!(err.to_string(), "Pitch B9 on line 1 cannot be played on any strings of the configured guitar.");
```

with:

```rust
        let err = create_arrangements(...).unwrap_err();
        let inner = std::sync::Arc::try_unwrap(err).unwrap_or_else(|arc| (*arc).clone());
        match inner {
            crate::error::TabError::UnplayablePitches { pitches } => {
                assert_eq!(pitches.len(), 1);
                assert_eq!(pitches[0].value, "B9");
                assert_eq!(pitches[0].line, 1);
            }
            other => panic!("expected UnplayablePitches, got {other:?}"),
        }
```

Apply the same pattern to the multi-pitch test (asserts three unplayable pitches).

- [ ] **Step 4: Run the suite; confirm green**

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/arrangement.rs
git commit -m "refactor(arrangement)!: typed errors and unified UnplayablePitch

create_arrangements returns Result<_, Arc<TabError>> with the new
UnplayablePitches variant. Removes the private UnplayablePitch duplicate
in favor of the public struct in error.rs. BUG-condition guards
('generate_fingering_combos called with empty input', 'no path despite
non-empty playable input') become panics rather than typed errors;
neither is reachable from external callers."
```

---

## Task 6: Migrate `parser.rs::parse_tuning` and drop empty-string-as-standard

**Files:**
- Modify: `src/parser.rs`

- [ ] **Step 1: Update the failing-test fixtures**

In `mod test_parse_tuning` (currently `src/parser.rs:88-137`), modify these tests:

Replace `empty_string_returns_standard` with:

```rust
    #[test]
    fn empty_string_returns_tuning_name_unknown() {
        let err = parse_tuning("").unwrap_err();
        match err {
            TabError::TuningNameUnknown { value } => assert_eq!(value, ""),
            other => panic!("expected TuningNameUnknown, got {other:?}"),
        }
    }
```

Replace `unrecognized_name_returns_invalid_input_error` with:

```rust
    #[test]
    fn unrecognized_name_returns_tuning_name_unknown() {
        let err = parse_tuning("opan G").unwrap_err();
        match err {
            TabError::TuningNameUnknown { value } => assert_eq!(value, "opan G"),
            other => panic!("expected TuningNameUnknown, got {other:?}"),
        }
    }
```

Keep `standard_tuning_returns_zero_offsets`, `standard_is_case_insensitive`, and `non_standard_tunings` unchanged.

- [ ] **Step 2: Run; confirm the new tests fail**

```bash
cargo test --lib parser::test_parse_tuning
```

Expected: FAIL (`parse_tuning("")` currently returns `Ok([0; 6])` and the unrecognized test expects `InvalidInput` not `TuningNameUnknown`).

- [ ] **Step 3: Update `parse_tuning`**

Replace `src/parser.rs:66-87` (`parse_tuning` body) with:

```rust
pub fn parse_tuning(tuning_name: &str) -> Result<[i8; 6], crate::error::TabError> {
    match TuningName::from_str(tuning_name) {
        Ok(TuningName::OpenG) => Ok([-2, 0, 0, 0, -2, -2]),
        Ok(TuningName::OpenD) => Ok([-2, 0, 0, -1, -2, -2]),
        Ok(TuningName::C6) => Ok([-4, 0, -2, 0, 1, 0]),
        Ok(TuningName::Dsus4) => Ok([-2, 0, 0, 0, -2, -2]),
        Ok(TuningName::DropD) => Ok([-2, 0, 0, 0, 0, 0]),
        Ok(TuningName::DropC) => Ok([-4, -2, -2, -2, -2, -2]),
        Ok(TuningName::OpenC) => Ok([-4, -2, -2, 0, 1, 0]),
        Ok(TuningName::DropB) => Ok([-5, -3, -3, -3, -3, -3]),
        Ok(TuningName::OpenE) => Ok([0, -2, -2, -2, 0, 0]),
        Err(_) if tuning_name.eq_ignore_ascii_case("standard") => Ok([0; 6]),
        Err(_) => Err(crate::error::TabError::TuningNameUnknown {
            value: tuning_name.to_owned(),
        }),
    }
}
```

The only changes from current: drop `tuning_name.is_empty() ||` from the second-to-last arm, and switch the final arm's error construction from `TabError::InvalidInput { field, message }` to `TabError::TuningNameUnknown { value }`.

- [ ] **Step 4: Run the suite**

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/parser.rs
git commit -m "refactor(parser)!: TuningNameUnknown variant; drop empty-string-as-standard

parse_tuning now returns TabError::TuningNameUnknown { value } instead of
the umbrella TabError::InvalidInput. The empty-string-as-standard
fallback is removed; callers wanting standard tuning must pass
\"standard\" (case-insensitive) explicitly. The non-empty case-insensitive
\"standard\" literal is preserved."
```

---

## Task 7: Migrate `lib.rs` boundary helpers

**Files:**
- Modify: `src/lib.rs`

`NumArrangements::try_new` returns `TabError::InvalidInput` today; switch to `NumArrangementsOutOfRange`. `out_of_bounds_error` returns `TabError::InvalidInput` today; switch to `IndexOutOfBounds`. Also drop the `.map_err(|e| TabError::Guitar { message: e.to_string() })` wrappers in `generate_arrangements` since the underlying errors are already typed.

- [ ] **Step 1: Update the tests that pin the old variant shapes**

In `mod test_num_arrangements` (currently `src/lib.rs:629-668`), replace:

```rust
    #[test]
    fn try_new_rejects_zero_with_unified_message() {
        let err = NumArrangements::try_new(0).unwrap_err();
        match err {
            TabError::InvalidInput { field, message } => {
                assert_eq!(field, "numArrangements");
                assert_eq!(message, "must be between 1 and 20 inclusive, got 0");
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }
```

with:

```rust
    #[test]
    fn try_new_rejects_zero_with_typed_variant() {
        let err = NumArrangements::try_new(0).unwrap_err();
        match err {
            TabError::NumArrangementsOutOfRange { value, max } => {
                assert_eq!(value, 0);
                assert_eq!(max, 20);
            }
            other => panic!("expected NumArrangementsOutOfRange, got {other:?}"),
        }
    }
```

Apply the same shape change to `try_new_rejects_above_max_with_unified_message` (rename to `..._with_typed_variant` and pattern-match on `NumArrangementsOutOfRange { value: 21, max: 20 }`).

In `mod test_generate_arrangements_and_render` (currently `src/lib.rs:312-485`), the tests `num_arrangements_zero_is_invalid`, `num_arrangements_above_cap_is_invalid`, `invalid_guitar_config_returns_guitar_error`, and `unreachable_pitch_returns_arrangement_error` all match on `TabError::InvalidInput { field, .. }` or `TabError::Guitar { message }` or `TabError::Arrangement { message }`. Update each to match the new variants:

- `num_arrangements_zero_is_invalid`: match `TabError::NumArrangementsOutOfRange { value: 0, .. }`.
- `num_arrangements_above_cap_is_invalid`: match `TabError::NumArrangementsOutOfRange { value: 21, max: 20 }`.
- `invalid_guitar_config_returns_guitar_error`: rename to `..._returns_num_frets_too_high` and match `TabError::NumFretsTooHigh { num_frets: 31, max: 30 }`. (Drop the prose-string match.)
- `unreachable_pitch_returns_arrangement_error`: rename to `..._returns_unplayable_pitches` and match `TabError::UnplayablePitches { pitches }` with `pitches[0].value == "A1"` and `pitches[0].line == 1`.

In `mod test_boundary_types`, update every `TabError::InvalidInput { field, .. } => assert_eq!(field, "index")` to `TabError::IndexOutOfBounds { .. } => ()`. There are five such test bodies (search for `field, "index"`).

- [ ] **Step 2: Run; confirm the new tests fail**

```bash
cargo test --lib
```

Expected: FAIL on each rewritten test (existing `try_new` still produces `InvalidInput`).

- [ ] **Step 3: Update `NumArrangements::try_new` and `out_of_bounds_error`**

Replace `NumArrangements::try_new` body (currently `src/lib.rs:112-121`) with:

```rust
    pub fn try_new(n: u8) -> Result<Self, TabError> {
        if n == 0 || n > Self::MAX {
            return Err(TabError::NumArrangementsOutOfRange { value: n, max: Self::MAX });
        }
        let nz = NonZeroU8::new(n).expect("BUG: n != 0 verified above");
        Ok(Self(nz))
    }
```

Replace `out_of_bounds_error` (currently `src/lib.rs:234-239`) with:

```rust
fn out_of_bounds_error(index: usize, len: usize) -> TabError {
    TabError::IndexOutOfBounds { index, len }
}
```

In `generate_arrangements` (currently `src/lib.rs:264-309`), update the two `.map_err(...)` calls:

- The `guitar` construction (around line 293-294):

```rust
    // BEFORE:
    let guitar = Guitar::new(tuning, tab_input.guitar_num_frets, tab_input.guitar_capo)
        .map_err(|e| TabError::Guitar { message: e.to_string() })?;

    // AFTER:
    let guitar = Guitar::new(tuning, tab_input.guitar_num_frets, tab_input.guitar_capo)?;
```

- The `arrangements` construction (around line 296-302):

```rust
    // BEFORE:
    let arrangements = arrangement::create_arrangements(
        guitar.clone(),
        input_lines,
        num_arrangements,
        tab_input.max_fret_span_filter,
    )
    .map_err(|e| TabError::Arrangement { message: e.to_string() })?;

    // AFTER:
    let arrangements = arrangement::create_arrangements(
        guitar.clone(),
        input_lines,
        num_arrangements,
        tab_input.max_fret_span_filter,
    )
    .map_err(|arc| std::sync::Arc::try_unwrap(arc).unwrap_or_else(|a| (*a).clone()))?;
```

- [ ] **Step 4: Run the suite**

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs
git commit -m "refactor(lib)!: typed boundary errors

NumArrangements::try_new returns NumArrangementsOutOfRange; the
out_of_bounds_error helper returns IndexOutOfBounds. generate_arrangements
propagates the underlying TabError directly instead of wrapping with
.to_string() into the umbrella Guitar/Arrangement variants. Tests update
to match the new variants."
```

---

## Task 8: Delete the umbrella variants

**Files:**
- Modify: `src/error.rs`

Nothing constructs `TabError::Guitar`, `TabError::Arrangement`, or `TabError::InvalidInput` any more (verify with `cargo build` after removal). Delete them and their Display arms.

- [ ] **Step 1: Confirm no constructors remain**

```bash
grep -rn "TabError::Guitar\|TabError::Arrangement\|TabError::InvalidInput" src/ tests/ examples/ benches/
```

Expected: zero hits. If any hits appear, fix them before proceeding (they should have been migrated in Tasks 5-7).

- [ ] **Step 2: Remove the three umbrella variants from `src/error.rs`**

In the `pub enum TabError { ... }` block, delete:

```rust
    Guitar { message: String },
    Arrangement { message: String },
    InvalidInput { field: String, message: String },
```

In the Display match arms (added in Task 1), delete:

```rust
            TabError::Guitar { message } => write!(f, "{message}"),
            TabError::Arrangement { message } => write!(f, "{message}"),
            TabError::InvalidInput { field, message } => {
                write!(f, "invalid input for `{field}`: {message}")
            }
```

Remove the two existing tests that target the deleted variants. In `mod test_tab_error_display`, delete:

```rust
    #[test]
    fn invalid_input_includes_field_name() { ... }
```

(Keep `parse_variant_joins_errors_with_newlines`.)

- [ ] **Step 3: Run the suite**

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/error.rs
git commit -m "refactor(error)!: remove Guitar/Arrangement/InvalidInput umbrella variants

All concrete variants now constructed directly at the throw site. The
flat TabError shape is the final 2.0.0 wire surface."
```

---

## Task 9: Update `examples/advanced.rs`

**Files:**
- Modify: `examples/advanced.rs`

The example uses `Guitar::new`, `create_string_tuning`, `parse_lines`, `create_arrangements`, `render_tab`. The signatures of the first two changed (anyhow -> TabError). The example also has explicit error handling on `parse_lines` and `create_arrangements` whose Err types changed shape.

- [ ] **Step 1: Update the imports and Err matching**

Inspect the current Err handling:

```bash
sed -n '1,80p' examples/advanced.rs
```

The example's `match parse_lines(...)` and `match create_arrangements(...)` arms format the error as a string. The `TabError` Display impl preserves the legacy strings (Task 1), so the prose output stays correct.

Update the imports if anyhow is referenced:

```bash
grep -n "anyhow" examples/advanced.rs
```

If `anyhow::Error`, `anyhow::Result`, or `anyhow::Context` appear in `examples/advanced.rs`, replace with `Result<_, guitar_tab_generator::TabError>` or drop the type annotation.

The `Arc::try_unwrap(arc).unwrap_or_else(|arc| (*arc).clone())` pattern still applies (memoize still wraps in `Arc`); only the inner type changes from `anyhow::Error` to `TabError`.

- [ ] **Step 2: Run the example**

```bash
cargo run --example advanced
```

Expected: prints the rendered tab without error. If a compile error appears, the most likely culprit is an `anyhow::Error` type annotation that needs to become `TabError`.

- [ ] **Step 3: Commit**

```bash
git add examples/advanced.rs
git commit -m "refactor(examples)!: advanced.rs adapts to typed TabError surface

Replaces anyhow type annotations with TabError. Error message strings
are preserved by the TabError Display impl."
```

---

## Task 10: Update `tests/integration_public_surface.rs`

**Files:**
- Modify: `tests/integration_public_surface.rs`

The canary test pattern-matches on `TabError::Guitar`, `TabError::Arrangement`, and `TabError::InvalidInput` in several places. Each match arm switches to the new variants. Also add a per-variant smoke test that exercises each new error condition through the WASM boundary entry point (`generate_arrangements`).

- [ ] **Step 1: Update the existing variant matches**

```bash
grep -n "TabError::" tests/integration_public_surface.rs
```

Update each `TabError::Guitar { message } => ...` to match the specific variant the test triggers. Same for `TabError::Arrangement` and `TabError::InvalidInput`. Use the variant table in `.scratch/2.0.0-final-pass/PRD.md` (the "Variant-to-callsite mapping" section) to pick the right replacement.

- [ ] **Step 2: Add boundary smoke tests for each new variant**

Add this test module at the end of `tests/integration_public_surface.rs`:

```rust
#[cfg(test)]
mod boundary_variant_smoke {
    use guitar_tab_generator::{generate_arrangements, TabError, TabInput};

    fn input(num_frets: u8, capo: u8, num_arrangements: u8, tuning: &str, input: &str) -> TabInput {
        TabInput {
            input: input.to_owned(),
            tuning_name: tuning.to_owned(),
            guitar_num_frets: num_frets,
            guitar_capo: capo,
            num_arrangements,
            max_fret_span_filter: None,
        }
    }

    #[test]
    fn num_frets_too_high() {
        let err = generate_arrangements(input(31, 0, 1, "standard", "E2")).unwrap_err();
        assert!(matches!(err, TabError::NumFretsTooHigh { num_frets: 31, max: 30 }), "got {err:?}");
    }

    #[test]
    fn capo_too_high() {
        let err = generate_arrangements(input(18, 9, 1, "standard", "E2")).unwrap_err();
        assert!(matches!(err, TabError::CapoTooHigh { capo: 9, max: 8 }), "got {err:?}");
    }

    #[test]
    fn capo_exceeds_frets() {
        let err = generate_arrangements(input(2, 4, 1, "standard", "E2")).unwrap_err();
        assert!(matches!(err, TabError::CapoExceedsFrets { capo: 4, num_frets: 2 }), "got {err:?}");
    }

    #[test]
    fn num_arrangements_out_of_range() {
        let err = generate_arrangements(input(18, 0, 0, "standard", "E2")).unwrap_err();
        assert!(matches!(err, TabError::NumArrangementsOutOfRange { value: 0, max: 20 }), "got {err:?}");
    }

    #[test]
    fn tuning_name_unknown_empty_string() {
        let err = generate_arrangements(input(18, 0, 1, "", "E2")).unwrap_err();
        match err {
            TabError::TuningNameUnknown { value } => assert_eq!(value, ""),
            other => panic!("expected TuningNameUnknown, got {other:?}"),
        }
    }

    #[test]
    fn tuning_name_unknown_garbage() {
        let err = generate_arrangements(input(18, 0, 1, "openZ", "E2")).unwrap_err();
        match err {
            TabError::TuningNameUnknown { value } => assert_eq!(value, "openZ"),
            other => panic!("expected TuningNameUnknown, got {other:?}"),
        }
    }

    #[test]
    fn parse_error() {
        let err = generate_arrangements(input(18, 0, 1, "standard", "E2\n???")).unwrap_err();
        match err {
            TabError::Parse { errors } => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].line, 2);
                assert_eq!(errors[0].text, "???");
            }
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn unplayable_pitches() {
        let err = generate_arrangements(input(18, 0, 1, "standard", "A1")).unwrap_err();
        match err {
            TabError::UnplayablePitches { pitches } => {
                assert_eq!(pitches.len(), 1);
                assert_eq!(pitches[0].value, "A1");
                assert_eq!(pitches[0].line, 1);
            }
            other => panic!("expected UnplayablePitches, got {other:?}"),
        }
    }

    #[test]
    fn index_out_of_bounds() {
        let set = generate_arrangements(input(18, 0, 1, "standard", "E2")).unwrap();
        let err = set.render(99, 30, 1, None).unwrap_err();
        assert!(matches!(err, TabError::IndexOutOfBounds { index: 99, len: 1 }), "got {err:?}");
    }
}
```

(`OpenPitchOutOfRange` and `FretRangeExceedsPitchRange` are hard to trigger through `generate_arrangements` because the standard 6-string tuning at max capo (8) stays in range and the highest standard open string (E4) at max frets (30) only reaches `B6`. Skip the boundary smoke for those two; the unit tests in `src/guitar.rs` already cover them.)

- [ ] **Step 3: Run the integration suite**

```bash
cargo test --test integration_public_surface
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add tests/integration_public_surface.rs
git commit -m "test(integration): cover every new TabError variant at the boundary

Updates existing variant-match arms for the flat TabError shape and adds
a per-variant smoke test through generate_arrangements.
OpenPitchOutOfRange and FretRangeExceedsPitchRange are unreachable
through the boundary with current presets and are covered by the unit
tests in src/guitar.rs."
```

---

## Task 11: Drop `anyhow` from `Cargo.toml` if unused

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`

- [ ] **Step 1: Verify no direct uses remain**

```bash
grep -rn "anyhow" src/ tests/ examples/ benches/
```

Expected: zero hits. If any remain, resolve them before continuing.

- [ ] **Step 2: Check the dep tree**

```bash
cargo tree -i anyhow
```

If the only consumer listed is `guitar-tab-generator` itself (which uses none after Step 1) plus possibly a transitive bring-in from a dev-dep (criterion may pull anyhow), the direct dep can be dropped.

- [ ] **Step 3: Remove the dep**

In `Cargo.toml`, delete this line under `[dependencies]`:

```toml
anyhow = "1.0.100"
```

- [ ] **Step 4: Rebuild and run the suite**

```bash
cargo build
cargo test
```

Expected: PASS. The build succeeds because no source uses anyhow directly.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: drop anyhow direct dep

All public Rust signatures now return TabError; no internal source uses
anyhow. Removes the direct dependency. If criterion or another dev-dep
pulls anyhow transitively, it stays in the lockfile but no longer in
the published crate's dep tree."
```

---

## Task 12: Add ADR-0007 for flat TabError variants

**Files:**
- Create: `docs/adr/0007-flat-taberror-variants.md`

- [ ] **Step 1: Read the existing ADR style**

```bash
cat docs/adr/0002-tab-error-discriminated-union.md
```

Note the structure: Context, Decision, Consequences. Match it.

- [ ] **Step 2: Write the ADR**

Write `docs/adr/0007-flat-taberror-variants.md` with this content:

```markdown
# 0007: Flat TabError variants

Status: accepted
Date: 2026-05-20

## Context

The 2.0.0 preview surface kept three umbrella TabError variants from the
original 1.x shape: `Guitar { message }`, `Arrangement { message }`, and
`InvalidInput { field, message }`. The `message` field was prose, not a
typed wire field, which forced JS callers to fall back to string
inspection for anything more granular than the umbrella kind.
`error.rs:41` documented this explicitly: "treat it like UI strings, not
like a stable wire field."

With 2.0.0 about to ship, the umbrella shape would have to be removed
behind a major bump if structured payloads were added later. The window
to flatten was now.

## Decision

`TabError` is a flat tagged union. Each concrete failure mode is its own
variant with a structured payload. The umbrella variants are removed.
The initial variant set (eleven kinds including the unchanged `Parse`)
captures every error path currently reachable from `generate_arrangements`
and the public Rust API:

- `Parse { errors: Vec<ParseError> }`
- `NumFretsTooHigh { num_frets, max }`
- `CapoTooHigh { capo, max }`
- `CapoExceedsFrets { capo, num_frets }`
- `StringNumberOutOfRange { value, max }`
- `OpenPitchOutOfRange { string, semitones }`
- `FretRangeExceedsPitchRange { open_pitch, playable_frets }`
- `UnplayablePitches { pitches: Vec<UnplayablePitch> }`
- `NumArrangementsOutOfRange { value, max }`
- `TuningNameUnknown { value }`
- `IndexOutOfBounds { index, len }`

The enum stays `#[non_exhaustive]`, so new variants can be added in 2.x
without a major bump. The grouped alternative (Guitar/Arrangement
sub-enums) was rejected because it preserved the umbrella indirection
that this decision exists to remove; the flat shape matches the existing
flat `Parse` variant and the JS-side `switch (err.kind)` pattern the
demo already uses.

## Consequences

- JS callers extend their `switch (err.kind)` blocks. The Tsify wire
  shape is the tagged object only; there is no free-form `message`
  field on the catch-all. UIs that previously rendered `err.message`
  build a per-kind string from the structured fields, or fall through
  to a default handler.
- `UnplayablePitch` becomes a public type. Its prior home as a private
  struct in `arrangement.rs` is gone.
- Removing the umbrella variants required removing `anyhow` from public
  Rust signatures so the typed errors do not get re-wrapped. See ADR-0007's
  companion changes in the 2.0.0 final-pass commits.
- `Pitch::plus_offset` returns `Option<Pitch>` rather than `Result<_, TabError>`
  because the math has no context to populate `OpenPitchOutOfRange` -- the
  caller has the string number and offset, the function does not.
```

- [ ] **Step 3: Commit**

```bash
git add docs/adr/0007-flat-taberror-variants.md
git commit -m "docs(adr): capture flat TabError decision

ADR-0007 records the rationale for collapsing the Guitar/Arrangement/InvalidInput
umbrellas into specific variants, the initial variant set, and the
companion decision to drop anyhow from public Rust signatures."
```

---

## Task 13: Update `MIGRATION.md`

**Files:**
- Modify: `MIGRATION.md`

- [ ] **Step 1: Add the final-pass subsection**

Append this section to `MIGRATION.md` (just before `## See also`):

```markdown
## 2.0.0 final-pass error and validation changes

The 2.0.0 release that shipped from the `v2.0.0` branch carries an
additional pass of breaking changes on top of the WASM surface redesign
above:

### Flat TabError variants

The umbrella variants `Guitar`, `Arrangement`, and `InvalidInput` are
removed. Each concrete failure mode is now its own variant. JS callers
extend their `switch (err.kind)` blocks:

```ts
// Before:
switch (err.kind) {
  case "parse": ...; break;
  case "guitar": showMessage(err.message); break;
  case "arrangement": showMessage(err.message); break;
  case "invalidInput": showField(err.field, err.message); break;
}

// After:
switch (err.kind) {
  case "parse": ...; break;
  case "numFretsTooHigh": showFretLimit(err.numFrets, err.max); break;
  case "capoTooHigh": showCapoLimit(err.capo, err.max); break;
  case "capoExceedsFrets": showCapoVsFrets(err.capo, err.numFrets); break;
  case "stringNumberOutOfRange": ...; break;
  case "openPitchOutOfRange": ...; break;
  case "fretRangeExceedsPitchRange": ...; break;
  case "unplayablePitches": showPitches(err.pitches); break;
  case "numArrangementsOutOfRange": showRangeError(err.value, err.max); break;
  case "tuningNameUnknown": showUnknownTuning(err.value); break;
  case "indexOutOfBounds": showRangeError(err.index, err.len); break;
}
```

`TabError` remains `#[non_exhaustive]`; future 2.x releases may add
variants. Defensive default arms remain a good idea.

### `UnplayablePitch` is now a public type

`TabError::UnplayablePitches { pitches }` carries `Vec<UnplayablePitch>`
with `{ value: string, line: number }` per pitch. Replaces the prose
"Pitch X on line N cannot be played..." string the umbrella `Arrangement`
variant used to carry.

### Anyhow removed from public Rust signatures

`StringNumber::new`, `Guitar::new`, and `create_string_tuning` now return
`Result<_, TabError>` instead of `anyhow::Result`. Direct Rust callers
replace `.context(...)` with pattern-matching on `TabError`.

`Pitch::plus_offset` returns `Option<Pitch>` instead of `anyhow::Result<Pitch>`.
Callers replace `?` with `.ok_or_else(...)` and construct a typed error
themselves.

### Capo cannot exceed `num_frets`

`Guitar::new(tuning, num_frets, capo)` with `capo > num_frets` now
returns `TabError::CapoExceedsFrets { capo, num_frets }`. Previously this
combination underflowed `let playable_frets = num_frets - capo;` and
either panicked in debug or wrapped around to a large `playable_frets`
in release. Callers that supplied a capo position above the fret count
must clamp before calling.

### Empty string no longer means standard tuning

`tuningName: ""` previously fell back to standard tuning. It now returns
`TabError::TuningNameUnknown { value: "" }`. Callers wanting standard
tuning must pass `"standard"` (case-insensitive) explicitly. The
case-insensitive `"standard"` literal continues to work.
```

- [ ] **Step 2: Commit**

```bash
git add MIGRATION.md
git commit -m "docs: 2.0.0 final-pass migration notes

Documents the flat TabError variants, UnplayablePitch promotion,
anyhow removal, CapoExceedsFrets validation, and the dropped
empty-string-as-standard tuning shortcut."
```

---

## Task 14: Update `CHANGELOG.md`

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add bullets under the existing 2.0.0 Breaking changes section**

Append to the existing `### Breaking changes` list in `CHANGELOG.md`:

```markdown
- `TabError` flattened: the umbrella `Guitar { message }`, `Arrangement { message }`, and `InvalidInput { field, message }` variants are removed. Each failure mode now has its own variant with structured payload. See [MIGRATION.md](MIGRATION.md#flat-taberror-variants) for the full mapping and [ADR-0007](docs/adr/0007-flat-taberror-variants.md) for the rationale.
- `UnplayablePitch` is now a public type carried by `TabError::UnplayablePitches`. Replaces the prose error string with structured `{ value, line }` records.
- `StringNumber::new`, `Guitar::new`, and `create_string_tuning` return `Result<_, TabError>` instead of `anyhow::Result`. Direct Rust callers must update error handling.
- `Pitch::plus_offset` returns `Option<Pitch>` instead of `anyhow::Result<Pitch>`. Callers replace `?` with `.ok_or_else(...)`.
- `Guitar::new` validates that `capo <= num_frets` before computing `playable_frets`. The previous code underflowed; the new behavior is `TabError::CapoExceedsFrets`.
- `tuningName: ""` no longer means standard tuning. Pass `"standard"` (case-insensitive) explicitly.
- `Guitar::MAX_NUM_FRETS` and `Guitar::MAX_CAPO` are now `pub const` on `Guitar`, alongside the existing `NumArrangements::MAX`. (Additive.)
- `StringNumber::MAX` is now `pub const` on `StringNumber`. (Additive.)
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): 2.0.0 final-pass entries"
```

---

## Task 15: Update `CONTEXT.md`

**Files:**
- Modify: `CONTEXT.md`

- [ ] **Step 1: Add the `UnplayablePitch` glossary entry**

After the existing `Difficulty features` entry in `CONTEXT.md` (or alphabetically, wherever the project's glossary order has settled), insert:

```markdown
**UnplayablePitch**:
A pitch that could not be placed on any string of the configured [[Guitar]], carrying its plain-text value (e.g. `"A1"`) and the 1-indexed `line` number from the input. Returned in `TabError::UnplayablePitches`. The structured replacement for the 1.x and pre-final-2.0.0 prose error string "Pitch X on line N cannot be played on any strings of the configured guitar."
_Avoid_: Invalid pitch (ambiguous with "unparseable text"), unreachable pitch (current shorthand; `unplayable` is the canonical word at the error layer).
```

- [ ] **Step 2: Commit**

```bash
git add CONTEXT.md
git commit -m "docs(context): add UnplayablePitch glossary entry"
```

---

## Task 16: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Full test suite green**

```bash
cargo test
```

Expected: every test passes.

- [ ] **Step 2: Clippy clean**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: zero warnings.

- [ ] **Step 3: Docs build**

```bash
cargo doc --no-deps
```

Expected: zero warnings.

- [ ] **Step 4: WASM build still produces typed TS surface**

```bash
wasm-pack build --target web --out-dir pkg/wasm_guitar_tab_generator
```

Expected: build succeeds, `pkg/wasm_guitar_tab_generator/guitar_tab_generator.d.ts` mentions the new `TabError` variants (`numFretsTooHigh`, `capoTooHigh`, etc.) and the `UnplayablePitch` interface.

- [ ] **Step 5: Manual diff of the generated TS bindings**

```bash
git diff -- pkg/wasm_guitar_tab_generator/guitar_tab_generator.d.ts
```

Expected: new variants in the `TabError` union, new `UnplayablePitch` interface. No regressions on `TabInput`, `NormalizedBeat`, `ParseError`, `TuningName`.

- [ ] **Step 6: Confirm anyhow shrinkage**

```bash
cargo tree -i anyhow 2>&1 | head -5
```

Expected: either "package ID specification `anyhow` did not match any packages" (best case) or the only remaining hits come from dev-deps like criterion. The published crate's direct dep tree should not list anyhow.

- [ ] **Step 7: Push the branch**

```bash
git push origin v2.0.0
```

(Or open a PR if the team's workflow prefers a feature branch.)

---

## Spec coverage

| Spec section | Tasks |
|---|---|
| Section 2 (flat TabError variants) | Task 1 (variants in), Task 8 (umbrellas out) |
| Section 3 (anyhow removal) | Tasks 2, 3, 4, 5 |
| Section 4 (CapoExceedsFrets) | Task 4 |
| Section 5 (drop empty-string-as-standard) | Task 6 |
| Section 6 (docs / ADRs / migration) | Tasks 12, 13, 14, 15 |
| Section 7 (tests) | Tests embedded in Tasks 1-7; integration smoke in Task 10 |
| `examples/advanced.rs` adaptation | Task 9 |
| `Cargo.toml` anyhow drop | Task 11 |
| Final verification | Task 16 |

Every spec requirement is covered. The plan diverges from the spec's sequencing in two places, both deliberate:

1. The spec listed "delete the old umbrella variants" as step 7 in the sequencing; the plan moves it to Task 8 (after every constructor has cut over) so the build stays green at every commit.
2. The spec proposed `Arc<TabError>` for `create_arrangements`' Err to match the `parse_lines` pattern. The plan keeps that choice (Task 5, Step 3).
