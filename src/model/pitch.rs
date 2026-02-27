use std::fmt;
use std::str::FromStr;

use crate::constant::ET_SIZE;
use crate::model::octave::Octave;
use crate::model::pitch_class::{Chroma, PitchClass};
use crate::parser::pitch_parser::pest::Parser;
use crate::parser::pitch_parser::{PitchParser, Rule};

/// A musical pitch, stored as a MIDI note number (0–127).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pitch(u8);

impl Pitch {
    pub fn new(midi_num: u8) -> Self {
        Pitch(midi_num)
    }

    pub fn from_pitch_class_octave(pc: PitchClass, oct: Octave) -> Self {
        let note = pc.value() as i16 + (oct.to_i8() as i16 + 1) * ET_SIZE as i16;
        Pitch(note as u8)
    }

    pub fn get_pitch_class(self) -> PitchClass {
        PitchClass::new(self.0 % ET_SIZE as u8)
    }

    pub fn get_octave(self) -> Octave {
        Octave::from_i8((self.0 / ET_SIZE as u8) as i8 - 1)
            .expect("MIDI note out of octave range")
    }

    pub fn get_number(self) -> u8 {
        self.0
    }

    pub fn get_chroma(self) -> Chroma {
        self.get_pitch_class().get_chroma()
    }
}

impl std::ops::Sub for Pitch {
    type Output = i32;
    fn sub(self, rhs: Pitch) -> i32 {
        self.0 as i32 - rhs.0 as i32
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.get_pitch_class(), self.get_octave())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParsePitchError(String);

impl fmt::Display for ParsePitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParsePitchError: {}", self.0)
    }
}

impl FromStr for Pitch {
    type Err = ParsePitchError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pairs = PitchParser::parse(Rule::PITCH, s)
            .map_err(|e| ParsePitchError(e.to_string()))?;

        let pair = pairs
            .next()
            .ok_or_else(|| ParsePitchError("no match".into()))?;

        match pair.as_rule() {
            Rule::PITCH => {
                let mut inner = pair.into_inner();
                let stepname_str = inner.next().unwrap().as_str();
                let accidental_str = inner.next().unwrap().as_str();
                let octave_str = inner.next().unwrap().as_str();

                let pc_str = format!("{}{}", stepname_str, match accidental_str {
                    "#" => "#",
                    "-" => "-",
                    _ => "",
                });

                let pitch_class: PitchClass = pc_str
                    .parse()
                    .map_err(|_| ParsePitchError(format!("unknown pitch class '{}'", pc_str)))?;

                let octave = if octave_str.is_empty() {
                    Octave::OMinus1
                } else {
                    let oct_num: i8 = octave_str
                        .parse()
                        .map_err(|_| ParsePitchError(format!("bad octave '{}'", octave_str)))?;
                    Octave::from_i8(oct_num)
                        .ok_or_else(|| ParsePitchError(format!("octave {} out of range", oct_num)))?
                };

                Ok(Pitch::from_pitch_class_octave(pitch_class, octave))
            }
            rule => Err(ParsePitchError(format!("unexpected rule {:?}", rule))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitch_serialization() {
        assert_eq!("B".parse::<Pitch>().unwrap(), Pitch::from_pitch_class_octave(PitchClass::B, Octave::OMinus1));
        assert_eq!("C4".parse::<Pitch>().unwrap(), Pitch::from_pitch_class_octave(PitchClass::C, Octave::O4));
        assert_eq!("A-5".parse::<Pitch>().unwrap(), Pitch::from_pitch_class_octave(PitchClass::AB, Octave::O5));
        assert_eq!("D#".parse::<Pitch>().unwrap(), Pitch::from_pitch_class_octave(PitchClass::DS, Octave::OMinus1));
        assert_eq!("C4".parse::<Pitch>().unwrap().get_number(), 60);
        assert_eq!("A4".parse::<Pitch>().unwrap().get_number(), 69);
    }

    #[test]
    fn pitch_to_string() {
        assert_eq!("C4".parse::<Pitch>().unwrap().to_string(), "C4");
        assert_eq!("F#5".parse::<Pitch>().unwrap().to_string(), "F#5");
    }

    #[test]
    fn pitch_chroma() {
        assert_eq!("C".parse::<Pitch>().unwrap().get_chroma(), Chroma::new(0));
        assert_eq!("G".parse::<Pitch>().unwrap().get_chroma(), Chroma::new(1));
        assert_eq!("D".parse::<Pitch>().unwrap().get_chroma(), Chroma::new(2));
    }

    #[test]
    fn pitch_subtraction() {
        let c4: Pitch = "C4".parse().unwrap();
        let e4: Pitch = "E4".parse().unwrap();
        assert_eq!(e4 - c4, 4);
        assert_eq!(c4 - e4, -4);
    }

    #[test]
    fn pitch_ordering() {
        let c4: Pitch = "C4".parse().unwrap();
        let e4: Pitch = "E4".parse().unwrap();
        assert!(c4 < e4);
    }
}
