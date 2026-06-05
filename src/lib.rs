#![deny(clippy::correctness)]
#![doc = include_str!("../README.md")]

use serde::{Deserialize, Serialize};
use std::num::NonZeroU8;
use tsify_next::Tsify;
use wasm_bindgen::prelude::*;

pub(crate) mod arrangement;
pub(crate) mod error;
pub(crate) mod guitar;
pub(crate) mod parser;
pub(crate) mod pitch;
pub(crate) mod renderer;
pub(crate) mod string_number;

/// `Arrangement` is re-exported for direct Rust consumers. The canonical 2.x access path
/// for per-arrangement metadata is `ArrangementSet::difficulty(i)` and
/// `ArrangementSet::max_fret_span(i)`; direct construction of `Arrangement` values is internal.
pub use arrangement::{Arrangement, BeatVec, Line, create_arrangements};
pub use error::{ParseError, TabError, UnplayablePitch};
pub use guitar::{Guitar, PitchFingering, create_string_tuning};
pub use parser::{TuningName, get_tuning_names, parse_lines};
pub use pitch::Pitch;
pub use renderer::render_tab;
pub use string_number::StringNumber;

/// Bench-only escape hatches the crate exposes for criterion benchmarks.
///
/// Two unrelated concerns share this namespace:
///
/// 1. **`memoize` bypasses.** `memoized_original_*` variants of `create_arrangements`
///    and `parse_lines` skip the memoize cache so benchmarks measure the underlying
///    work, not cache lookup cost.
/// 2. **Internal tuning-offset helpers.** `parse_tuning` and `create_string_tuning_offset`
///    are the `[i8; 6]` offset machinery the preset-tuning path uses; benches reach for them
///    to construct fixtures without going through the full WASM boundary.
///
/// Not part of the stable 2.x API. May be removed without a major version bump.
#[doc(hidden)]
pub mod __bench_internals {
    pub use crate::arrangement::memoized_original_create_arrangements;
    pub use crate::parser::{
        create_string_tuning_offset, memoized_original_parse_lines, parse_tuning,
    };
}

/// Configuration bundle for one tab-generation request.
///
/// Crosses the WASM boundary via `tsify_next`; JS sees a camelCase interface generated
/// alongside the `.wasm`. `num_arrangements` must be in `1..=NumArrangements::MAX`; the value is validated
/// at the boundary and a [`TabError::NumArrangementsOutOfRange`] is thrown when out of range.
#[derive(Debug, Clone, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TabInput {
    pub input: String,
    /// Name of the tuning preset. Accepts the case-insensitive literal `"standard"` for
    /// standard tuning, or any variant of `TuningName` (case-insensitive, camelCase on the
    /// wire: `"openG"`, `"dropD"`, etc.). Other strings (including the empty string) are
    /// rejected with [`TabError::TuningNameUnknown`].
    pub tuning_name: String,
    pub guitar_num_frets: u8,
    pub guitar_capo: u8,
    pub num_arrangements: u8,
    /// Upper bound on per-beat fret span. An aggressive value can drop the set to zero
    /// arrangements; callers receive `Ok(set)` with `set.len == 0`, not `Err`.
    #[tsify(optional)]
    pub max_fret_span_filter: Option<u8>,
}

impl TabInput {
    /// Builds a `TabInput` with `max_fret_span_filter` defaulted to `None`.
    ///
    /// External callers use this instead of a struct literal. Set the optional fret-span
    /// filter with [`TabInput::with_max_fret_span_filter`].
    #[must_use]
    pub fn new(
        input: impl Into<String>,
        tuning_name: impl Into<String>,
        guitar_num_frets: u8,
        guitar_capo: u8,
        num_arrangements: u8,
    ) -> Self {
        Self {
            input: input.into(),
            tuning_name: tuning_name.into(),
            guitar_num_frets,
            guitar_capo,
            num_arrangements,
            max_fret_span_filter: None,
        }
    }

