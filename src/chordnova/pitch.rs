use std::fmt;

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

pub struct PitchClass(u8);


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


pub fn convert_ps_to_step(midi_note_number: u8) -> (Stepname, Accidental, i8) {
    // from https://github.com/cuthbertLab/music21/blob/master/music21/pitch.py
    let pitch_class = PitchClass(midi_note_number % 12);
    let (step_name, accidental) = pitch_class.to_step_name();
    return (step_name, accidental, (TryInto::<i16>::try_into(midi_note_number).unwrap() / 12 - 1).try_into().unwrap());
}


#[derive(PartialOrd)]
#[derive(Ord)]
pub struct Pitch(pub u8);

impl Pitch {
    pub fn convert_ps_to_step(&self) -> (Stepname, Accidental, i8) {
        convert_ps_to_step(self.0)
    }

    pub fn get_name(&self) -> String {
        let (stepname, acc, octive) = self.convert_ps_to_step();
        format!("{}{}{}", stepname, acc, octive)
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
