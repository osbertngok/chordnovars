mod chordnova {
    pub mod chord;
    pub mod pitch;
    pub mod pitchparser;
}

use crate::chordnova::chord::{CNChord, OverflowState};
use crate::chordnova::pitch::Pitch;

use std::str::FromStr;

fn main() {
    let chord: CNChord = CNChord::from_str("C4 E4 G4").unwrap();
    println!("{}", chord);
}