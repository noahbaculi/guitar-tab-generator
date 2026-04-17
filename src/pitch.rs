use anyhow::{anyhow, Result};
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
        f.write_str(match self {
            Pitch::C0 => "C0",
            Pitch::CSharpDFlat0 => "C♯D♭0",
            Pitch::D0 => "D0",
            Pitch::DSharpEFlat0 => "D♯E♭0",
            Pitch::E0 => "E0",
            Pitch::F0 => "F0",
            Pitch::FSharpGFlat0 => "F♯G♭0",
            Pitch::G0 => "G0",
            Pitch::GSharpAFlat0 => "G♯A♭0",
            Pitch::A0 => "A0",
            Pitch::ASharpBFlat0 => "A♯B♭0",
            Pitch::B0 => "B0",
            Pitch::C1 => "C1",
            Pitch::CSharpDFlat1 => "C♯D♭1",
            Pitch::D1 => "D1",
            Pitch::DSharpEFlat1 => "D♯E♭1",
            Pitch::E1 => "E1",
            Pitch::F1 => "F1",
            Pitch::FSharpGFlat1 => "F♯G♭1",
            Pitch::G1 => "G1",
            Pitch::GSharpAFlat1 => "G♯A♭1",
            Pitch::A1 => "A1",
            Pitch::ASharpBFlat1 => "A♯B♭1",
            Pitch::B1 => "B1",
            Pitch::C2 => "C2",
            Pitch::CSharpDFlat2 => "C♯D♭2",
            Pitch::D2 => "D2",
            Pitch::DSharpEFlat2 => "D♯E♭2",
            Pitch::E2 => "E2",
            Pitch::F2 => "F2",
            Pitch::FSharpGFlat2 => "F♯G♭2",
            Pitch::G2 => "G2",
            Pitch::GSharpAFlat2 => "G♯A♭2",
            Pitch::A2 => "A2",
            Pitch::ASharpBFlat2 => "A♯B♭2",
            Pitch::B2 => "B2",
            Pitch::C3 => "C3",
            Pitch::CSharpDFlat3 => "C♯D♭3",
            Pitch::D3 => "D3",
            Pitch::DSharpEFlat3 => "D♯E♭3",
            Pitch::E3 => "E3",
            Pitch::F3 => "F3",
            Pitch::FSharpGFlat3 => "F♯G♭3",
            Pitch::G3 => "G3",
            Pitch::GSharpAFlat3 => "G♯A♭3",
            Pitch::A3 => "A3",
            Pitch::ASharpBFlat3 => "A♯B♭3",
            Pitch::B3 => "B3",
            Pitch::C4 => "C4",
            Pitch::CSharpDFlat4 => "C♯D♭4",
            Pitch::D4 => "D4",
            Pitch::DSharpEFlat4 => "D♯E♭4",
            Pitch::E4 => "E4",
            Pitch::F4 => "F4",
            Pitch::FSharpGFlat4 => "F♯G♭4",
            Pitch::G4 => "G4",
            Pitch::GSharpAFlat4 => "G♯A♭4",
            Pitch::A4 => "A4",
            Pitch::ASharpBFlat4 => "A♯B♭4",
            Pitch::B4 => "B4",
            Pitch::C5 => "C5",
            Pitch::CSharpDFlat5 => "C♯D♭5",
            Pitch::D5 => "D5",
            Pitch::DSharpEFlat5 => "D♯E♭5",
            Pitch::E5 => "E5",
            Pitch::F5 => "F5",
            Pitch::FSharpGFlat5 => "F♯G♭5",
            Pitch::G5 => "G5",
            Pitch::GSharpAFlat5 => "G♯A♭5",
            Pitch::A5 => "A5",
            Pitch::ASharpBFlat5 => "A♯B♭5",
            Pitch::B5 => "B5",
            Pitch::C6 => "C6",
            Pitch::CSharpDFlat6 => "C♯D♭6",
            Pitch::D6 => "D6",
            Pitch::DSharpEFlat6 => "D♯E♭6",
            Pitch::E6 => "E6",
            Pitch::F6 => "F6",
            Pitch::FSharpGFlat6 => "F♯G♭6",
            Pitch::G6 => "G6",
            Pitch::GSharpAFlat6 => "G♯A♭6",
            Pitch::A6 => "A6",
            Pitch::ASharpBFlat6 => "A♯B♭6",
            Pitch::B6 => "B6",
            Pitch::C7 => "C7",
            Pitch::CSharpDFlat7 => "C♯D♭7",
            Pitch::D7 => "D7",
            Pitch::DSharpEFlat7 => "D♯E♭7",
            Pitch::E7 => "E7",
            Pitch::F7 => "F7",
            Pitch::FSharpGFlat7 => "F♯G♭7",
            Pitch::G7 => "G7",
            Pitch::GSharpAFlat7 => "G♯A♭7",
            Pitch::A7 => "A7",
            Pitch::ASharpBFlat7 => "A♯B♭7",
            Pitch::B7 => "B7",
            Pitch::C8 => "C8",
            Pitch::CSharpDFlat8 => "C♯D♭8",
            Pitch::D8 => "D8",
            Pitch::DSharpEFlat8 => "D♯E♭8",
            Pitch::E8 => "E8",
            Pitch::F8 => "F8",
            Pitch::FSharpGFlat8 => "F♯G♭8",
            Pitch::G8 => "G8",
            Pitch::GSharpAFlat8 => "G♯A♭8",
            Pitch::A8 => "A8",
            Pitch::ASharpBFlat8 => "A♯B♭8",
            Pitch::B8 => "B8",
            Pitch::C9 => "C9",
            Pitch::CSharpDFlat9 => "C♯D♭9",
            Pitch::D9 => "D9",
            Pitch::DSharpEFlat9 => "D♯E♭9",
            Pitch::E9 => "E9",
            Pitch::F9 => "F9",
            Pitch::FSharpGFlat9 => "F♯G♭9",
            Pitch::G9 => "G9",
            Pitch::GSharpAFlat9 => "G♯A♭9",
            Pitch::A9 => "A9",
            Pitch::ASharpBFlat9 => "A♯B♭9",
            Pitch::B9 => "B9",
        })
    }
}
#[cfg(test)]
mod test_pitch_display {
    use super::*;
    #[test]
    fn natural_pitch() {
        assert_eq!(format!("{}", Pitch::E2), "E2");
    }
    #[test]
    fn sharp_flat_pitch() {
        assert_eq!(format!("{}", Pitch::DSharpEFlat3), "D♯E♭3");
    }
}

