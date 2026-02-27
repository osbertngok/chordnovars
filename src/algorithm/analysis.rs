use crate::model::bigram_statistics::{calculate_bigram_statistics, BigramChordStatistics};
use crate::model::chord_statistics::{calculate_statistics, OrderedChordStatistics};
use crate::model::ordered_chord::OrderedChord;
use crate::service::voice_leading::{find_voice_leading, VoiceLeadingResult};

/// Result of analysing a two-chord progression (ante → post).
///
/// Port of `AnalysisResult` from `ChordNova/src/include/algorithm/analysis.h`.
pub struct AnalysisResult {
    pub ante_stats: OrderedChordStatistics,
    pub post_stats: OrderedChordStatistics,
    pub vl_result: VoiceLeadingResult,
    pub bigram_stats: BigramChordStatistics,
}

/// Analyse the relationship between two consecutive chords.
///
/// Port of `analyse()` from `ChordNova/src/algorithm/analysis.cpp`.
///
/// # Arguments
/// * `ante`              – The antecedent (previous) chord.
/// * `post`              – The consequent (next) chord.
/// * `prev_chroma_old`   – `chroma_old` from the chord before `ante` (0.0 if none).
/// * `prev_single_chroma`– `single_chroma` from the chord before `ante` (empty if none).
pub fn analyse(
    ante: &OrderedChord,
    post: &OrderedChord,
    prev_chroma_old: f64,
    prev_single_chroma: &[i32],
) -> AnalysisResult {
    let ante_stats = calculate_statistics(ante);
    let post_stats = calculate_statistics(post);

    let vl = find_voice_leading(ante, post);

    // vl_max is derived from the actual maximum absolute component
    let vl_max = vl.vec.iter().map(|v| v.abs()).max().unwrap_or(0).max(1);

    let bigram = calculate_bigram_statistics(
        ante, post, &ante_stats, &post_stats,
        vl.vec.clone(), vl.sv, vl_max,
        prev_chroma_old, prev_single_chroma,
    );

    AnalysisResult { ante_stats, post_stats, vl_result: vl, bigram_stats: bigram }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::pitch::Pitch;

    fn chord(notes: &[u8]) -> OrderedChord {
        OrderedChord::new(notes.iter().copied().map(Pitch::new).collect())
    }

    #[test]
    fn analyse_same_chord_sv_zero() {
        let c = chord(&[60, 64, 67]);
        let result = analyse(&c, &c, 0.0, &[]);
        assert_eq!(result.vl_result.sv, 0);
        assert_eq!(result.vl_result.vec, vec![0, 0, 0]);
    }

    #[test]
    fn analyse_reports_correct_stats() {
        let ante = chord(&[60, 64, 67]);
        let post = chord(&[65, 69, 72]); // F major
        let result = analyse(&ante, &post, 0.0, &[]);
        assert_eq!(result.ante_stats.num_of_pitches, 3);
        assert_eq!(result.post_stats.num_of_pitches, 3);
        assert!(result.vl_result.sv > 0);
    }

    #[test]
    fn analyse_vl_max_at_least_one() {
        // Same chord: max abs vec = 0, vl_max should be 1 (not 0)
        let c = chord(&[60, 64, 67]);
        let result = analyse(&c, &c, 0.0, &[]);
        // bigram stats should compute without division by zero
        let _ = result.bigram_stats.similarity;
    }
}
