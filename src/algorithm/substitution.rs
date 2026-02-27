use crate::algorithm::sorting::{sort_candidates, CandidateEntry};
use crate::constant::ET_SIZE;
use crate::model::bigram_statistics::{calculate_bigram_statistics, BigramChordStatistics};
use crate::model::chord_statistics::calculate_statistics;
use crate::model::config::SubstituteObj;
use crate::model::ordered_chord::OrderedChord;
use crate::model::pitch::Pitch;
use crate::model::pitch_iterable::PitchIterable;
use crate::model::substitution_config::{ParamTolerance, SubstitutionConfig};
use crate::service::voice_leading::find_voice_leading_substitution;

/// A single substitution result entry.
///
/// Port of `SubstitutionEntry` from `ChordNova/src/include/algorithm/substitution.h`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SubstitutionEntry {
    pub chord: OrderedChord,
    pub stats: BigramChordStatistics,
    /// Similarity to the original chord (0–100).
    pub sim_orig: i32,
}

/// Paired substitution entry for BothChords mode.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SubstitutionPair {
    pub ante: SubstitutionEntry,
    pub post: SubstitutionEntry,
}

/// Result of a substitution search.
#[derive(serde::Serialize)]
pub struct SubstitutionResult {
    /// Postchord/Antechord mode: single-chord substitutes.
    pub entries: Vec<SubstitutionEntry>,
    /// BothChords mode: paired substitutes.
    pub pairs: Vec<SubstitutionPair>,
    pub total_evaluated: i32,
}

/// Optional progress callback: `(current, total)`.
pub type SubstitutionProgressCallback = Box<dyn Fn(i64, i64)>;

// ── Utility functions ────────────────────────────────────────────────────────

/// Convert a 12-bit pitch-class bitmask to an `OrderedChord` at octave 6 (MIDI 72+).
///
/// Port of `id_to_chord()` from `ChordNova/src/algorithm/substitution.cpp`.
pub fn id_to_chord(id: i32) -> OrderedChord {
    let mut pitches: Vec<Pitch> = Vec::new();
    let mut note = 72i32;
    let mut copy = id;
    while copy != 0 {
        if copy % 2 == 1 {
            pitches.push(Pitch::new(note as u8));
        }
        note += 1;
        copy /= 2;
    }
    OrderedChord::new(pitches)
}

/// Reduce a chord to pitch classes mapped to MIDI 72–83 (octave 6).
///
/// Port of `reduce_to_octave6()` from `ChordNova/src/algorithm/substitution.cpp`.
pub fn reduce_to_octave6(chord: &OrderedChord) -> OrderedChord {
    let mut pc_bits = [false; ET_SIZE];
    for p in chord.get_pitches() {
        pc_bits[p.get_number() as usize % ET_SIZE] = true;
    }
    let pitches: Vec<Pitch> = (0..ET_SIZE)
        .filter(|&i| pc_bits[i])
        .map(|i| Pitch::new((72 + i) as u8))
        .collect();
    OrderedChord::new(pitches)
}

/// Compute substitution similarity between an original and a candidate chord.
///
/// Port of `compute_substitution_similarity()` from
/// `ChordNova/src/algorithm/substitution.cpp`.
pub fn compute_substitution_similarity(sv: i32, same_root: bool) -> i32 {
    let mut temp = 1.0 - sv as f64 / 36.0;
    if temp < 0.0 { temp = 0.0; }
    if same_root { temp = temp.sqrt(); }
    (100.0 * temp).round() as i32
}

