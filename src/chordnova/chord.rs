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
use crate::chordnova::pitch::{ParsePitchError, Pitch, PitchClass};
use crate::chordnova::util::iterable_to_str;

#[allow(dead_code)]
pub enum OverflowState {
    NoOverflow,
    Single,
    Total,
}

#[allow(dead_code)]
pub enum OutputMode {
    Both,
    MidiOnly,
    TextOnly,
}

#[allow(dead_code)]
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

    #[allow(dead_code)]
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

impl Clone for CNChord {
    fn clone(&self) -> Self {
        CNChord::from_notes(&self._pitches, false)
    }
}

impl CNChord {
    // pub fn new() -> Self {
    //     Self {
    //         _pitches: Vec::new(),
    //
    //     }
    // }

    /// See also
    ///     Chord(const vector<int>& _notes, double _chroma_old = 0.0);
    /// in original C++ implementation
    pub fn from_notes(notes: &Vec<Pitch>, dedup: bool) -> CNChord {
        match dedup {
            true => CNChord {
                _pitches: notes.into_iter().sorted().dedup().map(|pitch| *pitch).collect(),
            },
            false => CNChord {
                _pitches: notes.into_iter().sorted().map(|pitch| *pitch).collect(),
            }
        }
    }

    /// an integer representing 'note_set'; unique for different 'note_set's
    /// Assign a unique id for each pitch set (according to set theory)
    /// See also: https://web.mit.edu/music21/doc/moduleReference/moduleChord.html#music21.chord.Chord.chordTablesAddress
    #[allow(dead_code)]
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

    /// traverse some inversions
    #[allow(dead_code)] // It is used in test cases
    pub fn find_vec_simple(&self, new_chord: &CNChord) -> Result<(CNChord, CNChord), ParseCNChordError> {
        // Corresponding to the original implementation in c++
        // 1. Consider all possible inversions.
        // 2. Consider two octives
        // 3. Always invert the second chord
        let base = itertools::iproduct!((-1i8..1i8), (0..new_chord._pitches.len()));
        match base.min_by_key(|inversion_map| {
            match new_chord.apply_inversion(inversion_map.0, inversion_map.1) {
                Ok(inverted_new_chord) => match self.diff(&inverted_new_chord) {
                    Ok(p) => p.sv,
                    Err(_) => u16::MAX
                },
                Err(_) => u16::MAX
            }
        }) {
            Some(selected_inversion_map) => self.find_best_chord_pairs(&new_chord.apply_inversion(selected_inversion_map.0, selected_inversion_map.1).unwrap()),
            None => Err(ParseCNChordError { msg: String::from("Unknown Error") })
        }
    }

    /// traverse all inversions
    #[allow(dead_code)]
    pub fn find_vec_by_pitch_class(&self, new_chord: &CNChord) -> Result<(CNChord, CNChord), ParseCNChordError> {
        // convert to pitch classes
        let pitch_classes = new_chord.get_pitch_classes();
        let mut unused_pitch_classes = pitch_classes.to_vec();
        let mut new_pitches = vec! {};
        for pitch in &self._pitches {
            let new_pitch = pitch.get_nearest_pitch_by_pitch_class(&unused_pitch_classes);
            new_pitches.push(new_pitch);
            let pos = unused_pitch_classes.iter().position(|unused_pitch_class| *unused_pitch_class == new_pitch.get_pitch_class()).unwrap();
            unused_pitch_classes.remove(pos);
        }
        let new_chord = CNChord::from_notes(&new_pitches, false);
        println!("{}", new_chord);
        Ok(((*self).clone(), new_chord))
    }

    /// interface of '_find_vec'
    ///
    /// See Also:
    ///     void find_vec(Chord& new_chord, bool in_analyser = false, bool in_substitution = false);
    /// In original C++ Implementation
    #[allow(dead_code)]
    pub fn find_vec(&self, new_chord: &CNChord, _in_analyser: bool, in_substitution: bool) -> Result<(CNChord, CNChord), ParseCNChordError> {
        match in_substitution {
            false => self.find_best_chord_pairs(new_chord),
            true => self.find_vec_simple(new_chord),
            // true => self.find_vec_by_pitch_class(new_chord)
        }
    }

