use std::fmt;
use strum_macros::{EnumIter, EnumString, FromRepr};

#[derive(
    Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, EnumIter, FromRepr, EnumString,
)]
#[strum(ascii_case_insensitive)]
pub enum Pitch {
    C0,
    #[strum(serialize = "C#0", serialize = "Db0")]
    CSharpDFlat0,
    D0,
    DSharpEFlat0,
    E0,
    F0,
    FSharpGFlat0,
    G0,
    GSharpAFlat0,
    A0,
    ASharpBFlat0,
    B0,
    C1,
    CSharpDFlat1,
    D1,
    DSharpEFlat1,
    E1,
    F1,
    FSharpGFlat1,
    G1,
    GSharpAFlat1,
    A1,
    ASharpBFlat1,
    B1,
    C2,
    CSharpDFlat2,
    D2,
    DSharpEFlat2,
    E2,
    F2,
    FSharpGFlat2,
    G2,
    GSharpAFlat2,
    A2,
    ASharpBFlat2,
    B2,
    C3,
    CSharpDFlat3,
    D3,
    DSharpEFlat3,
    E3,
    F3,
    FSharpGFlat3,
    G3,
    GSharpAFlat3,
    A3,
    ASharpBFlat3,
    B3,
    C4,
    CSharpDFlat4,
    D4,
    DSharpEFlat4,
    E4,
    F4,
    FSharpGFlat4,
    G4,
    GSharpAFlat4,
    A4,
    ASharpBFlat4,
    B4,
    C5,
    CSharpDFlat5,
    D5,
    DSharpEFlat5,
    E5,
    F5,
    FSharpGFlat5,
    G5,
    GSharpAFlat5,
    A5,
    ASharpBFlat5,
    B5,
    C6,
    CSharpDFlat6,
    D6,
    DSharpEFlat6,
    E6,
    F6,
    FSharpGFlat6,
    G6,
    GSharpAFlat6,
    A6,
    ASharpBFlat6,
    B6,
    C7,
    CSharpDFlat7,
    D7,
    DSharpEFlat7,
    E7,
    F7,
    FSharpGFlat7,
    G7,
    GSharpAFlat7,
    A7,
    ASharpBFlat7,
    B7,
    C8,
    CSharpDFlat8,
    D8,
    DSharpEFlat8,
    E8,
    F8,
    FSharpGFlat8,
    G8,
    GSharpAFlat8,
    A8,
    ASharpBFlat8,
    B8,
    C9,
    CSharpDFlat9,
    D9,
    DSharpEFlat9,
    E9,
    F9,
    FSharpGFlat9,
    G9,
    GSharpAFlat9,
    A9,
    ASharpBFlat9,
    B9,
}
impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pitch_w_pretty_sharp = format!("{:?}", self)
            .replace("Sharp", "♯")
            .replace("Flat", "♭");
        write!(f, "{}", pitch_w_pretty_sharp)
    }
}
#[cfg(test)]
mod test_pitch_display {
    use super::*;
    #[test]
    fn valid_simple() {
        assert_eq!(format!("{}", Pitch::E2), "E2");
        assert_eq!(format!("{}", Pitch::DSharpEFlat3), "D♯E♭3");
    }
}

impl Pitch {
    pub fn index(&self) -> u8 {
        *self as u8
    }
}
