use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

use crate::constant::ET_SIZE;

/// Position on the Circle of Fifths.
/// C is 0, F is -1, G is 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Chroma(i32);

impl Chroma {
    pub fn new(value: i32) -> Self {
        Chroma(value)
    }

    pub fn value(self) -> i32 {
        self.0
    }
}

/// Interval in semitones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Semitone(i32);

impl Semitone {
    pub fn new(value: i32) -> Self {
        Semitone(value)
    }

    pub fn value(self) -> i32 {
        self.0
    }
}

/// Unit on the Circle of Fifths for distance calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct COFUnit(i32);

impl COFUnit {
    pub fn new(value: i32) -> Self {
        COFUnit(value)
    }

    pub fn value(self) -> i32 {
        self.0
    }
}

impl std::ops::Sub for COFUnit {
    type Output = COFUnit;
    fn sub(self, rhs: COFUnit) -> COFUnit {
        COFUnit(self.0 - rhs.0)
    }
}

/// Musical pitch class (C, C#, D, etc.) — wraps a value 0..11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PitchClass(u8);

impl PitchClass {
    pub const C: PitchClass = PitchClass(0);
    pub const CS: PitchClass = PitchClass(1);
    pub const DB: PitchClass = PitchClass(1);
    pub const D: PitchClass = PitchClass(2);
    pub const DS: PitchClass = PitchClass(3);
    pub const EB: PitchClass = PitchClass(3);
    pub const E: PitchClass = PitchClass(4);
    pub const F: PitchClass = PitchClass(5);
    pub const FS: PitchClass = PitchClass(6);
    pub const GB: PitchClass = PitchClass(6);
    pub const G: PitchClass = PitchClass(7);
    pub const GS: PitchClass = PitchClass(8);
    pub const AB: PitchClass = PitchClass(8);
    pub const A: PitchClass = PitchClass(9);
    pub const AS: PitchClass = PitchClass(10);
    pub const BB: PitchClass = PitchClass(10);
    pub const B: PitchClass = PitchClass(11);

    pub fn new(value: u8) -> Self {
        debug_assert!(value < ET_SIZE as u8);
        PitchClass(value)
    }

    pub fn value(self) -> u8 {
        self.0
    }

    /// Position on the Circle of Fifths.
    pub fn get_chroma(self) -> Chroma {
        let et = ET_SIZE as i32;
        let v = self.0 as i32 % et;
        Chroma(et / 2 - ((et / 2 - 1) * v + et / 2) % et)
    }
}

impl Serialize for PitchClass {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.0)
    }
}

impl<'de> Deserialize<'de> for PitchClass {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u8::deserialize(deserializer)?;
        if value >= ET_SIZE as u8 {
            return Err(serde::de::Error::custom(format!(
                "pitch class must be 0-11, got {}", value
            )));
        }
        Ok(PitchClass(value))
    }
}

impl From<PitchClass> for i32 {
    fn from(pc: PitchClass) -> i32 {
        pc.0 as i32
    }
}

/// String representation using Circle of Fifths naming (e.g. "F#", "Bb").
impl fmt::Display for PitchClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let chroma = self.get_chroma().value();
        const TABLE: [char; 7] = ['F', 'C', 'G', 'D', 'A', 'E', 'B'];
        let idx = ((chroma + 36) % 7) as usize;
        let letter = TABLE[idx];
        let accidental = (chroma + 36) / 7 - 5;
        match accidental {
            -2 => write!(f, "{}bb", letter),
            -1 => write!(f, "{}b", letter),
            1 => write!(f, "{}#", letter),
            2 => write!(f, "{}x", letter),
            _ => write!(f, "{}", letter),
        }
    }
}

