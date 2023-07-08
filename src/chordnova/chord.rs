/*
   ChordNova v3.0 [Build: 2021.1.14]
   (c) 2020 Wenge Chen, Ji-woon Sim.
   Port to Rust by osbertngok
 */

use std::fmt;
use std::rc::Rc;
use itertools::Itertools;
use crate::chordnova::pitch::Pitch;


pub enum OverflowState {
    NoOverflow,
    Single,
    Total,
}


pub enum OutputMode {
    Both,
    MidiOnly,
    TextOnly,
}


pub struct CNChord {
    /*
       Implementation of Chord based on
        chorddata.h / chorddata.cpp
       of original C++ implementation

       But:
        1. Attempt to utilize music21 to avoid re-inventing the wheels
        2. It only contains model information;
           generation logics are separated into a standalone module.
     */

    pub _pitches: Vec<Pitch>,

    pub _voice_leading_max: i64,
    // Range of Movement, refers to Chord.vlmax
    pub s_size: i16,
    // m; size of note_set
    pub tension: f32,
    // t
    pub thickness: f32,
    // h
    pub root: i16,
    // r
    pub g_center: i16,
    // g
    pub span: i16,
    // s
    pub sspan: i16,
    // ss
    pub similarity: i16,
    // x
    pub _chroma_old: f32,
    // kk
    pub chroma: f32,
    // k
    pub q_indicator: f32,
    // Q
    pub common_note: i16,
    // c
    pub sv: i16,  // sv, Σvec

    pub overflow_state: OverflowState,
    pub hide_octave: bool,
    pub name: Option<String>,
    //name of each note in the chord
    pub name_with_octave: Option<String>,
    // name and octave of each note in the chord
    pub vec: Vec<i16>,
    // v
    pub self_diff: Vec<i16>,
    // d
    pub count_vec: Vec<i16>,  // vec

    pub ref_chord: Option<Rc<CNChord>>,  // reference chord to calculate chroma_old. This is to replace prev_chroma_old

    /*
       We want to make evaluation lazy. Evaluation won't be triggered
       until property is accessed.
     */
    pub _dirty: bool,
}

impl CNChord {
    pub fn new() -> Self {
        Self {
            _pitches: Vec::new(),
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
            vec: Vec::new(),
            self_diff: Vec::new(),
            count_vec: Vec::new(),
            ref_chord: None,
            _dirty: false,
        }
    }

    pub fn from_notes(notes: Vec<Pitch>, ref_chord: Option<Rc<CNChord>>) -> CNChord {
        /*
            See also
                Chord(const vector<int>& _notes, double _chroma_old = 0.0);
            in original C++ implementation
         */
        let ret = CNChord {
            _pitches: notes.into_iter().sorted().dedup().collect(),
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
            ref_chord,
            _dirty: false,
        };
        return ret;
    }

    pub fn set_id(&self) -> i64 {
        /*
            an integer representing 'note_set'; unique for different 'note_set's
            Assign a unique id for each pitch set (according to set theory)
            See also: https://web.mit.edu/music21/doc/moduleReference/moduleChord.html#music21.chord.Chord.chordTablesAddress
         */
        unimplemented!();
    }

    pub fn voice_leading_max(&self) -> i64 {
        /*
           See also
             void _set_vl_max(const int&);
           in original C++ Implementation
         */
        return self._voice_leading_max;
    }

    pub fn find_vec(&self, in_analyser: bool, in_substitution: bool) -> CNChord {
        /*
            interface of '_find_vec'

            See Also:
                void find_vec(Chord& new_chord, bool in_analyser = false, bool in_substitution = false);
            In original C++ Implementation

            Note:

            The original function in C++ implementation is using pass by reference signature,
            which is commonly used in C++ for memory optimization
            at the expense of readability.

            In Python there is no point to follow this.
         */
        unimplemented!();
    }

    pub fn inverse_param(&self) -> () {
        /*
         # Swap
                self.prev_chroma_old, self.chroma_old = self.chroma_old, self.prev_chroma_old
                self.chroma *= -1
                self.Q_indicator *= -1
         */
        unimplemented!();
    }

    pub fn notes(&self) -> Vec<Pitch> {
        /*
           always regarded as a sorted (L -> H) vector
           TODO: materialize this so we do not need to call list comprehension and sorted function
         */
        unimplemented!();
    }

    pub fn t_size(&self) -> usize {
        /* n; size of notes */
        return self._pitches.len();
    }
}

impl fmt::Display for CNChord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self._pitches.iter().map(|pitch: &Pitch| pitch.get_name()).join(", "))
    }
}