    /// Sets `max_fret_span_filter` to `Some(filter)`.
    #[must_use]
    pub fn with_max_fret_span_filter(mut self, filter: u8) -> Self {
        self.max_fret_span_filter = Some(filter);
        self
    }
}

/// Validated count of arrangements to compute. Construction enforces `1..=NumArrangements::MAX`.
///
/// Constructed at the boundary by `generate_arrangements` from `TabInput::num_arrangements`.
/// `create_arrangements` accepts this newtype rather than a raw `u8`, so the validation lives
/// in exactly one place.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct NumArrangements(NonZeroU8);

impl NumArrangements {
    /// Upper bound enforced by `try_new`.
    pub const MAX: u8 = 20;

    /// Validates `n` is in `1..=MAX` and returns a `NumArrangements`.
    ///
    /// # Errors
    ///
    /// Returns [`TabError::NumArrangementsOutOfRange`] for `n == 0` or `n > MAX`.
    pub fn try_new(n: u8) -> Result<Self, TabError> {
        if n == 0 || n > Self::MAX {
            return Err(TabError::NumArrangementsOutOfRange {
                value: n,
                max: Self::MAX,
            });
        }
        let nz = NonZeroU8::new(n).expect("BUG: n != 0 verified above");
        Ok(Self(nz))
    }

    /// Returns the underlying `u8` in `1..=MAX`.
    #[inline]
    #[must_use]
    pub fn get(self) -> u8 {
        self.0.get()
    }
}

impl From<NumArrangements> for u8 {
    fn from(n: NumArrangements) -> Self {
        n.get()
    }
}

/// One beat in the normalized input echoed back from `ArrangementSet::normalized_input`.
///
/// Serialized as a discriminated union tagged by `kind`, so JS code can `switch (b.kind)`
/// instead of comparing strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NormalizedBeat {
    Playable { pitches: Vec<String> },
    Rest,
    MeasureBreak,
}

/// Opaque handle holding the result of one `generate_arrangements` call.
///
/// Owns the arrangements, the guitar configuration, and the normalized input shared across
/// arrangements. Per-arrangement metadata (`difficulty`, `max_fret_span`) and the rendered
/// tab string are reached by index through methods on the handle.
#[derive(Debug)]
#[wasm_bindgen]
pub struct ArrangementSet {
    arrangements: Vec<arrangement::Arrangement>,
    guitar: Guitar,
    normalized_input: Vec<NormalizedBeat>,
}

