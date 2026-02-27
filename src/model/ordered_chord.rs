use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use serde::{Serialize, Deserialize, Serializer, Deserializer};

use crate::model::pitch::Pitch;
use crate::model::pitch_class::PitchClass;
use crate::model::pitch_iterable::PitchIterable;
use crate::model::pitch_set::PitchSet;

/// An ordered list of pitches (may contain duplicates).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedChord {
    pitches: Vec<Pitch>,
}

impl OrderedChord {
    pub fn new(pitches: Vec<Pitch>) -> Self {
        OrderedChord { pitches }
    }

    pub fn from_str_space_separated(s: &str) -> Result<Self, String> {
        let pitches: Result<Vec<Pitch>, _> = s
            .split_whitespace()
            .map(|p| p.parse::<Pitch>().map_err(|e| e.to_string()))
            .collect();
        Ok(OrderedChord::new(pitches?))
    }

    pub fn get_num_of_pitches(&self) -> usize {
        self.pitches.len()
    }

    pub fn get_num_of_unique_pitches(&self) -> usize {
        self.pitches
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub fn to_set(&self) -> PitchSet {
        PitchSet::new(self.pitches.iter().copied())
    }
}

impl PartialOrd for OrderedChord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedChord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.pitches.len()
            .cmp(&other.pitches.len())
            .then_with(|| self.pitches.cmp(&other.pitches))
    }
}

impl PitchIterable for OrderedChord {
    fn contains_pitch_class(&self, pc: PitchClass) -> bool {
        self.pitches.iter().any(|p| p.get_pitch_class() == pc)
    }

    fn contains_pitch(&self, pitch: Pitch) -> bool {
        self.pitches.contains(&pitch)
    }

    fn get_pitches(&self) -> Vec<Pitch> {
        self.pitches.clone()
    }

    fn get_pitch_classes_ordered_by_circle_of_fifths(&self) -> Vec<PitchClass> {
        self.to_set().get_pitch_classes_ordered_by_circle_of_fifths()
    }

    fn get_tension(&self) -> f64 {
        self.to_set().get_tension()
    }

    fn get_thickness(&self) -> f64 {
        self.to_set().get_thickness()
    }

    fn get_geometrical_center(&self) -> f64 {
        self.to_set().get_geometrical_center()
    }

    fn find_root(&self) -> Option<PitchClass> {
        self.to_set().find_root()
    }
}

impl Serialize for OrderedChord {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let midi_numbers: Vec<u8> = self.pitches.iter().map(|p| p.get_number()).collect();
        midi_numbers.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OrderedChord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let midi_numbers = Vec::<u8>::deserialize(deserializer)?;
        Ok(OrderedChord::new(midi_numbers.into_iter().map(Pitch::new).collect()))
    }
}

impl fmt::Display for OrderedChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.pitches.iter().map(|p| p.to_string()).collect();
        write!(f, "{}", parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::pitch_iterable::PitchIterable;

    fn oc(s: &str) -> OrderedChord {
        OrderedChord::from_str_space_separated(s).unwrap()
    }

    #[test]
    fn contains_pitch_class() {
        let chord = oc("C4 E4 G4");
        assert!(chord.contains_pitch_class(PitchClass::C));
        assert!(!chord.contains_pitch_class(PitchClass::D));
    }

    #[test]
    fn ordering() {
        let c1 = oc("C4 E4 G4");
        let c2 = oc("C4 E4 A4");
        assert!(c1 < c2);
    }

    #[test]
    fn num_of_pitches() {
        let chord = oc("C4 E4 G4");
        assert_eq!(chord.get_num_of_pitches(), 3);
        assert_eq!(chord.get_num_of_unique_pitches(), 3);
    }

    #[test]
    fn display() {
        let chord = oc("C4 E4 G4");
        assert_eq!(chord.to_string(), "C4 E4 G4");
    }
}
