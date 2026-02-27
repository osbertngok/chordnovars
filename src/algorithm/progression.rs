use std::collections::HashSet;

use crate::algorithm::sorting::{sort_candidates, CandidateEntry};
use crate::algorithm::validation::{ChordValidationPipeline, ValidationContext};
use crate::model::bigram_statistics::calculate_bigram_statistics;
use crate::model::chord_statistics::calculate_statistics;
use crate::model::config::{ProgressionConfig, UniqueMode};
use crate::model::ordered_chord::OrderedChord;
use crate::model::pitch::Pitch;
use crate::service::expansion::expand_single;
use crate::service::voice_leading::VoiceLeadingResult;
use crate::utility::combinatorics::expansion_count;
use crate::utility::mixed_radix::MixedRadixRange;

/// Result of a single-step progression generation.
///
/// Port of `ProgressionResult` from `ChordNova/src/include/algorithm/progression.h`.
pub struct ProgressionResult {
    /// All valid candidate chords with bigram statistics.
    pub candidates: Vec<CandidateEntry>,
    /// Total number of mutation vectors evaluated.
    pub total_evaluated: i32,
}

/// Optional progress callback: `(current_count, total_count)`.
pub type ProgressCallback = Box<dyn Fn(i64, i64)>;

/// Generates all valid next chords from an initial chord.
///
/// Algorithm:
/// 1. Expand the initial chord to `m_max` voices (all `C(m_max-1, m-1)` expansions)
/// 2. For each expansion, iterate all mutation vectors in `[-vl_max, vl_max]`
/// 3. Validate each candidate through the 15-stage pipeline
/// 4. Collect all passing candidates with their bigram statistics
///
/// Port of `generate_single()` from `ChordNova/src/algorithm/progression.cpp`.
pub fn generate_single(
    initial_chord: &OrderedChord,
    config: &ProgressionConfig,
    prev_single_chroma: &[i32],
    prev_chroma_old: f64,
    record: &[OrderedChord],
    progress: Option<&ProgressCallback>,
) -> ProgressionResult {
    let initial_stats = calculate_statistics(initial_chord);
    let m_notes_size = initial_chord.get_num_of_pitches();
    let m_max = config.range.m_max as usize;
    let vl_max = config.voice_leading.vl_max;
    let vl_min = config.voice_leading.vl_min;

    let mut rec_ids: HashSet<i32> = HashSet::new();
    let mut vec_ids: HashSet<i64> = HashSet::new();
    let mut dup_ids: HashSet<Vec<u8>> = HashSet::new();

    let num_expansions = expansion_count(m_notes_size as i32, m_max as i32);
    let mutation_range = MixedRadixRange::new(vl_max, m_max, vl_min);
    let mutations_per_expansion = mutation_range.total_count();
    let total_iterations = num_expansions as i64 * mutations_per_expansion;
    let mut iteration_count: i64 = 0;

    let pipeline = ChordValidationPipeline::new();
    let mut candidates: Vec<CandidateEntry> = Vec::new();
    let mut total_evaluated: i32 = 0;

    for exp_idx in 0..num_expansions {
        let expansion = expand_single(initial_chord, m_max, exp_idx);
        let exp_pitches = expansion.get_pitches();

        for mutation_vec in mutation_range.iter() {
            iteration_count += 1;
            total_evaluated += 1;

            // Apply mutation to the expanded chord
            let mut out_of_range = false;
            let mut new_pitches: Vec<Pitch> = Vec::with_capacity(m_max);
            for i in 0..m_max {
                let new_midi = exp_pitches[i].get_number() as i32 + mutation_vec[i];
                if new_midi < 0 || new_midi > 127 {
                    out_of_range = true;
                    break;
                }
                new_pitches.push(Pitch::new(new_midi as u8));
            }
            if out_of_range {
                if let Some(cb) = progress {
                    if iteration_count % 10000 == 0 { cb(iteration_count, total_iterations); }
                }
                continue;
            }

            let candidate = OrderedChord::new(new_pitches);

            let mut ctx = ValidationContext {
                config,
                prev_chord: initial_chord,
                prev_stats: &initial_stats,
                vl_result: VoiceLeadingResult::default(),
                candidate_stats: None,
                rec_ids: &mut rec_ids,
                vec_ids: &mut vec_ids,
                prev_single_chroma,
                prev_chroma_old,
                record,
            };

            if pipeline.validate(&mut ctx, &candidate) {
                // RemoveDup: deduplicate exact MIDI-note vectors across expansion paths
                if config.uniqueness.unique_mode == UniqueMode::RemoveDup {
                    let midi_vec: Vec<u8> = candidate.get_pitches()
                        .iter().map(|p| p.get_number()).collect();
                    if !dup_ids.insert(midi_vec) {
                        if let Some(cb) = progress {
                            if iteration_count % 10000 == 0 { cb(iteration_count, total_iterations); }
                        }
                        continue;
                    }
                }

                let cand_stats = ctx.candidate_stats
                    .unwrap_or_else(|| calculate_statistics(&candidate));

                let bigram = calculate_bigram_statistics(
                    initial_chord,
                    &candidate,
                    &initial_stats,
                    &cand_stats,
                    ctx.vl_result.vec.clone(),
                    ctx.vl_result.sv,
                    vl_max,
                    prev_chroma_old,
                    prev_single_chroma,
                );

                candidates.push(CandidateEntry { chord: candidate, stats: bigram });
            }

            if let Some(cb) = progress {
                if iteration_count % 10000 == 0 { cb(iteration_count, total_iterations); }
            }
        }
    }

    if !config.sort.sort_order.is_empty() {
        sort_candidates(&mut candidates, &config.sort.sort_order);
    }

    ProgressionResult { candidates, total_evaluated }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::pitch::Pitch;

    fn chord(notes: &[u8]) -> OrderedChord {
        OrderedChord::new(notes.iter().copied().map(Pitch::new).collect())
    }

    #[test]
    fn generate_single_produces_candidates() {
        use crate::model::config::{RangeConstraints, VoiceLeadingConstraints};
        // Use tight constraints to keep the search space small for testing
        let initial = chord(&[60, 64, 67]);
        let mut config = ProgressionConfig::default();
        config.range = RangeConstraints {
            lowest: 55, highest: 75, m_min: 3, m_max: 3, n_min: 1, n_max: 12,
            ..RangeConstraints::default()
        };
        config.voice_leading = VoiceLeadingConstraints {
            vl_max: 2, vl_min: 0, ..VoiceLeadingConstraints::default()
        };
        let result = generate_single(&initial, &config, &[], 0.0, &[], None);
        assert!(!result.candidates.is_empty(),
            "Expected some candidates, got 0 (total_evaluated={})", result.total_evaluated);
        assert!(result.total_evaluated > 0);
    }

    #[test]
    fn generate_single_respects_range() {
        use crate::model::config::{RangeConstraints, VoiceLeadingConstraints};
        let initial = chord(&[60, 64, 67]);
        let mut config = ProgressionConfig::default();
        config.range = RangeConstraints {
            lowest: 58, highest: 70, m_min: 3, m_max: 3,
            n_min: 1, n_max: 12, ..RangeConstraints::default()
        };
        config.voice_leading = VoiceLeadingConstraints {
            vl_max: 2, vl_min: 0, ..VoiceLeadingConstraints::default()
        };
        let result = generate_single(&initial, &config, &[], 0.0, &[], None);
        for entry in &result.candidates {
            let pitches = entry.chord.get_pitches();
            for p in pitches {
                assert!(p.get_number() >= 58 && p.get_number() <= 70,
                    "Pitch {} out of range [58, 70]", p.get_number());
            }
        }
    }

    // ── Golden test ──────────────────────────────────────────────────────────

    /// Build the golden config mirroring legacy preset_1 from the C++ golden test.
    ///
    /// Note: k_min/k_max/t_min/t_max are percentile-based post-processing in the
    /// legacy code and are not applied during generation, so left at defaults.
    fn make_golden_config() -> ProgressionConfig {
        use crate::model::config::{
            AlignMode, AlignmentConfig, HarmonicConstraints, RangeConstraints,
            UniqueMode, UniquenessConfig, VLSetting, VoiceLeadingConstraints,
        };

        let mut config = ProgressionConfig::default();

        config.range = RangeConstraints {
            lowest: 0,
            highest: 127,
            m_min: 1,
            m_max: 4,
            n_min: 1,
            n_max: 12,
            h_min: 0.0,
            h_max: 50.0,
            r_min: 0,
            r_max: 11,
            g_min: 0,
            g_max: 70,
        };

        config.voice_leading = VoiceLeadingConstraints {
            vl_min: 0,
            vl_max: 4,
            vl_setting: VLSetting::Default,
            ..VoiceLeadingConstraints::default()
        };

        config.alignment = AlignmentConfig {
            align_mode: AlignMode::Unlimited,
            ..AlignmentConfig::default()
        };

        config.uniqueness = UniquenessConfig {
            unique_mode: UniqueMode::RemoveDup,
        };

        config.harmonic = HarmonicConstraints {
            c_min: 0,
            c_max: 2,
            s_min: 0,
            s_max: 12,
            ss_min: 0,
            ss_max: 12,
            sv_min: 4,
            sv_max: 12,
            q_min: -500.0,
            q_max: 500.0,
            x_min: 0,
            x_max: 100,
            ..HarmonicConstraints::default()
        };

        config.root_movement.enabled = false;
        config.exclusion.enabled = false;
        config.similarity.enabled = false;

        // Chord library: Major {0,4,7} + Minor {0,3,7}, all 12 transpositions
        let mut library: Vec<i32> = Vec::new();
        for j in 0..12i32 {
            // Major triad
            let mut val = 0i32;
            for &n in &[0i32, 4, 7] { val += 1 << ((n + j).rem_euclid(12)); }
            library.push(val);
            // Minor triad
            let mut val = 0i32;
            for &n in &[0i32, 3, 7] { val += 1 << ((n + j).rem_euclid(12)); }
            library.push(val);
        }
        library.sort_unstable();
        library.dedup();
        config.chord_library.chord_library = library;

        config
    }

    fn midi_notes(entry: &CandidateEntry) -> Vec<u8> {
        entry.chord.get_pitches().iter().map(|p| p.get_number()).collect()
    }

    #[test]
    fn golden_total_count() {
        let initial = chord(&[60, 64, 67]); // C4 E4 G4
        let config = make_golden_config();
        let result = generate_single(&initial, &config, &[], 0.0, &[], None);

        assert_eq!(
            result.candidates.len(), 92,
            "Expected 92 candidates, got {}. \
             First few: {:?}",
            result.candidates.len(),
            result.candidates.iter().take(5).map(midi_notes).collect::<Vec<_>>()
        );
    }

    #[test]
    fn golden_first_chord() {
        let initial = chord(&[60, 64, 67]);
        let config = make_golden_config();
        let result = generate_single(&initial, &config, &[], 0.0, &[], None);

        assert!(!result.candidates.is_empty(), "No candidates generated");
        assert_eq!(
            midi_notes(&result.candidates[0]),
            vec![56, 60, 60, 63],
            "First candidate mismatch"
        );
    }

    #[test]
    fn golden_contains_legacy_chords() {
        let initial = chord(&[60, 64, 67]);
        let config = make_golden_config();
        let result = generate_single(&initial, &config, &[], 0.0, &[], None);

        let legacy: &[&[u8]] = &[
            &[56, 59, 64, 68],
            &[59, 62, 67, 71],
            &[57, 60, 65, 69],
            &[58, 62, 67, 70],
            &[59, 64, 68, 71],
        ];
        for &expected in legacy {
            let found = result.candidates.iter().any(|e| midi_notes(e) == expected);
            assert!(found, "Missing legacy chord: {:?}", expected);
        }
    }

    #[test]
    fn golden_all_stats_within_bounds() {
        let initial = chord(&[60, 64, 67]);
        let config = make_golden_config();
        let result = generate_single(&initial, &config, &[], 0.0, &[], None);

        for entry in &result.candidates {
            let s = &entry.stats;
            assert!(s.sv >= config.harmonic.sv_min && s.sv <= config.harmonic.sv_max,
                "sv={} out of [{}, {}]", s.sv, config.harmonic.sv_min, config.harmonic.sv_max);
            assert!(s.similarity >= config.harmonic.x_min && s.similarity <= config.harmonic.x_max,
                "similarity={} out of [{}, {}]",
                s.similarity, config.harmonic.x_min, config.harmonic.x_max);
            assert!(s.common_note >= config.harmonic.c_min && s.common_note <= config.harmonic.c_max,
                "common_note={} out of [{}, {}]",
                s.common_note, config.harmonic.c_min, config.harmonic.c_max);
        }
    }
}

use crate::model::pitch_iterable::PitchIterable;