impl Pitch {
    #[inline]
    #[must_use]
    pub fn index(&self) -> u8 {
        *self as u8
    }

    #[must_use]
    pub fn plain_text(&self) -> String {
        let pitch_variant_name = format!("{self:?}");

        let pitch_plain_text = &pitch_variant_name[pitch_variant_name.find("Sharp").unwrap_or(0)..]
            .replace("Sharp", "")
            .replace("Flat", "b");

        pitch_plain_text.to_owned()
    }

    pub fn plus_offset(&self, offset: i16) -> Result<Pitch> {
        let new_index = self.index() as i16 + offset;
        if new_index < 0 {
            return Err(anyhow!(
                "Pitch {self} offset by {offset} pitches results in a pitch out of range."
            ));
        }
        Pitch::from_repr(new_index as usize).ok_or_else(|| {
            anyhow!("Pitch {self} offset by {offset} pitches results in a pitch out of range.")
        })
    }
}
#[cfg(test)]
mod test_pitch_plain_text {
    use super::*;

    #[test]
    fn natural_pitch() {
        assert_eq!(Pitch::C8.plain_text(), "C8");
    }
    #[test]
    fn sharp_flat_pitch() {
        assert_eq!(Pitch::CSharpDFlat9.plain_text(), "Db9");
    }
}
#[cfg(test)]
mod test_pitch_plus_offset {
    use super::*;

    #[test]
    fn valid_positive() {
        assert_eq!(Pitch::FSharpGFlat3.plus_offset(3).unwrap(), Pitch::A3);
    }
    #[test]
    fn valid_negative() {
        assert_eq!(
            Pitch::FSharpGFlat3.plus_offset(-3).unwrap(),
            Pitch::DSharpEFlat3
        );
    }
    #[test]
    fn test_plus_offset_exceeds_range() {
        let error = Pitch::ASharpBFlat9.plus_offset(2).unwrap_err();
        let error_msg = format!("{error}");

        assert_eq!(
            error_msg,
            "Pitch A♯B♭9 offset by 2 pitches results in a pitch out of range."
        );
    }
}
