use std::fmt;

/// Octave of a pitch, from -1 to 7 (matching MIDI convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i8)]
pub enum Octave {
    OMinus1 = -1,
    O0 = 0,
    O1 = 1,
    O2 = 2,
    O3 = 3,
    O4 = 4,
    O5 = 5,
    O6 = 6,
    O7 = 7,
}

impl Octave {
    pub fn from_i8(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Octave::OMinus1),
            0 => Some(Octave::O0),
            1 => Some(Octave::O1),
            2 => Some(Octave::O2),
            3 => Some(Octave::O3),
            4 => Some(Octave::O4),
            5 => Some(Octave::O5),
            6 => Some(Octave::O6),
            7 => Some(Octave::O7),
            _ => None,
        }
    }

    pub fn to_i8(self) -> i8 {
        self as i8
    }
}

impl fmt::Display for Octave {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_i8())
    }
}
