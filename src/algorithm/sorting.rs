use crate::model::bigram_statistics::BigramChordStatistics;
use crate::model::ordered_chord::OrderedChord;
use crate::model::pitch_iterable::PitchIterable;

/// A candidate chord with its computed bigram statistics.
///
/// Port of `CandidateEntry` from `ChordNova/src/include/algorithm/sorting.h`.
#[derive(Debug, Clone)]
pub struct CandidateEntry {
    pub chord: OrderedChord,
    pub stats: BigramChordStatistics,
}

/// Sort candidate entries according to a sort_order string.
///
/// The sort_order is read right-to-left; each character selects a sort key,
/// and an optional '+' suffix means ascending (default = descending).
///
/// Key codes:
/// `P`=sim_orig, `N`=pitch-class count, `T`=tension, `K`=chroma, `C`=common_note,
/// `a`=span, `A`=sspan, `m`=note count, `h`=thickness, `g`=geometrical_center,
/// `S`=sv, `Q`=q_indicator, `X`=similarity, `k`=chroma_old, `R`/`V`=root_movement
///
/// Port of `sort_candidates()` from `ChordNova/src/algorithm/sorting.cpp`.
pub fn sort_candidates(candidates: &mut Vec<CandidateEntry>, sort_order: &str) {
    if sort_order.is_empty() || candidates.is_empty() { return; }

    let keys = parse_sort_keys(sort_order);
    if keys.is_empty() { return; }

    // Sort by index to avoid Clone constraints; stable_sort preserves relative order on ties
    let n = candidates.len();
    let mut indices: Vec<usize> = (0..n).collect();

    indices.sort_by(|&a, &b| {
        for (extractor, ascending) in &keys {
            let va = extractor(&candidates[a]);
            let vb = extractor(&candidates[b]);
            if (va - vb).abs() > 1e-15 {
                return if *ascending {
                    va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
                };
            }
        }
        std::cmp::Ordering::Equal
    });

    // Apply permutation
    let original: Vec<CandidateEntry> = candidates.drain(..).collect();
    *candidates = indices.into_iter().map(|i| original[i].clone()).collect();
}

type Extractor = Box<dyn Fn(&CandidateEntry) -> f64>;

fn get_extractor(ch: char) -> Option<Extractor> {
    match ch {
        'P' => Some(Box::new(|e: &CandidateEntry| e.stats.sim_orig as f64)),
        'N' => Some(Box::new(|e: &CandidateEntry| e.stats.pitch_class_set.len() as f64)),
        'T' => Some(Box::new(|e: &CandidateEntry| e.chord.get_tension())),
        'K' => Some(Box::new(|e: &CandidateEntry| e.stats.chroma)),
        'C' => Some(Box::new(|e: &CandidateEntry| e.stats.common_note as f64)),
        'a' => Some(Box::new(|e: &CandidateEntry| e.stats.span as f64)),
        'A' => Some(Box::new(|e: &CandidateEntry| e.stats.sspan as f64)),
        'm' => Some(Box::new(|e: &CandidateEntry| e.stats.notes.len() as f64)),
        'h' => Some(Box::new(|e: &CandidateEntry| e.chord.get_thickness())),
        'g' => Some(Box::new(|e: &CandidateEntry| e.chord.get_geometrical_center())),
        'S' => Some(Box::new(|e: &CandidateEntry| e.stats.sv as f64)),
        'Q' => Some(Box::new(|e: &CandidateEntry| e.stats.q_indicator)),
        'X' => Some(Box::new(|e: &CandidateEntry| e.stats.similarity as f64)),
        'k' => Some(Box::new(|e: &CandidateEntry| e.stats.chroma_old)),
        'R' | 'V' => Some(Box::new(|e: &CandidateEntry| e.stats.root_movement as f64)),
        _ => None,
    }
}

/// Parses sort_order right-to-left into (extractor, ascending) pairs,
/// ordered primary-first (leftmost = most significant = first in vec).
fn parse_sort_keys(sort_order: &str) -> Vec<(Extractor, bool)> {
    let chars: Vec<char> = sort_order.chars().collect();
    let mut keys: Vec<(Extractor, bool)> = Vec::new();
    let mut pos = chars.len() as i32 - 1;

    while pos >= 0 {
        let ch = chars[pos as usize];
        let mut ascending = false;

        if ch == '+' {
            ascending = true;
            pos -= 1;
            if pos < 0 { break; }
            let ch2 = chars[pos as usize];
            if let Some(extractor) = get_extractor(ch2) {
                keys.push((extractor, ascending));
            }
        } else if let Some(extractor) = get_extractor(ch) {
            keys.push((extractor, ascending));
        }
        pos -= 1;
    }

    keys.reverse();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::bigram_statistics::BigramChordStatistics;
    use crate::model::pitch::Pitch;

    fn make_entry(midi: &[u8], sv: i32, tension: f64) -> CandidateEntry {
        use crate::model::bigram_statistics::OverflowState;
        let chord = OrderedChord::new(midi.iter().copied().map(Pitch::new).collect());
        let mut stats = BigramChordStatistics::default();
        stats.sv = sv;
        CandidateEntry { chord, stats }
    }

    #[test]
    fn sort_empty_noop() {
        let mut candidates: Vec<CandidateEntry> = vec![];
        sort_candidates(&mut candidates, "S");
        assert!(candidates.is_empty());
    }

    #[test]
    fn sort_no_order_noop() {
        let mut candidates = vec![
            make_entry(&[60, 64, 67], 5, 1.0),
            make_entry(&[60, 64, 67], 2, 2.0),
        ];
        sort_candidates(&mut candidates, "");
        // No change in relative order
        assert_eq!(candidates[0].stats.sv, 5);
    }

    #[test]
    fn sort_by_sv_descending() {
        let mut candidates = vec![
            make_entry(&[60, 64, 67], 3, 1.0),
            make_entry(&[60, 64, 67], 7, 1.0),
            make_entry(&[60, 64, 67], 1, 1.0),
        ];
        sort_candidates(&mut candidates, "S"); // descending by sv
        assert_eq!(candidates[0].stats.sv, 7);
        assert_eq!(candidates[1].stats.sv, 3);
        assert_eq!(candidates[2].stats.sv, 1);
    }

    #[test]
    fn sort_by_sv_ascending() {
        let mut candidates = vec![
            make_entry(&[60, 64, 67], 3, 1.0),
            make_entry(&[60, 64, 67], 7, 1.0),
            make_entry(&[60, 64, 67], 1, 1.0),
        ];
        sort_candidates(&mut candidates, "S+"); // ascending by sv ('+' is suffix in sort_order)
        assert_eq!(candidates[0].stats.sv, 1);
        assert_eq!(candidates[1].stats.sv, 3);
        assert_eq!(candidates[2].stats.sv, 7);
    }
}
