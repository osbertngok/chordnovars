/*
   ChordNova v3.0 [Build: 2021.1.14]
   (c) 2020 Wenge Chen, Ji-woon Sim.
   Port to Rust by osbertngok
 */

use crate::chordnova::pitchparser::pest::Parser;
use crate::chordnova::pitchparser::Rule;
use crate::chordnova::pitchparser::PitchParser;


use std::fmt;
use std::str::FromStr;
use std::rc::Rc;
use itertools::Itertools;
use crate::chordnova::pitch::{ParsePitchError, Pitch};
use crate::chordnova::util::iterable_to_str;


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

pub struct CNChordExtendedData {
    /// Range of Movement, refers to Chord.vlmax
    pub _voice_leading_max: i64,
    /// m; size of note_set
    pub s_size: i16,
    /// t
    pub tension: f32,
    /// h
    pub thickness: f32,
    /// r
    pub root: i16,
    /// g
    pub g_center: i16,
    /// s
    pub span: i16,
    /// ss
    pub sspan: i16,
    /// x
    pub similarity: i16,
    /// kk
    pub _chroma_old: f32,
    /// k
    pub chroma: f32,
    /// Q
    pub q_indicator: f32,
    /// c
    pub common_note: i16,
    /// sum of vec, Σvec
    pub sv: i16,
    pub overflow_state: OverflowState,
    pub hide_octave: bool,
    ///name of each note in the chord
    pub name: Option<String>,
    /// name and octave of each note in the chord
    pub name_with_octave: Option<String>,
    /// v
    pub vec: Vec<i16>,
    /// d
    pub self_diff: Vec<i16>,
    /// vec
    pub count_vec: Vec<i16>,
    /// reference chord to calculate chroma_old. This is to replace prev_chroma_old
    pub ref_chord: Option<Rc<CNChord>>,
}

pub struct ChordDiff {
    pub diff_vec: Vec<i16>,
    /// sum of (absolute value) of (diff) vector
    pub sv: u16,
    /// norm of the diff vector. Penalize large diff more.
    pub norm: f64,
}

impl Clone for ChordDiff {
    fn clone(&self) -> Self {
        ChordDiff::new(self.diff_vec.iter().map(|x| *x).collect::<Vec<i16>>())
    }
}

impl ChordDiff {
    pub fn new(diff_vec: Vec<i16>) -> Self {
        let sv = ChordDiff::sv(&diff_vec);
        let norm = ChordDiff::norm(&diff_vec);
        ChordDiff {
            diff_vec,
            sv,
            norm,
        }
    }

    /// sum of (absolute value) of (diff) vector
    /// Measuring the distance of two chords in the original cpp ChordNova implementation
    fn sv(diff_vec: &Vec<i16>) -> u16 {
        u16::try_from(diff_vec.iter().map(|x| (*x).abs() as u32).sum::<u32>()).unwrap()
    }

    /// norm of the diff vector. Penalize large diff more.
    fn norm(diff_vec: &Vec<i16>) -> f64 {
        (diff_vec.iter().map(|x| (*x as i32).pow(2)).sum::<i32>() as f64).sqrt()
    }

    fn negate(&self) -> ChordDiff {
        ChordDiff::new(self.diff_vec.iter().map(|x| -*x).collect::<Vec<i16>>())
    }
}

impl fmt::Display for ChordDiff {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<ChordDiff: {}, sv: {}, norm: {:.2}>", iterable_to_str(&self.diff_vec), self.sv, self.norm)
    }
}

/// Implementation of Chord based on
///  chorddata.h / chorddata.cpp
/// of original C++ implementation
///
/// But:
///  1. Attempt to utilize music21 to avoid re-inventing the wheels
///  2. It only contains model information;
///     generation logics are separated into a standalone module.
pub struct CNChord {
    pub _pitches: Vec<Pitch>,
}

impl CNChord {
    pub fn new() -> Self {
        Self {
            _pitches: Vec::new(),

        }
    }

    /// See also
    ///     Chord(const vector<int>& _notes, double _chroma_old = 0.0);
    /// in original C++ implementation
    pub fn from_notes(notes: Vec<Pitch>, dedup: bool) -> CNChord {
        match dedup {
            true => CNChord {
                _pitches: notes.into_iter().sorted().dedup().collect(),
            },
            false => CNChord {
                _pitches: notes.into_iter().sorted().collect(),
            }
        }
    }