/// Compute tolerance min/max from center and radius.
///
/// Port of `compute_tolerance_range()` from `ChordNova/src/algorithm/substitution.cpp`.
pub fn compute_tolerance_range(tol: &mut ParamTolerance) {
    if tol.use_percentage {
        tol.min_sub = tol.center * (1.0 - tol.radius / 100.0);
        tol.max_sub = tol.center * (1.0 + tol.radius / 100.0);
    } else {
        tol.min_sub = tol.center - tol.radius;
        tol.max_sub = tol.center + tol.radius;
    }
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn contains_char(s: &str, c: char) -> bool {
    s.contains(c)
}

fn get_tolerance_mut<'a>(config: &'a mut SubstitutionConfig, var: char) -> &'a mut ParamTolerance {
    match var {
        'P' => &mut config.sim_orig,
        'N' => &mut config.cardinality,
        'T' => &mut config.tension,
        'K' => &mut config.chroma,
        'C' => &mut config.common_note,
        'a' => &mut config.span,
        'A' => &mut config.sspan,
        'S' => &mut config.sv,
        'Q' => &mut config.q_indicator,
        'X' => &mut config.similarity,
        'k' => &mut config.chroma_old,
        'R' => &mut config.root,
        _   => &mut config.sim_orig,
    }
}

fn get_tolerance<'a>(config: &'a SubstitutionConfig, var: char) -> &'a ParamTolerance {
    match var {
        'P' => &config.sim_orig,
        'N' => &config.cardinality,
        'T' => &config.tension,
        'K' => &config.chroma,
        'C' => &config.common_note,
        'a' => &config.span,
        'A' => &config.sspan,
        'S' => &config.sv,
        'Q' => &config.q_indicator,
        'X' => &config.similarity,
        'k' => &config.chroma_old,
        'R' => &config.root,
        _   => &config.sim_orig,
    }
}

fn in_range(tol: &ParamTolerance, value: f64) -> bool {
    value >= tol.min_sub && value <= tol.max_sub
}

fn compute_param_centers(
    config: &mut SubstitutionConfig,
    ante: &OrderedChord,
    post: &OrderedChord,
) {
    let ante_stats = calculate_statistics(ante);
    let post_stats = calculate_statistics(post);

    let vl = find_voice_leading_substitution(ante, post);
    let bigram = calculate_bigram_statistics(
        ante, post, &ante_stats, &post_stats,
        vl.vec.clone(), vl.sv, 6, 0.0, &[],
    );

    let sim_orig_val = compute_substitution_similarity(
        vl.sv,
        ante_stats.root == post_stats.root,
    );

    let centers: Vec<(char, f64)> = vec![
        ('P', sim_orig_val as f64),
        ('N', post_stats.num_of_unique_pitch_classes as f64),
        ('T', post_stats.tension),
        ('K', bigram.chroma),
        ('C', bigram.common_note as f64),
        ('a', bigram.span as f64),
        ('A', bigram.sspan as f64),
        ('S', bigram.sv as f64),
        ('Q', bigram.q_indicator),
        ('X', bigram.similarity as f64),
        ('k', bigram.chroma_old - bigram.prev_chroma_old),
        ('R', post_stats.root.map(|r| r.value() as f64).unwrap_or(0.0)),
    ];

    let reset_list = config.reset_list.clone();
    let percentage_list = config.percentage_list.clone();
    for (var, computed) in centers {
        let tol = get_tolerance_mut(config, var);
        if !contains_char(&reset_list, var) {
            tol.center = computed;
        }
        tol.use_percentage = contains_char(&percentage_list, var);
        compute_tolerance_range(tol);
    }
}

