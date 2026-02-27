use serde::{Serialize, Deserialize};

use crate::constant::ET_SIZE;
use crate::model::ordered_chord::OrderedChord;
use crate::model::pitch::Pitch;
use crate::model::pitch_iterable::PitchIterable;
use crate::service::expansion::expand_single;
use crate::utility::combinatorics::expansion_count;

/// Signed semitone movement per voice and sum of absolute distances.
///
/// Port of `VoiceLeadingResult` from `ChordNova/src/include/service/voiceleading.h`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VoiceLeadingResult {
    /// Signed semitone movement per voice (positive = ascending).
    pub vec: Vec<i32>,
    /// Sum of absolute voice-leading distances (Σ|vec[i]|).
    pub sv: i32,
}

/// Finds the optimal voice-leading vector between two chords.
///
/// When the two chords have different sizes, the smaller is expanded to match
/// the larger, trying all possible expansions and selecting the one that
/// minimises total voice-leading distance (sv).
///
/// Port of `find_voice_leading()` from `ChordNova/src/service/voiceleading.cpp`.
pub fn find_voice_leading(from: &OrderedChord, to: &OrderedChord) -> VoiceLeadingResult {
    let from_pitches = from.get_pitches();
    let to_pitches = to.get_pitches();
    let from_size = from_pitches.len();
    let to_size = to_pitches.len();

    if to_size > from_size {
        // New chord is larger: expand 'from' to match
        let len = expansion_count(from_size as i32, to_size as i32);
        let mut min_diff = i32::MAX;
        let mut best_pitches: Vec<Pitch> = vec![Pitch::new(0); to_size];

        for i in 0..len {
            let expansion = expand_single(from, to_size, i);
            let exp_pitches = expansion.get_pitches();
            let diff: i32 = (0..to_size)
                .map(|j| (to_pitches[j].get_number() as i32 - exp_pitches[j].get_number() as i32).abs())
                .sum();
            if diff < min_diff {
                min_diff = diff;
                best_pitches = exp_pitches;
            }
        }

        let vec: Vec<i32> = (0..to_size)
            .map(|i| to_pitches[i].get_number() as i32 - best_pitches[i].get_number() as i32)
            .collect();
        VoiceLeadingResult { vec, sv: min_diff }
    } else {
        // 'from' is larger or equal: expand 'to' to match
        let len = expansion_count(to_size as i32, from_size as i32);
        let mut min_diff = i32::MAX;
        let mut best_pitches: Vec<Pitch> = vec![Pitch::new(0); from_size];

        for i in 0..len {
            let expansion = expand_single(to, from_size, i);
            let exp_pitches = expansion.get_pitches();
            let diff: i32 = (0..from_size)
                .map(|j| (exp_pitches[j].get_number() as i32 - from_pitches[j].get_number() as i32).abs())
                .sum();
            if diff < min_diff {
                min_diff = diff;
                best_pitches = exp_pitches;
            }
        }

        let vec: Vec<i32> = (0..from_size)
            .map(|i| best_pitches[i].get_number() as i32 - from_pitches[i].get_number() as i32)
            .collect();
        VoiceLeadingResult { vec, sv: min_diff }
    }
}

/// Finds voice-leading with substitution mode (octave-inversion search).
///
/// Tries all inversions (octave rotations) of the target chord and picks the
/// one with the smallest sv, provided all individual movements are <= 6 semitones.
///
/// Port of `find_voice_leading_substitution()` from `ChordNova/src/service/voiceleading.cpp`.
pub fn find_voice_leading_substitution(from: &OrderedChord, to: &OrderedChord) -> VoiceLeadingResult {
    let to_pitches = to.get_pitches();
    let size = to_pitches.len();

    let mut min_sv = i32::MAX;
    let mut min_vec: Vec<i32> = vec![];

    for i in 0..=(2 * size) {
        let mut inv_pitches: Vec<Pitch> = Vec::with_capacity(size);
        let mut out_of_range = false;

        for j in 0..size {
            let src_idx = (j + i) % size;
            let octave_shift = ((j + i) / size as usize) as i32 - 1;
            let midi = to_pitches[src_idx].get_number() as i32 + octave_shift * ET_SIZE as i32;
            if midi < 0 || midi > 127 {
                out_of_range = true;
                break;
            }
            inv_pitches.push(Pitch::new(midi as u8));
        }
        if out_of_range {
            continue;
        }

        let inversion = OrderedChord::new(inv_pitches);
        let vl = find_voice_leading(from, &inversion);

        let valid = vl.vec.iter().all(|&v| v.abs() <= 6);
        if valid && vl.sv < min_sv {
            min_sv = vl.sv;
            min_vec = vl.vec;
        }
    }

    VoiceLeadingResult { vec: min_vec, sv: if min_sv == i32::MAX { 0 } else { min_sv } }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(notes: &[u8]) -> OrderedChord {
        OrderedChord::new(notes.iter().copied().map(Pitch::new).collect())
    }

    #[test]
    fn vl_same_size_identity() {
        // Same chord: all movements should be zero
        let c = chord(&[60, 64, 67]);
        let result = find_voice_leading(&c, &c);
        assert_eq!(result.vec, vec![0, 0, 0]);
        assert_eq!(result.sv, 0);
    }

    #[test]
    fn vl_same_size_ascending() {
        // C major → G major: C→G(+7), E→B(+7), G→D(+7)
        let from = chord(&[60, 64, 67]);
        let to = chord(&[67, 71, 74]);
        let result = find_voice_leading(&from, &to);
        assert_eq!(result.vec, vec![7, 7, 7]);
        assert_eq!(result.sv, 21);
    }

    #[test]
    fn vl_size_increase_finds_minimum() {
        // C4 E4 G4 → C4 E4 G4 C5: expanding from has one good expansion
        let from = chord(&[60, 64, 67]);
        let to = chord(&[60, 64, 67, 72]);
        let result = find_voice_leading(&from, &to);
        // The best expansion doubles C4 to [C4 C4 E4 G4] — wait, that moves C4->C4(0), C4->E4(4), E4->G4(3), G4->C5(5) = 12
        // Or doubles E: [C4 E4 E4 G4] → C4->C4(0), E4->E4(0), E4->G4(3), G4->C5(5) = 8
        // Or doubles G: [C4 E4 G4 G4] → C4->C4(0), E4->E4(0), G4->G4(0), G4->C5(5) = 5
        // Best should be doubling G (sv=5)
        assert_eq!(result.sv, 5);
    }

    #[test]
    fn vl_size_decrease_finds_minimum() {
        // 4→3 voices: expanding the smaller 'to'
        let from = chord(&[60, 64, 67, 72]);
        let to = chord(&[60, 64, 67]);
        let result = find_voice_leading(&from, &to);
        // Best expansion of 'to' to 4 is [C4 E4 G4 G4]: sv = |60-60|+|64-64|+|67-67|+|72-67| = 5
        assert_eq!(result.sv, 5);
    }

    #[test]
    fn vl_substitution_finds_close_inversion() {
        // From C4 E4 G4 to a G major chord — substitution should find nearest inversion
        let from = chord(&[60, 64, 67]);
        let to = chord(&[67, 71, 74]);
        let result = find_voice_leading_substitution(&from, &to);
        // All moves must be within 6 semitones
        for &v in &result.vec {
            assert!(v.abs() <= 6, "movement {v} exceeds limit");
        }
    }
}
