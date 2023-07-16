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
    #[strum(serialize = "D#0", serialize = "Eb0")]
    DSharpEFlat0,
    E0,
    F0,
    #[strum(serialize = "F#0", serialize = "Gb0")]
    FSharpGFlat0,
    G0,
    #[strum(serialize = "G#0", serialize = "Ab0")]
    GSharpAFlat0,
    A0,
    #[strum(serialize = "A#0", serialize = "Bb0")]
    ASharpBFlat0,
    B0,
    C1,
    #[strum(serialize = "C#1", serialize = "Db1")]
    CSharpDFlat1,
    D1,
    #[strum(serialize = "D#1", serialize = "Eb1")]
    DSharpEFlat1,
    E1,
    F1,
    #[strum(serialize = "F#1", serialize = "Gb1")]
    FSharpGFlat1,
    G1,
    #[strum(serialize = "G#1", serialize = "Ab1")]
    GSharpAFlat1,
    A1,
    #[strum(serialize = "A#1", serialize = "Bb1")]
    ASharpBFlat1,
    B1,
    C2,
    #[strum(serialize = "C#2", serialize = "Db2")]
    CSharpDFlat2,
    D2,
    #[strum(serialize = "D#2", serialize = "Eb2")]
    DSharpEFlat2,
    E2,
    F2,
    #[strum(serialize = "F#2", serialize = "Gb2")]
    FSharpGFlat2,
    G2,
    #[strum(serialize = "G#2", serialize = "Ab2")]
    GSharpAFlat2,
    A2,
    #[strum(serialize = "A#2", serialize = "Bb2")]
    ASharpBFlat2,
    B2,
    C3,
    #[strum(serialize = "C#3", serialize = "Db3")]
    CSharpDFlat3,
    D3,
    #[strum(serialize = "D#3", serialize = "Eb3")]
    DSharpEFlat3,
    E3,
    F3,
    #[strum(serialize = "F#3", serialize = "Gb3")]
    FSharpGFlat3,
    G3,
    #[strum(serialize = "G#3", serialize = "Ab3")]
    GSharpAFlat3,
    A3,
    #[strum(serialize = "A#3", serialize = "Bb3")]
    ASharpBFlat3,
    B3,
    C4,
    #[strum(serialize = "C#4", serialize = "Db4")]
    CSharpDFlat4,
    D4,
    #[strum(serialize = "D#4", serialize = "Eb4")]
    DSharpEFlat4,
    E4,
    F4,
    #[strum(serialize = "F#4", serialize = "Gb4")]
    FSharpGFlat4,
    G4,
    #[strum(serialize = "G#4", serialize = "Ab4")]
    GSharpAFlat4,
    A4,
    #[strum(serialize = "A#4", serialize = "Bb4")]
    ASharpBFlat4,
    B4,
    C5,
    #[strum(serialize = "C#5", serialize = "Db5")]
    CSharpDFlat5,
    D5,
    #[strum(serialize = "D#5", serialize = "Eb5")]
    DSharpEFlat5,
    E5,
    F5,
    #[strum(serialize = "F#5", serialize = "Gb5")]
    FSharpGFlat5,
    G5,
    #[strum(serialize = "G#5", serialize = "Ab5")]
    GSharpAFlat5,
    A5,
    #[strum(serialize = "A#5", serialize = "Bb5")]
    ASharpBFlat5,
    B5,
    C6,
    #[strum(serialize = "C#6", serialize = "Db6")]
    CSharpDFlat6,
    D6,
    #[strum(serialize = "D#6", serialize = "Eb6")]
    DSharpEFlat6,
    E6,
    F6,
    #[strum(serialize = "F#6", serialize = "Gb6")]
    FSharpGFlat6,
    G6,
    #[strum(serialize = "G#6", serialize = "Ab6")]
    GSharpAFlat6,
    A6,
    #[strum(serialize = "A#6", serialize = "Bb6")]
    ASharpBFlat6,
    B6,
    C7,
    #[strum(serialize = "C#7", serialize = "Db7")]
    CSharpDFlat7,
    D7,
    #[strum(serialize = "D#7", serialize = "Eb7")]
    DSharpEFlat7,
    E7,
    F7,
    #[strum(serialize = "F#7", serialize = "Gb7")]
    FSharpGFlat7,
    G7,
    #[strum(serialize = "G#7", serialize = "Ab7")]
    GSharpAFlat7,
    A7,
    #[strum(serialize = "A#7", serialize = "Bb7")]
    ASharpBFlat7,
    B7,
    C8,
    #[strum(serialize = "C#8", serialize = "Db8")]
    CSharpDFlat8,
    D8,
    #[strum(serialize = "D#8", serialize = "Eb8")]
    DSharpEFlat8,
    E8,
    F8,
    #[strum(serialize = "F#8", serialize = "Gb8")]
    FSharpGFlat8,
    G8,
    #[strum(serialize = "G#8", serialize = "Ab8")]
    GSharpAFlat8,
    A8,
    #[strum(serialize = "A#8", serialize = "Bb8")]
    ASharpBFlat8,
    B8,
    C9,
    #[strum(serialize = "C#9", serialize = "Db9")]
    CSharpDFlat9,
    D9,
    #[strum(serialize = "D#9", serialize = "Eb9")]
    DSharpEFlat9,
    E9,
    F9,
    #[strum(serialize = "F#9", serialize = "Gb9")]
    FSharpGFlat9,
    G9,
    #[strum(serialize = "G#9", serialize = "Ab9")]
    GSharpAFlat9,
    A9,
    #[strum(serialize = "A#9", serialize = "Bb9")]
    ASharpBFlat9,
    B9,
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pitch_str_repr = format!("{:?}", self)
            .replace("Sharp", "♯")
            .replace("Flat", "♭");
        write!(f, "{}", pitch_str_repr)
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