fn valid_sub(
    config: &SubstitutionConfig,
    reference: &OrderedChord,
    candidate: &OrderedChord,
    sim_orig_val: i32,
    cand_n: usize,
    cand_tension: f64,
    cand_root: Option<crate::model::pitch_class::PitchClass>,
    object: &SubstituteObj,
) -> bool {
    let sort_order = &config.sort_order;

    if contains_char(sort_order, 'P')
        && !in_range(get_tolerance(config, 'P'), sim_orig_val as f64)
    { return false; }
    if contains_char(sort_order, 'N')
        && !in_range(get_tolerance(config, 'N'), cand_n as f64)
    { return false; }
    if contains_char(sort_order, 'T')
        && !in_range(get_tolerance(config, 'T'), cand_tension)
    { return false; }
    if contains_char(sort_order, 'R') {
        if let Some(r) = cand_root {
            if !in_range(get_tolerance(config, 'R'), r.value() as f64) { return false; }
        }
    }

    let ref_stats = calculate_statistics(reference);
    let vl = find_voice_leading_substitution(reference, candidate);
    let cand_stats = calculate_statistics(candidate);
    let bigram = calculate_bigram_statistics(
        reference, candidate, &ref_stats, &cand_stats,
        vl.vec.clone(), vl.sv, 6, 0.0, &[],
    );

    let chroma_val = if *object == SubstituteObj::Antechord {
        -bigram.chroma
    } else {
        bigram.chroma
    };
    let q_val = if *object == SubstituteObj::Antechord {
        -bigram.q_indicator
    } else {
        bigram.q_indicator
    };
    let chroma_old_diff = bigram.chroma_old - bigram.prev_chroma_old;

    if contains_char(sort_order, 'K') && !in_range(get_tolerance(config, 'K'), chroma_val) { return false; }
    if contains_char(sort_order, 'C') && !in_range(get_tolerance(config, 'C'), bigram.common_note as f64) { return false; }
    if contains_char(sort_order, 'a') && !in_range(get_tolerance(config, 'a'), bigram.span as f64) { return false; }
    if contains_char(sort_order, 'A') && !in_range(get_tolerance(config, 'A'), bigram.sspan as f64) { return false; }
    if contains_char(sort_order, 'S') && !in_range(get_tolerance(config, 'S'), bigram.sv as f64) { return false; }
    if contains_char(sort_order, 'Q') && !in_range(get_tolerance(config, 'Q'), q_val) { return false; }
    if contains_char(sort_order, 'X') && !in_range(get_tolerance(config, 'X'), bigram.similarity as f64) { return false; }
    if contains_char(sort_order, 'k') && !in_range(get_tolerance(config, 'k'), chroma_old_diff) { return false; }

    if contains_char(sort_order, 'V') && !config.rm_priority.is_empty() {
        let rm = bigram.root_movement;
        if rm >= 0 && (rm as usize) < config.rm_priority.len()
            && config.rm_priority[rm as usize] == -1
        { return false; }
    }

    true
}