    /// an integer representing 'note_set'; unique for different 'note_set's
    /// Assign a unique id for each pitch set (according to set theory)
    /// See also: https://web.mit.edu/music21/doc/moduleReference/moduleChord.html#music21.chord.Chord.chordTablesAddress
    pub fn set_id(&self) -> i64 {
        unimplemented!();
    }

    // pub fn voice_leading_max(&self) -> i64 {
    //     /*
    //        See also
    //          void _set_vl_max(const int&);
    //        in original C++ Implementation
    //      */
    //     return self._voice_leading_max;
    // }

    /// interface of '_find_vec'
    ///
    /// See Also:
    ///     void find_vec(Chord& new_chord, bool in_analyser = false, bool in_substitution = false);
    /// In original C++ Implementation
    pub fn find_vec(&self, in_analyser: bool, in_substitution: bool) -> CNChord {
        unimplemented!();
    }

    /// n; size of notes
    pub fn t_size(&self) -> usize {
        return self._pitches.len();
    }

    pub fn apply_expansion(&self, expansion_map: &Vec<&usize>, total_size: usize) -> CNChord {
        let mut ret = vec! {};
        let mut counter: usize = 0;
        for item in 1..(total_size + 1) {
            ret.push(Pitch(self._pitches[counter].0));
            if counter < expansion_map.len() && *expansion_map[counter] == item {
                counter += 1;
            }
        }
        let c = CNChord::from_notes(ret, false);
        return c;
    }

    pub fn diff(&self, chord: &CNChord) -> Result<ChordDiff, ParseCNChordError> {
        if self.t_size() == chord.t_size() {
            Ok(ChordDiff::new(
                (0..(self.t_size().min(chord.t_size()))).map(|index| i16::from(chord._pitches[index].0) - i16::from(self._pitches[index].0)).collect()
            ))
        } else if self.t_size() > chord.t_size() {
            match chord.diff(self) {
                Ok(p) => Ok(p.negate()),
                Err(p) => Err(p)
            }
        } else {
            let base = (1..(chord.t_size())).collect::<Vec<usize>>();
            let base2 = base.iter().combinations(self.t_size() - 1).collect::<Vec<Vec<&usize>>>();
            let selected_expansion_map = base2.iter().min_by_key(|expansion_map| {
                let expanded_chord = self.apply_expansion(expansion_map, chord.t_size());
                assert_eq!(expanded_chord.t_size(), chord.t_size(), "expansion_map: {}", iterable_to_str(expansion_map.iter()));
                match chord.diff(&expanded_chord) {
                    Ok(p) => p.sv,
                    Err(p) => u16::MAX
                }
            });
            match selected_expansion_map {
                Some(p) => {
                    let expanded_chord = self.apply_expansion(p, chord.t_size());
                    assert_eq!(expanded_chord.t_size(), chord.t_size());
                    chord.diff(&expanded_chord)
                }
                None => Err(ParseCNChordError { msg: String::from("Unknown Error") })
            }
        }
    }
}

impl fmt::Display for CNChord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self._pitches.iter().map(|pitch: &Pitch| pitch.get_name()).join(", "))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseCNChordError {
    msg: String,
}

impl FromStr for CNChord {
    type Err = ParseCNChordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let wrapped_pairs = PitchParser::parse(Rule::FULL_PITCHES, s);
        match wrapped_pairs {
            Ok(mut pairs) => {
                match pairs.next() {
                    Some(pair) => match pair.as_rule() {
                        Rule::PITCHES => {
                            let pitches: Result<Vec<Pitch>, ParsePitchError> = pair.into_inner().map(|p| Pitch::from_str(p.as_str())).collect();
                            match pitches {
                                Ok(pcs) => Ok(CNChord { _pitches: pcs }),
                                Err(e) => Err(ParseCNChordError {
                                    msg: e.to_string()
                                })
                            }
                        }
                        e => Err(ParseCNChordError {
                            msg: String::from("Hi")
                        })
                    },
                    None => Err(ParseCNChordError {
                        msg: String::from(format!("{:?}", pairs))
                    })
                }
            }
            Err(e) => Err(ParseCNChordError { msg: String::from(e.to_string()) })
        }
    }
}
