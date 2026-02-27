use crate::model::pitch::Pitch;
use crate::model::pitch_class::{COFUnit, PitchClass, get_circle_of_fifth_distance};

/// A container of pitches that supports analysis operations.
pub trait PitchIterable {
    fn contains_pitch_class(&self, pc: PitchClass) -> bool;
    fn contains_pitch(&self, pitch: Pitch) -> bool;
    fn get_tension(&self) -> f64;
    fn get_thickness(&self) -> f64;
    fn get_geometrical_center(&self) -> f64;
    fn find_root(&self) -> Option<PitchClass>;
    fn get_pitches(&self) -> Vec<Pitch>;
    fn get_pitch_classes_ordered_by_circle_of_fifths(&self) -> Vec<PitchClass>;

    /// Minimal span on the Circle of Fifths.
    fn get_span(&self) -> COFUnit {
        let pcs = self.get_pitch_classes_ordered_by_circle_of_fifths();
        let n = pcs.len();
        if n == 0 {
            return COFUnit::new(0);
        }
        let mut max_gap = COFUnit::new(0);
        for i in 0..n {
            let gap = get_circle_of_fifth_distance(pcs[i], pcs[(i + 1) % n]);
            if gap > max_gap {
                max_gap = gap;
            }
        }
        COFUnit::new(12) - max_gap
    }
}
