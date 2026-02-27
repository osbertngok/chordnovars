use crate::model::ordered_chord::OrderedChord;
use crate::model::pitch::Pitch;
use crate::model::pitch_iterable::PitchIterable;
use crate::utility::combinatorics::{expansion_count, expansion_get};

/// Returns a single expansion of a chord using a specific combination index.
///
/// Port of `expand_single()` from `ChordNova/src/service/expansion.cpp`.
///
/// # Panics
/// Panics if `target_size < chord size` or `index` is out of range.
pub fn expand_single(chord: &OrderedChord, target_size: usize, index: i32) -> OrderedChord {
    let pitches = chord.get_pitches();
    let src_size = pitches.len();

    if target_size < src_size {
        panic!("expand_single: target_size ({target_size}) < chord size ({src_size})");
    }
    if target_size == src_size {
        return chord.clone();
    }

    let mapping = expansion_get(src_size as i32, target_size as i32, index)
        .expect("expand_single: index out of range");

    let result_pitches: Vec<Pitch> = mapping.iter()
        .map(|&idx| pitches[idx as usize])
        .collect();
    OrderedChord::new(result_pitches)
}

/// Expands an `OrderedChord` to a target voice count by duplicating existing notes.
///
/// Returns all `C(target_size - 1, src_size - 1)` possible expansions, each an
/// `OrderedChord` of length `target_size`.
///
/// Port of `expand()` from `ChordNova/src/service/expansion.cpp`.
///
/// # Panics
/// Panics if `target_size < chord size`.
pub fn expand(chord: &OrderedChord, target_size: usize) -> Vec<OrderedChord> {
    let src_size = chord.get_num_of_pitches();

    if target_size < src_size {
        panic!("expand: target_size ({target_size}) < chord size ({src_size})");
    }

    let count = expansion_count(src_size as i32, target_size as i32);
    (0..count)
        .map(|i| expand_single(chord, target_size, i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(notes: &[u8]) -> OrderedChord {
        OrderedChord::new(notes.iter().copied().map(Pitch::new).collect())
    }

    #[test]
    fn expand_single_identity() {
        // target == src: returns clone unchanged
        let c = chord(&[60, 64, 67]);
        let result = expand_single(&c, 3, 0);
        assert_eq!(result.get_pitches(), c.get_pitches());
    }

    #[test]
    fn expand_count() {
        // C major triad → 4 voices: C(3,2) = 3 expansions
        let c = chord(&[60, 64, 67]);
        let expansions = expand(&c, 4);
        assert_eq!(expansions.len(), 3);
        for exp in &expansions {
            assert_eq!(exp.get_num_of_pitches(), 4);
        }
    }

    #[test]
    fn expand_all_pitches_in_source() {
        // Every pitch in the expansion must come from the original chord's pitches
        let c = chord(&[60, 64, 67]);
        let src_pitches = c.get_pitches();
        for exp in expand(&c, 5) {
            for p in exp.get_pitches() {
                assert!(src_pitches.contains(&p),
                    "expanded pitch {p:?} not in source");
            }
        }
    }

    #[test]
    fn expand_single_to_two_voices() {
        // Single note C4 → 2 voices: only [C4, C4]
        let c = chord(&[60]);
        let expansions = expand(&c, 2);
        assert_eq!(expansions.len(), 1);
        assert_eq!(expansions[0].get_pitches(), vec![Pitch::new(60), Pitch::new(60)]);
    }

    #[test]
    fn expand_single_three_to_five() {
        // C(4,2) = 6 expansions
        let c = chord(&[60, 64, 67]);
        assert_eq!(expand(&c, 5).len(), 6);
    }
}