    pub fn get_pitch_classes(&self) -> Vec<PitchClass> {
        self._pitches.iter().map(|pitch| pitch.get_pitch_class()).dedup().collect::<Vec<PitchClass>>()
    }

    /// n; size of notes
    pub fn t_size(&self) -> usize {
        return self._pitches.len();
    }

    pub fn apply_inversion(&self, octive: i8, inversion: usize) -> Result<CNChord, ParseCNChordError> {
        // handling inversion
        // handling octive
        let octive_to_shift_due_to_inversion = match inversion {
            0 => 0,
            _ => (self._pitches[self._pitches.len() - 1].0 - self._pitches[inversion - 1].0 + 12 - 1) / 12
        };
        let new_pitches = self._pitches[inversion..self._pitches.len()].iter().cloned().chain(self._pitches[0..inversion].iter().map(|i| *i + i8::try_from(12 * octive_to_shift_due_to_inversion).unwrap())).map(|item| item + i8::try_from(12 * octive).unwrap()).collect();
        return Ok(CNChord::from_notes(&new_pitches, false));
    }

    pub fn apply_expansion(&self, expansion_map: &Vec<&usize>, total_size: usize) -> CNChord {
        // FIXME: return Result instead
        let mut ret = vec! {};
        let mut counter: usize = 0;
        for item in 1..(total_size + 1) {
            ret.push(Pitch(self._pitches[counter].0));
            if counter < expansion_map.len() && *expansion_map[counter] == item {
                counter += 1;
            }
        }
        let c = CNChord::from_notes(&ret, false);
        return c;
    }

