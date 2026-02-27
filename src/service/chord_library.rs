use once_cell::sync::Lazy;
use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::model::chord_statistics::{calculate_statistics, OrderedChordStatistics};
use crate::model::ordered_chord::OrderedChord;

/// Cache of computed `OrderedChordStatistics` keyed by `OrderedChord`.
///
/// Port of `ChordLibrary` singleton from `ChordNova/src/service/chordlibrary.cpp`.
/// Access via `chord_library_get()` rather than constructing directly.
static CHORD_LIBRARY: Lazy<Mutex<BTreeMap<OrderedChord, OrderedChordStatistics>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

/// Returns the `OrderedChordStatistics` for a given chord, computing and
/// caching them on first access.
///
/// Thread-safe via a global `Mutex`-protected `BTreeMap`.
pub fn chord_library_get(chord: &OrderedChord) -> OrderedChordStatistics {
    let mut cache = CHORD_LIBRARY.lock().unwrap();
    if let Some(stats) = cache.get(chord) {
        return stats.clone();
    }
    let stats = calculate_statistics(chord);
    cache.insert(chord.clone(), stats.clone());
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::pitch::Pitch;

    fn chord(notes: &[u8]) -> OrderedChord {
        OrderedChord::new(notes.iter().copied().map(Pitch::new).collect())
    }

    #[test]
    fn library_returns_correct_stats() {
        let c = chord(&[60, 64, 67]); // C major
        let stats = chord_library_get(&c);
        assert_eq!(stats.num_of_pitches, 3);
        assert_eq!(stats.num_of_unique_pitch_classes, 3);
    }

    #[test]
    fn library_caches_and_returns_same() {
        let c = chord(&[60, 64, 67]);
        let s1 = chord_library_get(&c);
        let s2 = chord_library_get(&c);
        assert_eq!(s1.num_of_pitches, s2.num_of_pitches);
        assert_eq!(s1.tension, s2.tension);
    }

    #[test]
    fn library_different_chords() {
        let c_major = chord(&[60, 64, 67]);
        let g_major = chord(&[67, 71, 74]);
        let sc = chord_library_get(&c_major);
        let sg = chord_library_get(&g_major);
        // Different chords should have different roots
        assert_ne!(sc.root, sg.root);
    }
}
