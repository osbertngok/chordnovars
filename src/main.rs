mod chordnova {
    pub mod chord;
}

use crate::chordnova::chord::{CNChord, OverflowState};

fn main() {
    let chord = CNChord{
        _notes: Vec::new(),
        _voice_leading_max: 0,
        s_size: 0,
        tension: 0.0,
        thickness: 0.0,
        root: 0,
        g_center: 0,
        span: 0,
        sspan: 0,
        similarity: 0,
        _chroma_old: 0.0,
        chroma: 0.0,
        q_indicator: 0.0,
        common_note: 0,
        sv: 0,
        overflow_state: OverflowState::NoOverflow,
        hide_octave: false,
        name: None,
        name_with_octave: None,
        vec: vec![],
        self_diff: vec![],
        count_vec: vec![],
        ref_chord: None,
        _dirty: false
    };
    println!("{}", chord);
    println!("Hello world!");
}