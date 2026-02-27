use std::collections::BTreeSet;
use std::str::FromStr;

use crate::constant::ET_SIZE;
use crate::model::pitch::Pitch;
use crate::model::pitch_class::PitchClass;
use crate::model::pitch_iterable::PitchIterable;
use crate::utility::general::{RESTRICTION, ZXS_TENSION_WEIGHT_VECTOR};

/// A set of unique pitches, sorted in ascending order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PitchSet {
    pitches: BTreeSet<Pitch>,
}

impl PitchSet {
    pub fn new(pitches: impl IntoIterator<Item = Pitch>) -> Self {
        PitchSet {
            pitches: pitches.into_iter().collect(),
        }
    }

    pub fn from_str_space_separated(s: &str) -> Result<Self, String> {
        let pitches: Result<Vec<Pitch>, _> = s
            .split_whitespace()
            .map(|p| p.parse::<Pitch>().map_err(|e| e.to_string()))
            .collect();
        Ok(PitchSet::new(pitches?))
    }
}

impl PitchIterable for PitchSet {
    fn contains_pitch_class(&self, pc: PitchClass) -> bool {
        self.pitches.iter().any(|p| p.get_pitch_class() == pc)
    }

    fn contains_pitch(&self, pitch: Pitch) -> bool {
        self.pitches.contains(&pitch)
    }

    fn get_pitches(&self) -> Vec<Pitch> {
        self.pitches.iter().copied().collect()
    }

    fn get_pitch_classes_ordered_by_circle_of_fifths(&self) -> Vec<PitchClass> {
        let mut pcs: Vec<PitchClass> = self
            .pitches
            .iter()
            .map(|p| p.get_pitch_class())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        pcs.sort_by_key(|pc| pc.get_chroma());
        pcs
    }

    fn get_tension(&self) -> f64 {
        let pitch_vec = self.get_pitches();
        let n = pitch_vec.len();
        let mut tension = 0.0f64;
        for i in 0..n {
            for j in (i + 1)..n {
                let diff = (pitch_vec[j] - pitch_vec[i]) as usize;
                let interval = diff % ET_SIZE;
                let mut temp = ZXS_TENSION_WEIGHT_VECTOR[interval]
                    / (diff as f64 / ET_SIZE as f64 + 1.0);
                let restr = RESTRICTION[interval];
                let note_num = pitch_vec[j].get_number() as i32;
                if note_num < restr {
                    temp *= restr as f64 / note_num as f64;
                }
                tension += temp;
            }
        }
        tension / 10.0
    }

    fn get_thickness(&self) -> f64 {
        let pitch_vec = self.get_pitches();
        let n = pitch_vec.len();
        let mut thickness = 0.0f64;
        for i in 0..n {
            for j in (i + 1)..n {
                let diff = (pitch_vec[j] - pitch_vec[i]) as usize;
                if diff % ET_SIZE == 0 {
                    thickness += ET_SIZE as f64 / diff as f64;
                }
            }
        }
        thickness
    }

    fn get_geometrical_center(&self) -> f64 {
        let pitch_vec = self.get_pitches();
        if pitch_vec.is_empty() {
            return 0.5;
        }
        let min_p = pitch_vec.first().unwrap().get_number() as f64;
        let max_p = pitch_vec.last().unwrap().get_number() as f64;
        let sum: f64 = pitch_vec.iter().map(|p| p.get_number() as f64).sum();
        let n = pitch_vec.len() as f64;
        if (max_p - min_p).abs() < f64::EPSILON {
            0.5
        } else {
            (sum / n - min_p) / (max_p - min_p)
        }
    }

    fn find_root(&self) -> Option<PitchClass> {
        let pitch_vec = self.get_pitches();
        if pitch_vec.is_empty() {
            return None;
        }
        // interval_rank[interval] gives ranking; odd = lower note is root, even = upper note is root
        let interval_rank: [usize; ET_SIZE] = [11, 8, 6, 5, 3, 0, 10, 1, 2, 4, 7, 9];
        let n = pitch_vec.len();
        let mut root = *pitch_vec.last().unwrap();
        let mut best_rank = interval_rank[6]; // tritone as initial worst

        for i in 0..n {
            for j in (i + 1)..n {
                let diff = (pitch_vec[j] - pitch_vec[i]).unsigned_abs() as usize;
                let interval = diff % ET_SIZE;
                let rank = interval_rank[interval];
                if rank / 2 < best_rank / 2 {
                    root = if rank % 2 == 1 {
                        pitch_vec[i]
                    } else {
                        pitch_vec[j]
                    };
                    best_rank = rank;
                }
            }
        }
        Some(root.get_pitch_class())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ps(s: &str) -> PitchSet {
        PitchSet::from_str_space_separated(s).unwrap()
    }

    #[test]
    fn contains_pitch_class() {
        let set = ps("C4 E4 G4");
        assert!(set.contains_pitch_class(PitchClass::C));
        assert!(set.contains_pitch_class(PitchClass::E));
        assert!(!set.contains_pitch_class(PitchClass::D));
    }

    #[test]
    fn cof_ordering() {
        // C major: C, E, G → CoF order: F(-1) C(0) G(1) D(2) A(3) E(4)
        // so E(4) G(1) C(0) sorted: C(0) G(1) E(4)
        let set = ps("C4 E4 G4");
        let pcs = set.get_pitch_classes_ordered_by_circle_of_fifths();
        assert_eq!(pcs[0], PitchClass::C);
        assert_eq!(pcs[1], PitchClass::G);
        assert_eq!(pcs[2], PitchClass::E);
    }

    #[test]
    fn root_finding_major_triad() {
        let set = ps("C4 E4 G4");
        // C major → root should be C
        assert_eq!(set.find_root(), Some(PitchClass::C));
    }

    #[test]
    fn span() {
        // C major triad: C G E on CoF → span from C to E is 4
        use crate::model::pitch_iterable::PitchIterable;
        let set = ps("C4 E4 G4");
        let span = set.get_span();
        assert_eq!(span, crate::model::pitch_class::COFUnit::new(4));
    }
}