fn sort_substitution_entries(entries: &mut Vec<SubstitutionEntry>, sort_order: &str) {
    let mut candidates: Vec<CandidateEntry> = entries.iter()
        .map(|e| CandidateEntry { chord: e.chord.clone(), stats: e.stats.clone() })
        .collect();
    sort_candidates(&mut candidates, sort_order);
    *entries = candidates.into_iter()
        .map(|c| SubstitutionEntry { chord: c.chord, sim_orig: c.stats.sim_orig, stats: c.stats })
        .collect();
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Perform a chord substitution search.
///
/// Port of `substitute()` from `ChordNova/src/algorithm/substitution.cpp`.
pub fn substitute(
    ante: &OrderedChord,
    post: &OrderedChord,
    config: &mut SubstitutionConfig,
    progress: Option<&SubstitutionProgressCallback>,
) -> SubstitutionResult {
    let mut result = SubstitutionResult {
        entries: Vec::new(),
        pairs: Vec::new(),
        total_evaluated: 0,
    };

    let reduced_ante = reduce_to_octave6(ante);
    let reduced_post = reduce_to_octave6(post);

    compute_param_centers(config, &reduced_ante, &reduced_post);

    let total_ids = (1 << ET_SIZE) - 1; // 4095

    match config.object {
        SubstituteObj::Postchord => {
            let ante_stats = calculate_statistics(&reduced_ante);
            let reduced_post_pitches = reduced_post.get_pitches();

            for id in 1..=total_ids {
                let candidate = id_to_chord(id);
                let cand_pitches = candidate.get_pitches();

                // Skip the original post chord
                if cand_pitches == reduced_post_pitches { continue; }

                let cand_stats = calculate_statistics(&candidate);
                let vl = find_voice_leading_substitution(&reduced_post, &candidate);
                let same_root = cand_stats.root.is_some()
                    && ante_stats.root.is_some()
                    && cand_stats.root == ante_stats.root;
                let sim_orig_val = compute_substitution_similarity(vl.sv, same_root);

                if valid_sub(config, &reduced_ante, &candidate, sim_orig_val,
                             cand_stats.num_of_unique_pitch_classes, cand_stats.tension,
                             cand_stats.root, &SubstituteObj::Postchord)
                {
                    let ref_stats = calculate_statistics(&reduced_ante);
                    let vl2 = find_voice_leading_substitution(&reduced_ante, &candidate);
                    let cand_stats2 = calculate_statistics(&candidate);
                    let bigram = calculate_bigram_statistics(
                        &reduced_ante, &candidate, &ref_stats, &cand_stats2,
                        vl2.vec.clone(), vl2.sv, 6, 0.0, &[],
                    );
                    result.entries.push(SubstitutionEntry { chord: candidate, stats: bigram, sim_orig: sim_orig_val });
                }

                result.total_evaluated += 1;
                if let Some(cb) = progress {
                    if id % 100 == 0 { cb(id as i64, total_ids as i64); }
                }
            }

            sort_substitution_entries(&mut result.entries, &config.sort_order);
        }

        SubstituteObj::Antechord => {
            let post_stats = calculate_statistics(&reduced_post);
            let reduced_ante_pitches = reduced_ante.get_pitches();

            for id in 1..=total_ids {
                let candidate = id_to_chord(id);
                let cand_pitches = candidate.get_pitches();

                if cand_pitches == reduced_ante_pitches { continue; }

                let cand_stats = calculate_statistics(&candidate);
                let vl = find_voice_leading_substitution(&reduced_ante, &candidate);
                let same_root = cand_stats.root.is_some()
                    && post_stats.root.is_some()
                    && cand_stats.root == post_stats.root;
                let sim_orig_val = compute_substitution_similarity(vl.sv, same_root);

                if valid_sub(config, &reduced_post, &candidate, sim_orig_val,
                             cand_stats.num_of_unique_pitch_classes, cand_stats.tension,
                             cand_stats.root, &SubstituteObj::Antechord)
                {
                    let ref_stats = calculate_statistics(&reduced_post);
                    let vl2 = find_voice_leading_substitution(&candidate, &reduced_post);
                    let cand_stats2 = calculate_statistics(&candidate);
                    let post_stats2 = calculate_statistics(&reduced_post);
                    let bigram = calculate_bigram_statistics(
                        &candidate, &reduced_post, &cand_stats2, &post_stats2,
                        vl2.vec.clone(), vl2.sv, 6, 0.0, &[],
                    );
                    result.entries.push(SubstitutionEntry { chord: candidate, stats: bigram, sim_orig: sim_orig_val });
                }

                result.total_evaluated += 1;
                if let Some(cb) = progress {
                    if id % 100 == 0 { cb(id as i64, total_ids as i64); }
                }
            }

            sort_substitution_entries(&mut result.entries, &config.sort_order);
        }

        SubstituteObj::BothChords => {
            let total = if config.test_all {
                total_ids as i64 * total_ids as i64
            } else {
                config.sample_size as i64
            };

            // Seeded RNG for reproducibility
            let mut rng_state: u64 = 42;
            let mut next_rng = || -> i32 {
                rng_state ^= rng_state << 13;
                rng_state ^= rng_state >> 7;
                rng_state ^= rng_state << 17;
                ((rng_state % total_ids as u64) + 1) as i32
            };

            let reduced_ante_pitches = reduced_ante.get_pitches();
            let reduced_post_pitches = reduced_post.get_pitches();
            let reduced_ante_stats = calculate_statistics(&reduced_ante);

            for i in 0..total {
                let (ante_id, post_id) = if config.test_all {
                    ((i / total_ids as i64) as i32 + 1, (i % total_ids as i64) as i32 + 1)
                } else {
                    (next_rng(), next_rng())
                };

                let ante_cand = id_to_chord(ante_id);
                let post_cand = id_to_chord(post_id);

                // Skip the original pair
                if ante_cand.get_pitches() == reduced_ante_pitches
                    && post_cand.get_pitches() == reduced_post_pitches
                { continue; }

                let ante_cand_stats = calculate_statistics(&ante_cand);
                let post_cand_stats = calculate_statistics(&post_cand);
                let reduced_post_stats = calculate_statistics(&reduced_post);

                let vl_post = find_voice_leading_substitution(&reduced_post, &post_cand);
                let same_root_post = post_cand_stats.root.is_some()
                    && reduced_post_stats.root.is_some()
                    && post_cand_stats.root == reduced_post_stats.root;
                let sim_orig_post = compute_substitution_similarity(vl_post.sv, same_root_post);

                let vl_ante = find_voice_leading_substitution(&reduced_ante, &ante_cand);
                let same_root_ante = ante_cand_stats.root.is_some()
                    && reduced_ante_stats.root.is_some()
                    && ante_cand_stats.root == reduced_ante_stats.root;
                let sim_orig_ante = compute_substitution_similarity(vl_ante.sv, same_root_ante);

                if valid_sub(config, &ante_cand, &post_cand, sim_orig_post,
                             post_cand_stats.num_of_unique_pitch_classes, post_cand_stats.tension,
                             post_cand_stats.root, &SubstituteObj::Postchord)
                {
                    let vl2 = find_voice_leading_substitution(&ante_cand, &post_cand);
                    let bigram_post = calculate_bigram_statistics(
                        &ante_cand, &post_cand, &ante_cand_stats, &post_cand_stats,
                        vl2.vec.clone(), vl2.sv, 6, 0.0, &[],
                    );

                    let vl3 = find_voice_leading_substitution(&reduced_ante, &ante_cand);
                    let bigram_ante = calculate_bigram_statistics(
                        &reduced_ante, &ante_cand, &reduced_ante_stats, &ante_cand_stats,
                        vl3.vec.clone(), vl3.sv, 6, 0.0, &[],
                    );

                    result.pairs.push(SubstitutionPair {
                        ante: SubstitutionEntry { chord: ante_cand, stats: bigram_ante, sim_orig: sim_orig_ante },
                        post: SubstitutionEntry { chord: post_cand, stats: bigram_post, sim_orig: sim_orig_post },
                    });
                }

                result.total_evaluated += 1;
                if let Some(cb) = progress {
                    if i % 1000 == 0 { cb(i, total); }
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::pitch::Pitch;

    fn chord(notes: &[u8]) -> OrderedChord {
        OrderedChord::new(notes.iter().copied().map(Pitch::new).collect())
    }

    #[test]
    fn id_to_chord_bit0() {
        // id=1 (bit 0 set) → MIDI 72 (C5)
        let c = id_to_chord(1);
        assert_eq!(c.get_pitches().len(), 1);
        assert_eq!(c.get_pitches()[0].get_number(), 72);
    }

    #[test]
    fn id_to_chord_all_bits() {
        // id=0xFFF → all 12 pitch classes
        let c = id_to_chord(0xFFF);
        assert_eq!(c.get_pitches().len(), 12);
    }

    #[test]
    fn reduce_to_octave6_maps_correctly() {
        let c = chord(&[60, 64, 67]); // C4 E4 G4
        let reduced = reduce_to_octave6(&c);
        let pitches = reduced.get_pitches();
        assert_eq!(pitches.len(), 3);
        // C→72, E→76, G→79
        assert_eq!(pitches[0].get_number(), 72);
        assert_eq!(pitches[1].get_number(), 76);
        assert_eq!(pitches[2].get_number(), 79);
    }

    #[test]
    fn compute_substitution_similarity_same_pitch() {
        // sv=0, same root → 100
        assert_eq!(compute_substitution_similarity(0, false), 100);
    }

    #[test]
    fn compute_tolerance_range_absolute() {
        let mut tol = ParamTolerance {
            center: 10.0, radius: 3.0, use_percentage: false,
            min_sub: 0.0, max_sub: 0.0,
        };
        compute_tolerance_range(&mut tol);
        assert!((tol.min_sub - 7.0).abs() < 1e-10);
        assert!((tol.max_sub - 13.0).abs() < 1e-10);
    }
}
