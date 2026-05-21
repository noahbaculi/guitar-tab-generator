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
    /// Returns the underlying `u8`.
    #[inline]
    #[must_use]
    pub fn get(&self) -> u8 {
        self.0
    }
}
#[cfg(test)]
mod test_create_string_number {
    use super::*;
    #[test]
    fn valid_simple() {
        assert!(StringNumber::new(1).is_ok());
    }

    #[test]
    fn invalid() {
        for n in [0u8, 13, 100, 255] {
            assert!(StringNumber::new(n).is_err(), "n={n} must be Err");
        }
    }

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
}

impl fmt::Debug for StringNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            1 => f.write_str("1_e"),
            2 => f.write_str("2_B"),
            3 => f.write_str("3_G"),
            4 => f.write_str("4_D"),
            5 => f.write_str("5_A"),
            6 => f.write_str("6_E"),
            n => write!(f, "{n}"),
        }
    }
}
#[cfg(test)]
mod test_pitch_debug {
    use super::*;
    #[test]
    fn strings_1_to_6() {
        assert_eq!(format!("{:?}", StringNumber::new(1).unwrap()), "1_e");
        assert_eq!(format!("{:?}", StringNumber::new(2).unwrap()), "2_B");
        assert_eq!(format!("{:?}", StringNumber::new(3).unwrap()), "3_G");
        assert_eq!(format!("{:?}", StringNumber::new(4).unwrap()), "4_D");
        assert_eq!(format!("{:?}", StringNumber::new(5).unwrap()), "5_A");
        assert_eq!(format!("{:?}", StringNumber::new(6).unwrap()), "6_E");
    }
    #[test]
    fn string_greater_than_6() {
        assert_eq!(format!("{:?}", StringNumber::new(8).unwrap()), "8");
    }
}