/// `ArrangementSet` indexed accessors return [`TabError::IndexOutOfBounds`] when
/// `index >= self.len`. This is a programmer-side bounds error (the demo clamps before
/// calling); downstream callers can branch on the typed variant to surface it differently
/// from user-facing errors like [`TabError::TuningNameUnknown`] or
/// [`TabError::NumArrangementsOutOfRange`].
#[wasm_bindgen]
impl ArrangementSet {
    /// Number of arrangements in the set. Equal to the requested `num_arrangements`, possibly
    /// reduced by `max_fret_span_filter` when filtering would otherwise drop below the count.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.arrangements.len()
    }

    /// Returns true when `len == 0`.
    #[wasm_bindgen(getter, js_name = "isEmpty")]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.arrangements.is_empty()
    }

    /// The per-beat input echoed back as a sequence of tagged `NormalizedBeat` variants.
    /// Shared across all arrangements; lives once on the set.
    ///
    /// Returns a fresh `Vec` on each call; cache on the JS side if reading repeatedly.
    /// `examples/wasm.html` caches the result on `state.normalizedInput` and reads from that
    /// cache in the rerender path; that pattern is the intended consumer shape.
    #[wasm_bindgen(getter, js_name = "normalizedInput")]
    #[must_use]
    pub fn normalized_input(&self) -> Vec<NormalizedBeat> {
        self.normalized_input.clone()
    }

    /// Largest non-zero fret span across any beat in the arrangement at `index`.
    ///
    /// # Errors
    ///
    /// Returns [`TabError::IndexOutOfBounds`] when `index >= self.len`.
    #[wasm_bindgen(js_name = "maxFretSpan")]
    pub fn max_fret_span(&self, index: usize) -> Result<u8, TabError> {
        self.arrangements
            .get(index)
            .map(|a| a.max_fret_span())
            .ok_or(TabError::IndexOutOfBounds {
                index,
                len: self.arrangements.len(),
            })
    }

    /// Difficulty score for the arrangement at `index`. Lower is easier.
    ///
    /// # Errors
    ///
    /// Returns [`TabError::IndexOutOfBounds`] when `index >= self.len`.
    pub fn difficulty(&self, index: usize) -> Result<i32, TabError> {
        self.arrangements
            .get(index)
            .map(|a| a.difficulty())
            .ok_or(TabError::IndexOutOfBounds {
                index,
                len: self.arrangements.len(),
            })
    }

    /// Renders the arrangement at `index` at the supplied `width`, `padding`, and optional
    /// `playback` beat indicator. Cheap to call repeatedly with different render parameters
    /// -- pathfinding does not re-run.
    ///
    /// # Errors
    ///
    /// Returns [`TabError::RenderWidthTooSmall`] when `width` is below the minimum needed to
    /// lay out one beat at the given `padding` (`min_render_width(padding)`), in addition to the
    /// [`TabError::IndexOutOfBounds`] shared by every indexed accessor.
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
            .ok_or(TabError::IndexOutOfBounds {
                index,
                len: self.arrangements.len(),
            })?;
        let min = renderer::min_render_width(padding);
        if width < min {
            return Err(TabError::RenderWidthTooSmall { width, min });
        }
        Ok(renderer::render_tab(
            &arrangement.lines,
            &self.guitar,
            width,
            padding,
            playback,
        ))
    }
}