    /// traverse all combinations
    pub fn find_best_chord_pairs(&self, chord: &CNChord) -> Result<(CNChord, CNChord), ParseCNChordError> {
        if self.t_size() == chord.t_size() {
            return Ok(((*self).clone(), (*chord).clone()));
        } else if self.t_size() > chord.t_size() {
            match chord.find_best_chord_pairs(self) {
                Ok((f, s)) => Ok((s, f)),
                Err(e) => Err(e)
            }
        } else {
            let base = (1..(chord.t_size())).collect::<Vec<usize>>();
            let base2 = base.iter().combinations(self.t_size() - 1).collect::<Vec<Vec<&usize>>>();
            let selected_expansion_map = base2.iter().min_by_key(|expansion_map| {
                let expanded_chord = self.apply_expansion(expansion_map, chord.t_size());
                assert_eq!(expanded_chord.t_size(), chord.t_size(), "expansion_map: {}", iterable_to_str(expansion_map.iter()));
                match chord.diff(&expanded_chord) {
                    Ok(p) => p.sv,
                    Err(_) => u16::MAX
                }
            });
            match selected_expansion_map {
                Some(p) => {
                    let expanded_chord = self.apply_expansion(p, chord.t_size());
                    assert_eq!(expanded_chord.t_size(), chord.t_size());
                    return Ok((expanded_chord, (*chord).clone()));
                }
                None => Err(ParseCNChordError { msg: String::from("Unknown Error") })
            }
        }
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
                    Err(_) => u16::MAX
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
                        rule => Err(ParseCNChordError {
                            msg: String::from(format!("Unknown rule {:?}", rule))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting1() {
        let c_major: CNChord = CNChord::from_str("C4 E4 G4").unwrap();
        assert_eq!(c_major.to_string(), "C4, E4, G4");
    }

    #[test]
    fn formatting2() {
        let c_dominant_7: CNChord = CNChord::from_str("C4 E4 G4 B-4").unwrap();
        assert_eq!(c_dominant_7.to_string(), "C4, E4, G4, B-4");
    }

    #[test]
    fn diff1() {
        let c_major: CNChord = CNChord::from_str("C4 E4 G4").unwrap();
        let c_dominant_7: CNChord = CNChord::from_str("C4 E4 G4 B-4").unwrap();
        assert_eq!(c_major.diff(&c_dominant_7).unwrap().to_string(), "<ChordDiff: [0, 0, 0, -3], sv: 3, norm: 3.00>");
    }

    #[test]
    fn find_best_chord_pairs_1() {
        let c_major: CNChord = CNChord::from_str("C4 E4 G4").unwrap();
        let c_dominant_7: CNChord = CNChord::from_str("C4 E4 G4 B-4").unwrap();
        let result_tuple = c_major.find_best_chord_pairs(&c_dominant_7).unwrap();
        assert_eq!(result_tuple.0.to_string(), "C4, E4, G4, G4");
        assert_eq!(result_tuple.1.to_string(), "C4, E4, G4, B-4");
    }

    #[test]
    fn find_vec1_1() {
        let c_major: CNChord = CNChord::from_str("C4 E4 G4").unwrap();
        let f_major: CNChord = CNChord::from_str("F3 A3 C4").unwrap();
        let result_tuple = c_major.find_vec_simple(&f_major).unwrap();
        assert_eq!(result_tuple.0.to_string(), "C4, E4, G4");
        assert_eq!(result_tuple.1.to_string(), "C4, F4, A4");
    }

    #[test]
    fn find_vec1_2() {
        let c_major: CNChord = CNChord::from_str("C4 E4 G4").unwrap();
        let f_major: CNChord = CNChord::from_str("F3 A3 C4").unwrap();
        let result_tuple = c_major.find_vec_by_pitch_class(&f_major).unwrap();
        assert_eq!(result_tuple.0.to_string(), "C4, E4, G4");
        assert_eq!(result_tuple.1.to_string(), "C4, F4, A4");
    }

    #[test]
    fn find_vec2_1() {
        let b_diminished: CNChord = CNChord::from_str("B3 D4 F4").unwrap();
        let g_major: CNChord = CNChord::from_str("G4 B4 D5").unwrap();
        let result_tuple = b_diminished.find_vec_simple(&g_major).unwrap();
        assert_eq!(result_tuple.0.to_string(), "B3, D4, F4");
        assert_eq!(result_tuple.1.to_string(), "B3, D4, G4");
    }

    #[test]
    fn find_vec2_2() {
        let b_diminished: CNChord = CNChord::from_str("B3 D4 F4").unwrap();
        let g_major: CNChord = CNChord::from_str("G4 B4 D5").unwrap();
        let result_tuple = b_diminished.find_vec_by_pitch_class(&g_major).unwrap();
        assert_eq!(result_tuple.0.to_string(), "B3, D4, F4");
        assert_eq!(result_tuple.1.to_string(), "B3, D4, G4");
    }

    #[test]
    fn find_vec3_2() {
        let c_major: CNChord = CNChord::from_str("C3 G3 E4 C5").unwrap();
        let c_major_seventh: CNChord = CNChord::from_str("C3 E3 G3 B3").unwrap();
        let result_tuple = c_major.find_vec_by_pitch_class(&c_major_seventh).unwrap();
        assert_eq!(result_tuple.0.to_string(), "C3, G3, E4, C5");
        assert_eq!(result_tuple.1.to_string(), "C3, G3, E4, B4");
    }

    #[test]
    fn find_vec4_1() {
        let c_major: CNChord = CNChord::from_str("C3 E3 G3").unwrap();
        let c_major_seventh: CNChord = CNChord::from_str("C3 E3 G3 B3").unwrap();
        let result_tuple = c_major.find_vec_simple(&c_major_seventh).unwrap();
        assert_eq!(result_tuple.0.to_string(), "C3, C3, E3, G3");
        assert_eq!(result_tuple.1.to_string(), "B2, C3, E3, G3");
    }

    #[test]
    fn find_vec4_2() {
        let c_major: CNChord = CNChord::from_str("C3 E3 G3").unwrap();
        let c_major_seventh: CNChord = CNChord::from_str("C3 E3 G3 B3").unwrap();
        let result_tuple = c_major.find_vec_by_pitch_class(&c_major_seventh).unwrap();
        assert_eq!(result_tuple.0.to_string(), "C3, C3, E3, G3");
        assert_eq!(result_tuple.1.to_string(), "B2, C3, E3, G3");
    }
}
