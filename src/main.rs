mod chordnova {
    pub mod chord;
    pub mod pitch;
    pub mod pitchparser;
    pub mod util;
}

use crate::chordnova::chord::{CNChord, OverflowState};
use crate::chordnova::pitch::Pitch;

use std::str::FromStr;

fn main() {
    let chord1: CNChord = CNChord::from_str("C4 E4 G4").unwrap();
    let chord2: CNChord = CNChord::from_str("C4 F4 A4").unwrap();
    println!("{}", chord1);
    println!("{}", chord1.diff(&chord2).unwrap());
}