/// Lookup table for string → PitchClass parsing.
static PITCH_CLASS_TABLE: Lazy<HashMap<&'static str, PitchClass>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("A-", PitchClass::AB);
    m.insert("A", PitchClass::A);
    m.insert("A#", PitchClass::AS);
    m.insert("B-", PitchClass::BB);
    m.insert("B", PitchClass::B);
    m.insert("C", PitchClass::C);
    m.insert("Cs", PitchClass::CS);
    m.insert("C#", PitchClass::CS);
    m.insert("D-", PitchClass::DB);
    m.insert("D", PitchClass::D);
    m.insert("D#", PitchClass::DS);
    m.insert("E-", PitchClass::EB);
    m.insert("E", PitchClass::E);
    m.insert("F", PitchClass::F);
    m.insert("F#", PitchClass::FS);
    m.insert("G-", PitchClass::GB);
    m.insert("G", PitchClass::G);
    m.insert("G#", PitchClass::GS);
    m
});

#[derive(Debug, PartialEq, Eq)]
pub struct ParsePitchClassError(String);

impl fmt::Display for ParsePitchClassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cannot find pitch class \"{}\"", self.0)
    }
}

impl FromStr for PitchClass {
    type Err = ParsePitchClassError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PITCH_CLASS_TABLE
            .get(s)
            .copied()
            .ok_or_else(|| ParsePitchClassError(s.to_string()))
    }
}

/// Interval (ascending) in semitones between two pitch classes.
pub fn get_interval(from: PitchClass, to: PitchClass) -> Semitone {
    Semitone(((ET_SIZE as i32) + to.value() as i32 - from.value() as i32) % (ET_SIZE as i32))
}

/// Distance on the Circle of Fifths between two pitch classes.
/// Uses the identity 7*7 ≡ 1 (mod 12).
pub fn get_circle_of_fifth_distance(from: PitchClass, to: PitchClass) -> COFUnit {
    COFUnit(7 * get_interval(from, to).value() % (ET_SIZE as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chroma_values() {
        assert_eq!(PitchClass::C.get_chroma(), Chroma(0));
        assert_eq!(PitchClass::G.get_chroma(), Chroma(1));
        assert_eq!(PitchClass::D.get_chroma(), Chroma(2));
        assert_eq!(PitchClass::F.get_chroma(), Chroma(-1));
    }

    #[test]
    fn pitch_class_value() {
        assert_eq!(PitchClass::B.value(), 11);
    }

    #[test]
    fn pitch_class_to_string() {
        assert_eq!(PitchClass::C.to_string(), "C");
        assert_eq!(PitchClass::FS.to_string(), "F#");
        assert_eq!(PitchClass::BB.to_string(), "Bb");
        assert_eq!(PitchClass::G.to_string(), "G");
        assert_eq!(PitchClass::D.to_string(), "D");
        assert_eq!(PitchClass::A.to_string(), "A");
        assert_eq!(PitchClass::E.to_string(), "E");
        assert_eq!(PitchClass::B.to_string(), "B");
        assert_eq!(PitchClass::F.to_string(), "F");
    }

    #[test]
    fn pitch_class_from_str() {
        assert_eq!("C".parse::<PitchClass>().unwrap(), PitchClass::C);
        assert_eq!("F#".parse::<PitchClass>().unwrap(), PitchClass::FS);
        assert_eq!("A-".parse::<PitchClass>().unwrap(), PitchClass::AB);
        assert!("X".parse::<PitchClass>().is_err());
    }

    #[test]
    fn circle_of_fifth_distance() {
        assert_eq!(
            get_circle_of_fifth_distance(PitchClass::C, PitchClass::G),
            COFUnit(1)
        );
        assert_eq!(
            get_circle_of_fifth_distance(PitchClass::C, PitchClass::D),
            COFUnit(2)
        );
    }

    #[test]
    fn interval() {
        assert_eq!(get_interval(PitchClass::C, PitchClass::G), Semitone(7));
        assert_eq!(get_interval(PitchClass::C, PitchClass::E), Semitone(4));
        assert_eq!(get_interval(PitchClass::E, PitchClass::C), Semitone(8));
    }
}
