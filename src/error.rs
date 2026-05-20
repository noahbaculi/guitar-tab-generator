//! Error types crossing the WASM boundary.
//!
//! `ParseError` is used both internally by the parser and as a leaf of `TabError::Parse`.
//! `TabError` is the tagged enum the WASM boundary throws on failure.

use serde::Serialize;
use tsify_next::Tsify;

/// One unparseable substring in the input, with its 1-indexed line number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Tsify)]
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