/// Generates an `ArrangementSet` from a `TabInput`. Single entry point for both Rust callers
/// and the WASM boundary; JS sees this as `generateArrangements`.
///
/// # Errors
///
/// Returns the typed [`TabError`] variant for each failure mode reachable from this entry point:
///
/// - Input-shape validation: [`TabError::NumArrangementsOutOfRange`], [`TabError::TuningNameUnknown`],
///   [`TabError::NumFretsTooHigh`], [`TabError::CapoTooHigh`], [`TabError::CapoExceedsFrets`].
/// - Parser: [`TabError::Parse`] (carries `Vec<ParseError>` with line/text per unparseable substring),
///   [`TabError::InputTooManyLines`] (input exceeds the 65,535-line cap).
/// - Pathfinding: [`TabError::UnplayablePitches`] (one or more pitches reach no string),
///   [`TabError::NoArrangementsFound`] (every pitch reaches the guitar but no valid combination exists,
///   for example duplicate pitches in a single beat that the no-duplicate-strings constraint filters away).
///
/// [`TabError::OpenPitchOutOfRange`], [`TabError::StringNumberOutOfRange`], and
/// [`TabError::FretRangeExceedsPitchRange`] are members of the enum and live on the [`Guitar::new`] path
/// this function calls, but no `TabInput` reachable today can trip them: the preset tunings and fixed
/// 1..=6 string numbering keep every open-string pitch and fret range well inside the supported `Pitch`
/// range. They fire only when constructed directly through the lower-level Rust API ([`Guitar::new`],
/// [`create_string_tuning`]) with out-of-range inputs, such as a custom tuning (deferred to a later
/// release).
///
/// # Validation order
///
/// Input-shape errors (currently `numArrangements` range) are reported before `parse_lines`
/// runs. The ordering is deliberate: shape checks are O(1) and unambiguous, while parse errors
/// depend on the full input. When both are present the shape error wins because the parser's
/// output would be discarded anyway.
///
/// Guitar-configuration errors (`TuningNameUnknown`, `NumFretsTooHigh`, `CapoTooHigh`,
/// `CapoExceedsFrets`) are checked before the normalized input is built, so an invalid guitar
/// config does not pay for the per-beat allocation. `parse_lines` still runs first, so a `Parse`
/// error outranks a guitar-config error.
///
/// # Performance
///
/// `tab_input.input` is cloned once per call because `parse_lines` is `#[memoize]`d on owned
/// `String`. Memoization makes a repeat call with the same input cheap, but the clone runs
/// on every call (including cache hits). Hot loops over `generate_arrangements` should expect
/// one `String::clone` per invocation in addition to the boundary deserialization cost.
#[wasm_bindgen(js_name = "generateArrangements")]
pub fn generate_arrangements(tab_input: TabInput) -> Result<ArrangementSet, TabError> {
    let num_arrangements = NumArrangements::try_new(tab_input.num_arrangements)?;

    let input_lines = parser::parse_lines(tab_input.input.clone())?;

    // Validate the guitar configuration before materializing the normalized input, so a
    // request with a valid pitch list but a bad tuning name or out-of-range fret/capo fails
    // before allocating the per-beat `normalized_input` vector. `parse_lines` still runs
    // first, so a `Parse` error keeps precedence over a guitar-config error.
    let tuning = parser::create_string_tuning_offset(parser::parse_tuning(&tab_input.tuning_name)?);
    let guitar = Guitar::new(tuning, tab_input.guitar_num_frets, tab_input.guitar_capo)?;

    let first_playable_index = arrangement::first_playable_index(&input_lines);

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

    let arrangements = arrangement::create_arrangements(
        guitar.clone(),
        input_lines,
        num_arrangements,
        tab_input.max_fret_span_filter,
    )?;

    Ok(ArrangementSet {
        arrangements,
        guitar,
        normalized_input,
    })
}

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
        let set = generate_arrangements(tab_input).unwrap();

        assert_eq!(set.len(), 1);
        assert_eq!(set.max_fret_span(0).unwrap(), 0);

        let tab = set.render(0, 30, 2, Some(3)).unwrap();
        assert_eq!(
            tab,
            "           \u{25bc}\n--------------------|--0------\n-----------------0--|---------\n--------------0-----|---------\n--------0-----------|---------\n-----0--------------|---------\n--0-----------------|---------\n           \u{25b2}\n"
        );

        let beats = set.normalized_input();
        assert_eq!(beats.len(), 8);
        assert!(
            matches!(beats[0], NormalizedBeat::Playable { ref pitches } if pitches == &["E2".to_owned()])
        );
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
        let set = generate_arrangements(tab_input).unwrap();
        assert_eq!(set.len(), 2);
        assert_eq!(set.render(0, 30, 2, Some(3)).unwrap(), "");
        assert_eq!(set.render(1, 30, 2, Some(3)).unwrap(), "");

        // Pins the current behaviour: when no `Playable` line exists, `first_playable_index`
        // falls back to 0 and `normalized_input` echoes every input line (the trailing
        // `MeasureBreak` from `---` and the leading blank rests). Empty / all-rest input
        // returns Ok(set) by design (see docs/adr/0006-empty-input-returns-empty-set.md);
        // interactive UIs lean on this to avoid bouncing into an error pane during edits.
        let beats = set.normalized_input();
        assert!(
            beats
                .iter()
                .all(|b| matches!(b, NormalizedBeat::Rest | NormalizedBeat::MeasureBreak))
        );
        assert!(
            beats
                .iter()
                .any(|b| matches!(b, NormalizedBeat::MeasureBreak))
        );
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
        let err = generate_arrangements(tab_input).unwrap_err();
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
        let err = generate_arrangements(tab_input).unwrap_err();
        match err {
            TabError::NumArrangementsOutOfRange { value, max } => {
                assert_eq!(value, 0);
                assert_eq!(max, 20);
            }
            other => panic!("expected NumArrangementsOutOfRange, got {other:?}"),
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
        let err = generate_arrangements(tab_input).unwrap_err();
        match err {
            TabError::NumArrangementsOutOfRange { value, max } => {
                assert_eq!(value, 21);
                assert_eq!(max, 20);
            }
            other => panic!("expected NumArrangementsOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn invalid_guitar_config_returns_num_frets_too_high() {
        let tab_input = TabInput {
            input: "E2".to_owned(),
            tuning_name: "standard".to_owned(),
            guitar_num_frets: 31, // exceeds Guitar::MAX_NUM_FRETS = 30
            guitar_capo: 0,
            num_arrangements: 1,
            max_fret_span_filter: None,
        };
        let err = generate_arrangements(tab_input).unwrap_err();
        match err {
            TabError::NumFretsTooHigh { num_frets, max } => {
                assert_eq!(num_frets, 31);
                assert_eq!(max, 30);
            }
            other => panic!("expected NumFretsTooHigh, got {other:?}"),
        }
    }

    #[test]
    fn unreachable_pitch_returns_unplayable_pitches() {
        let tab_input = TabInput {
            input: "A1".to_owned(), // below standard tuning's low E2; unplayable on any string
            tuning_name: "standard".to_owned(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 1,
            max_fret_span_filter: None,
        };
        let err = generate_arrangements(tab_input).unwrap_err();
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
    fn unplayable_pitch_line_accounts_for_leading_rests() {
        // Two leading blank lines (rests) precede an unplayable pitch on input line 3.
        // The reported line must be the 1-indexed input line (3), not the position within
        // the post-leading-rest beat sequence (which would be 1).
        let tab_input = TabInput {
            input: "\n\nA1".to_owned(),
            tuning_name: "standard".to_owned(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 1,
            max_fret_span_filter: None,
        };
        let err = generate_arrangements(tab_input).unwrap_err();
        match err {
            TabError::UnplayablePitches { pitches } => {
                assert_eq!(pitches.len(), 1);
                assert_eq!(pitches[0].value, "A1");
                assert_eq!(pitches[0].line, 3);
            }
            other => panic!("expected UnplayablePitches, got {other:?}"),
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
        let set = generate_arrangements(tab_input).unwrap();
        let narrow = set.render(0, 12, 1, None).unwrap();
        let wide = set.render(0, 100, 1, None).unwrap();
        assert_ne!(narrow, wide);
    }
}

#[cfg(test)]
mod test_boundary_types {
    use super::*;

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
            TabError::IndexOutOfBounds { .. } => {}
            other => panic!("expected IndexOutOfBounds, got {other:?}"),
        }
    }

    #[test]
    fn arrangement_set_render_rejects_width_below_minimum() {
        let set = arrangement_set_fixture(1);
        // padding 1 -> min width = 2 * 1 + 2 + 1 = 5; width 3 is below it.
        let err = set.render(0, 3, 1, None).unwrap_err();
        assert_eq!(err, TabError::RenderWidthTooSmall { width: 3, min: 5 });
    }

    #[test]
    fn arrangement_set_max_fret_span_returns_value_for_in_bounds_index() {
        let set = arrangement_set_fixture(1);
        let span = set.max_fret_span(0).unwrap();
        assert_eq!(span, 0);
    }

    #[test]
    fn arrangement_set_max_fret_span_rejects_out_of_bounds_index() {
        let set = arrangement_set_fixture(1);
        let err = set.max_fret_span(99).unwrap_err();
        match err {
            TabError::IndexOutOfBounds { .. } => {}
            other => panic!("expected IndexOutOfBounds, got {other:?}"),
        }
    }

    #[test]
    fn arrangement_set_difficulty_returns_value_for_in_bounds_index() {
        let set = arrangement_set_fixture(1);
        // The fixture is all open strings (E2/A2/D3 in standard tuning), so the optimal
        // arrangement has difficulty 0. Pin the value, not just that the call succeeds.
        assert_eq!(set.difficulty(0).unwrap(), 0);
    }

    #[test]
    fn arrangement_set_difficulty_rejects_out_of_bounds_index() {
        let set = arrangement_set_fixture(1);
        let err = set.difficulty(99).unwrap_err();
        match err {
            TabError::IndexOutOfBounds { .. } => {}
            other => panic!("expected IndexOutOfBounds, got {other:?}"),
        }
    }

    #[test]
    fn arrangement_set_is_empty_returns_false_for_non_empty_set() {
        let set = arrangement_set_fixture(1);
        assert!(!set.is_empty());
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn arrangement_set_is_empty_returns_true_when_filter_drops_every_candidate() {
        // C3E3 forces both notes onto fretted positions in standard tuning, so every
        // candidate arrangement has a non-zero fret span. max_fret_span_filter = Some(0)
        // drops all of them. See src/arrangement.rs::max_fret_span_filter_can_produce_empty_set
        // for the analogous test at the internal layer.
        let tab_input = TabInput {
            input: "C3E3".to_owned(),
            tuning_name: "standard".to_owned(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 5,
            max_fret_span_filter: Some(0),
        };
        let set = generate_arrangements(tab_input).unwrap();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn arrangement_set_render_rejects_index_when_filter_empties_the_set() {
        // C3E3 forces both notes onto fretted positions, so max_fret_span_filter = Some(0)
        // drops every candidate and leaves an empty set. render(0, ..) must report
        // IndexOutOfBounds rather than reaching the renderer's non-empty guard.
        let tab_input = TabInput {
            input: "C3E3".to_owned(),
            tuning_name: "standard".to_owned(),
            guitar_num_frets: 20,
            guitar_capo: 0,
            num_arrangements: 5,
            max_fret_span_filter: Some(0),
        };
        let set = generate_arrangements(tab_input).unwrap();
        assert!(set.is_empty());
        let err = set.render(0, 30, 2, None).unwrap_err();
        assert!(
            matches!(err, TabError::IndexOutOfBounds { index: 0, len: 0 }),
            "got {err:?}"
        );
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
        generate_arrangements(tab_input).unwrap()
    }

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

#[cfg(test)]
mod test_num_arrangements {
    use super::*;

    #[test]
    fn try_new_accepts_one_through_max() {
        for n in 1u8..=NumArrangements::MAX {
            assert!(NumArrangements::try_new(n).is_ok(), "n={n} must be Ok");
        }
    }

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

    #[test]
    fn try_new_rejects_above_max_with_typed_variant() {
        let err = NumArrangements::try_new(NumArrangements::MAX + 1).unwrap_err();
        match err {
            TabError::NumArrangementsOutOfRange { value, max } => {
                assert_eq!(value, 21);
                assert_eq!(max, 20);
            }
            other => panic!("expected NumArrangementsOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn get_returns_inner_value() {
        let n = NumArrangements::try_new(7).unwrap();
        assert_eq!(n.get(), 7);
    }
}

#[cfg(test)]
mod test_tab_input {
    use super::*;

    #[test]
    fn new_defaults_max_fret_span_filter_to_none() {
        let input = TabInput::new("E2\nA2", "standard", 18, 0, 1);
        assert_eq!(input.input, "E2\nA2");
        assert_eq!(input.tuning_name, "standard");
        assert_eq!(input.guitar_num_frets, 18);
        assert_eq!(input.guitar_capo, 0);
        assert_eq!(input.num_arrangements, 1);
        assert_eq!(input.max_fret_span_filter, None);
    }

    #[test]
    fn with_max_fret_span_filter_sets_some() {
        let input = TabInput::new("E2", "standard", 18, 0, 1).with_max_fret_span_filter(5);
        assert_eq!(input.max_fret_span_filter, Some(5));
    }
}
