use serde::{Serialize, Deserialize};

use crate::constant::ET_SIZE;
use crate::model::ordered_chord::OrderedChord;
use crate::model::pitch_class::PitchClass;
use crate::model::pitch_iterable::PitchIterable;
use crate::utility::general::{NOTE_POS, normal_form};

/// Calculated properties of a single chord.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderedChordStatistics {
    /// Number of pitches (n).
    pub num_of_pitches: usize,
    /// Number of unique pitch objects (m). Named "unique pitch classes" in C++ but counts unique Pitch objects.
    pub num_of_unique_pitch_classes: usize,
    /// Tension score (t).
    pub tension: f64,
    /// Thickness score (h).
    pub thickness: f64,
    /// Root pitch class (r).
    pub root: Option<PitchClass>,
    /// Geometrical center ratio 0–1 (g).
    pub geometrical_center: f64,
    /// Scale-degree position of each note relative to root, via NOTE_POS lookup.
    pub alignment: Vec<i32>,
    /// Consecutive intervals of the pitch-class normal form.
    pub self_diff: Vec<i32>,
    /// Interval-class frequency vector (length 6), ic1–ic6.
    pub count_vec: Vec<i32>,
}

pub fn calculate_statistics(chord: &OrderedChord) -> OrderedChordStatistics {
    let root = chord.find_root();
    let pitches = chord.get_pitches();

    // alignment: for each pitch, map (pitch - root) % 12 through NOTE_POS
    let alignment = if let Some(r) = root {
        pitches
            .iter()
            .map(|p| {
                let diff = (p.get_number() as i32 - r.value() as i32)
                    .rem_euclid(ET_SIZE as i32) as usize;
                NOTE_POS[diff]
            })
            .collect()
    } else {
        vec![]
    };

    // unique pitch classes (sorted) via bitset equivalent
    let mut pc_bits = [false; ET_SIZE];
    for p in &pitches {
        pc_bits[p.get_pitch_class().value() as usize] = true;
    }
    let pc_vec: Vec<i32> = (0..ET_SIZE)
        .filter(|&i| pc_bits[i])
        .map(|i| i as i32)
        .collect();

    // self_diff: consecutive differences of the normal form
    let normal = normal_form(&pc_vec);
    let self_diff: Vec<i32> = normal.windows(2).map(|w| w[1] - w[0]).collect();

    // count_vec: interval-class histogram (ic1..ic6) over all pc pairs
    let mut count_vec = vec![0i32; 6];
    for i in 0..pc_vec.len() {
        for j in (i + 1)..pc_vec.len() {
            let interval = (pc_vec[j] - pc_vec[i]) as usize;
            let ic = interval.min(ET_SIZE - interval);
            if ic >= 1 && ic <= 6 {
                count_vec[ic - 1] += 1;
            }
        }
    }

    OrderedChordStatistics {
        num_of_pitches: chord.get_num_of_pitches(),
        num_of_unique_pitch_classes: chord.get_num_of_unique_pitches(),
        tension: chord.get_tension(),
        thickness: chord.get_thickness(),
        root,
        geometrical_center: chord.get_geometrical_center(),
        alignment,
        self_diff,
        count_vec,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oc(s: &str) -> OrderedChord {
        OrderedChord::from_str_space_separated(s).unwrap()
    }

    #[test]
    fn statistics_alignment() {
        // C4=60, E4=64, G4=67; root=C(0)
        // diffs: 0, 4, 7 → NOTE_POS[0]=1, NOTE_POS[4]=3, NOTE_POS[7]=5
        let stats = calculate_statistics(&oc("C4 E4 G4"));
        assert_eq!(stats.alignment, vec![1, 3, 5]);
    }

    #[test]
    fn statistics_count_vec() {
        // C(0) E(4) G(7): pairs → ic4, ic5, ic3 → [0,0,1,1,1,0]
        let stats = calculate_statistics(&oc("C4 E4 G4"));
        assert_eq!(stats.count_vec, vec![0, 0, 1, 1, 1, 0]);
    }

    #[test]
    fn statistics_basic_fields() {
        let stats = calculate_statistics(&oc("C4 E4 G4"));
        assert_eq!(stats.num_of_pitches, 3);
        assert_eq!(stats.num_of_unique_pitch_classes, 3);
        assert_eq!(stats.root, Some(PitchClass::C));
    }

    #[test]
    fn statistics_self_diff() {
        // C(0) E(4) G(7) → pc_vec=[0,4,7] → normal_form → diffs
        let stats = calculate_statistics(&oc("C4 E4 G4"));
        // normal_form([0,4,7]): minimum span rotation starts at 0 → [0,4,7], diffs=[4,3]
        assert_eq!(stats.self_diff, vec![4, 3]);
    }
}
