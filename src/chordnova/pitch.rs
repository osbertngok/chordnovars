use crate::chordnova::pitchparser::pest::Parser;
use crate::chordnova::pitchparser::Rule;
use crate::chordnova::pitchparser::PitchParser;

use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;
use itertools::iproduct;

pub enum Stepname {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl fmt::Display for Stepname {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Stepname::C => "C",
            Stepname::D => "D",
            Stepname::E => "E",
            Stepname::F => "F",
            Stepname::G => "G",
            Stepname::A => "A",
            Stepname::B => "B",
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseStepnameError;

impl FromStr for Stepname {
    type Err = ParseStepnameError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A" => Ok(Stepname::A),
            "B" => Ok(Stepname::B),
            "C" => Ok(Stepname::C),
            "D" => Ok(Stepname::D),
            "E" => Ok(Stepname::E),
            "F" => Ok(Stepname::F),
            "G" => Ok(Stepname::G),
            _ => Err(ParseStepnameError {})
        }
    }
}

pub enum Accidental {
    Natural,
    Flat,
    Sharp,
}

impl fmt::Display for Accidental {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Accidental::Natural => "",
            Accidental::Sharp => "#",
            Accidental::Flat => "-",
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseAccidentalError;

impl FromStr for Accidental {
    type Err = ParseAccidentalError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "#" => Ok(Accidental::Sharp),
            "-" => Ok(Accidental::Flat),
            "" => Ok(Accidental::Natural),
            _ => Err(ParseAccidentalError {})
        }
    }
}

#[derive(Debug)]
#[derive(Clone)]
pub struct PitchClass(u8);

impl PartialEq for PitchClass {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}


impl PitchClass {
    pub fn is_natural(&self) -> bool {
        return vec![0, 2, 4, 5, 7, 9, 11].contains(&self.0);
    }

    pub fn is_sharpable(&self) -> bool {
        return vec![1, 6, 8].contains(&self.0);
    }

    pub fn is_flatable(&self) -> bool {
        return vec![3, 10].contains(&self.0);
    }

    pub fn to_step_name(&self) -> (Stepname, Accidental) {
        (match self.0 {
            0 | 1 => Stepname::C,
            2 => Stepname::D,
            3 | 4 => Stepname::E,
            5 | 6 => Stepname::F,
            7 | 8 => Stepname::G,
            9 => Stepname::A,
            10 | 11 => Stepname::B,
            12_u8..=u8::MAX => unreachable!("Unknown pitch class {}", self.0)
        }, if self.is_natural() {
            Accidental::Natural
        } else if self.is_sharpable() {
            Accidental::Sharp
        } else if self.is_flatable() { Accidental::Flat } else {
            unreachable!()
        })
    }
}


pub fn convert_ps_to_step(midi_note_number: u8) -> (Stepname, Accidental, Option<u8>) {
    // from https://github.com/cuthbertLab/music21/blob/master/music21/pitch.py
    let pitch_class = PitchClass(midi_note_number % 12);
    let (step_name, accidental) = pitch_class.to_step_name();
    let octive = midi_note_number / 12;
    return (step_name, accidental, match octive {
        1..=u8::MAX => Some(octive - 1),
        0 => None
    });
}


#[derive(PartialOrd)]
#[derive(Ord)]
pub struct Pitch(pub u8);

impl Add<i8> for Pitch {
    type Output = Pitch;

    fn add(self, rhs: i8) -> Self::Output {
        Self {
            0: u8::try_from(i8::try_from(self.0).unwrap() + rhs).unwrap()
        }
    }
}

impl Sub<i8> for Pitch {
    type Output = Pitch;

    fn sub(self, rhs: i8) -> Self::Output {
        Self {
            0: u8::try_from(i8::try_from(self.0).unwrap() - rhs).unwrap()
        }
    }
}

impl Copy for Pitch {}

impl Clone for Pitch {
    fn clone(&self) -> Self {
        Pitch(self.0)
    }
}

impl Pitch {
    pub fn convert_ps_to_step(&self) -> (Stepname, Accidental, Option<u8>) {
        convert_ps_to_step(self.0)
    }

    pub fn get_name(&self) -> String {
        let (stepname, acc, octive) = self.convert_ps_to_step();
        format!("{}{}{}", stepname, acc, match octive {
            Some(t) => t.to_string(),
            None => String::new()
        })
    }

    pub fn get_pitch_class(&self) -> PitchClass {
        PitchClass(self.0 % 12)
    }

    pub fn get_nearest_pitch_by_pitch_class(&self, pitch_classes: &Vec<PitchClass>) -> Pitch {
        for (offset, direction) in iproduct!((0..12), vec![-1i8, 1i8]) {
            println!("Trying {} | {}", direction, offset);
            let selected_pitch = *self + direction * offset;
            match (*pitch_classes).iter().position(|pitch_class| *pitch_class == selected_pitch.get_pitch_class()) {
                Some(_) => {
                    println!("{}, {:?} == {:?}", selected_pitch, selected_pitch.get_pitch_class(), pitch_classes);
                    return selected_pitch;
                }
                None => {}
            }
            continue;
        }
        unreachable!()
    }

    pub fn from_stepname(stepname: Stepname, accidental: Accidental, octive: Option<u8>) -> Pitch {
        let midi_note: i16 = match stepname {
            Stepname::A => 9_i16,
            Stepname::B => 11_i16,
            Stepname::C => 0_i16,
            Stepname::D => 2_i16,
            Stepname::E => 4_i16,
            Stepname::F => 5_i16,
            Stepname::G => 7_i16
        } + match accidental {
            Accidental::Natural => 0_i16,
            Accidental::Sharp => 1_i16,
            Accidental::Flat => -1_i16
        } + match octive {
            Some(p) => (i16::from(p) + 1) * 12_i16,
            None => 0
        };
        Pitch(u8::try_from(midi_note).unwrap())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParsePitchError {
    msg: String,
}

impl fmt::Display for ParsePitchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<ParsePitchError: {}>", self.msg)
    }
}

impl FromStr for Pitch {
    type Err = ParsePitchError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let wrapped_pairs = PitchParser::parse(Rule::PITCH, s);
        match wrapped_pairs {
            Ok(mut pairs) => {
                match pairs.next() {
                    Some(pair) => match pair.as_rule() {
                        Rule::PITCH => {
                            let mut pitch_pair = pair.into_inner();
                            let stepname: Stepname = pitch_pair.next().unwrap().as_str().parse().unwrap();
                            let accidental: Accidental = pitch_pair.next()
                                .unwrap()
                                .as_str().parse().unwrap();
                            let octive: Option<u8> = match pitch_pair.next()
                                .unwrap()
                                .as_str() {
                                "" => None,
                                p => Some(p.parse::<u8>().unwrap().to_owned())
                            };
                            Ok(Pitch::from_stepname(stepname, accidental, octive))
                        }
                        rule => Err(ParsePitchError {
                            msg: String::from(format!("Unknown rule {:?}", rule))
                        })
                    },
                    None => Err(ParsePitchError {
                        msg: String::from("")
                    })
                }
            }
            Err(_) => Err(ParsePitchError {
                msg: String::from("Unknown Parse Pitch Error")
            })
        }
    }
}

impl Eq for Pitch {}

impl PartialEq for Pitch {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}
