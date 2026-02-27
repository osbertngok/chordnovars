use std::collections::HashSet;

use crate::constant::ET_SIZE;
use crate::model::chord_statistics::{calculate_statistics, OrderedChordStatistics};
use crate::model::config::{AlignMode, UniqueMode, VLSetting};
use crate::model::ordered_chord::OrderedChord;
use crate::model::pitch_iterable::PitchIterable;
use crate::model::config::ProgressionConfig;
use crate::service::voice_leading::{find_voice_leading, VoiceLeadingResult};
use crate::utility::general::{set_complement, set_intersect, set_union, sign};

/// Context passed to each validator in the pipeline.
///
/// Port of `ValidationContext` from `ChordNova/src/include/algorithm/validation.h`.
pub struct ValidationContext<'a> {
    pub config: &'a ProgressionConfig,
    pub prev_chord: &'a OrderedChord,
    pub prev_stats: &'a OrderedChordStatistics,

    /// Voice-leading result (populated by `validate_voice_leading`).
    pub vl_result: VoiceLeadingResult,

    /// Lazily-computed statistics for the candidate chord.
    pub candidate_stats: Option<OrderedChordStatistics>,

    /// Cached per-voice chroma values (sorted, deduped) for the candidate.
    /// Computed once by Stage 13 (span), reused by Stage 14 (q_indicator).
    pub candidate_single_chroma: Option<Vec<i32>>,

    /// Set IDs already seen (RemoveDupType uniqueness).
    pub rec_ids: &'a mut HashSet<i32>,

    /// Vec IDs already seen (movement vector uniqueness).
    pub vec_ids: &'a mut HashSet<i64>,

    /// Previous chord's per-voice chroma values (sorted, deduped).
    pub prev_single_chroma: &'a [i32],

    /// Previous chord's chroma_old value.
    pub prev_chroma_old: f64,

    /// History of all accepted chords so far.
    pub record: &'a [OrderedChord],
}

impl<'a> ValidationContext<'a> {
    fn ensure_stats(&mut self, chord: &OrderedChord) -> &OrderedChordStatistics {
        if self.candidate_stats.is_none() {
            self.candidate_stats = Some(calculate_statistics(chord));
        }
        self.candidate_stats.as_ref().unwrap()
    }

    /// Compute and cache per-voice chroma (sorted, deduped).
    /// Used by both span (Stage 13) and q_indicator (Stage 14).
    fn ensure_chroma(&mut self, chord: &OrderedChord) -> &[i32] {
        if self.candidate_single_chroma.is_none() {
            let pitches = chord.get_pitches();
            let mut chroma: Vec<i32> = pitches.iter()
                .map(|p| {
                    let midi = p.get_number() as i32;
                    6 - (5 * (midi % ET_SIZE as i32) + 6).rem_euclid(ET_SIZE as i32)
                })
                .collect();
            chroma.sort_unstable();
            chroma.dedup();
            self.candidate_single_chroma = Some(chroma);
        }
        self.candidate_single_chroma.as_ref().unwrap()
    }
}

// ── Individual Validators ────────────────────────────────────────────────────

/// Stage 1: Notes must be in non-decreasing order.
pub fn validate_monotonicity(_ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let pitches = chord.get_pitches();
    pitches.windows(2).all(|w| w[0].get_number() <= w[1].get_number())
}

/// Stage 2: Notes must be within [lowest, highest] MIDI range.
pub fn validate_range(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let pitches = chord.get_pitches();
    if pitches.is_empty() { return false; }
    let lo = ctx.config.range.lowest as u8;
    let hi = ctx.config.range.highest as u8;
    pitches.first().unwrap().get_number() >= lo
        && pitches.last().unwrap().get_number() <= hi
}

/// Stage 3: Voice alignment validation.
pub fn validate_alignment(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let align = &ctx.config.alignment;
    if align.align_mode == AlignMode::Unlimited {
        return true;
    }

    if align.align_mode == AlignMode::List {
        let stats = ctx.ensure_stats(chord);
        let alignment = stats.alignment.clone();
        return align.alignment_list.iter().any(|a| *a == alignment);
    }

    // Interval mode
    let pitches = chord.get_pitches();
    let size = pitches.len();
    if size < 2 { return true; }

    let nums: Vec<i32> = pitches.iter().map(|p| p.get_number() as i32).collect();
    if nums[1] - nums[0] < align.i_low { return false; }
    if nums[size - 1] - nums[size - 2] > align.i_high { return false; }
    for i in 2..=(size as i32 - 2) {
        let interval = nums[i as usize] - nums[(i - 1) as usize];
        if interval > align.i_max || interval < align.i_min { return false; }
    }
    true
}

/// Stage 4: Exclusion of forbidden notes/roots/intervals.
pub fn validate_exclusion(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let excl = &ctx.config.exclusion;
    if !excl.enabled { return true; }

    let pitches = chord.get_pitches();

    // Forbidden MIDI notes
    for p in &pitches {
        if excl.exclusion_notes.contains(&(p.get_number() as i32)) { return false; }
    }

    // Forbidden roots
    {
        let stats = ctx.ensure_stats(chord);
        if let Some(root) = stats.root {
            if excl.exclusion_roots.contains(&(root.value() as i32)) { return false; }
        }
    }

    // Forbidden interval patterns
    if !excl.exclusion_intervals.is_empty() {
        let m = pitches.len();
        let mut diffs: Vec<i32> = Vec::new();
        for i in 0..m {
            for j in (i + 1)..m {
                diffs.push(pitches[j].get_number() as i32 - pitches[i].get_number() as i32);
            }
        }
        for ei in &excl.exclusion_intervals {
            let mut count = 0;
            for &d in &diffs {
                let temp1 = d - ei.interval;
                let temp2 = d + ei.interval - ET_SIZE as i32;
                if temp1 % ET_SIZE as i32 == 0 {
                    let octave = temp1 / ET_SIZE as i32;
                    if octave >= ei.octave_min && octave <= ei.octave_max { count += 1; }
                } else if temp2 % ET_SIZE as i32 == 0 {
                    let octave = temp2 / ET_SIZE as i32;
                    if octave >= ei.octave_min && octave <= ei.octave_max { count += 1; }
                }
            }
            if count >= ei.num_min && count <= ei.num_max { return false; }
        }
    }
    true
}

/// Stage 5: Pedal note inclusion check.
pub fn validate_pedal(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let pedal = &ctx.config.pedal;
    if !pedal.enabled || !ctx.config.continual { return true; }

    let pitches = chord.get_pitches();

    if pedal.in_bass {
        for (i, &pn) in pedal.pedal_notes.iter().enumerate() {
            if i >= pitches.len() { return false; }
            if pitches[i].get_number() as i32 != pn { return false; }
        }
        return true;
    }

    let record_size = ctx.record.len();
    if record_size % pedal.period as usize == 0 {
        // Must contain all pedal_notes_set pitch classes
        let chord_pcs: HashSet<i32> = pitches.iter()
            .map(|p| p.get_pitch_class().value() as i32)
            .collect();
        for &pc in &pedal.pedal_notes_set {
            if !chord_pcs.contains(&pc) { return false; }
        }
        if pedal.realign && record_size != 0 {
            let mut chord_midi: Vec<i32> = pitches.iter()
                .map(|p| p.get_number() as i32).collect();
            chord_midi.sort_unstable();
            let all_same = pedal.pedal_notes.iter()
                .all(|pn| chord_midi.binary_search(pn).is_ok());
            if all_same { return false; }
        }
        return true;
    }

    // Non-period beat: must contain all pedal_notes exactly (by MIDI number)
    let mut chord_midi: Vec<i32> = pitches.iter()
        .map(|p| p.get_number() as i32).collect();
    chord_midi.sort_unstable();
    pedal.pedal_notes.iter()
        .all(|pn| chord_midi.binary_search(pn).is_ok())
}

/// Stage 6: Note count (m) and pitch-class count (n) constraints.
pub fn validate_cardinality(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let range = &ctx.config.range;
    let (m, n) = {
        let stats = ctx.ensure_stats(chord);
        (stats.num_of_pitches as i32, stats.num_of_unique_pitch_classes as i32)
    };
    m >= range.m_min && m <= range.m_max && n >= range.n_min && n <= range.n_max
}

/// Stage 7: Single-chord statistics (thickness, root, geometrical center).
pub fn validate_single_chord_stats(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let range = &ctx.config.range;
    let (thickness, root_val, g) = {
        let stats = ctx.ensure_stats(chord);
        (stats.thickness, stats.root.map(|r| r.value() as i32), stats.geometrical_center)
    };

    if thickness < range.h_min || thickness > range.h_max { return false; }
    if let Some(r) = root_val {
        if r < range.r_min || r > range.r_max { return false; }
    }
    if g < range.g_min as f64 || g > range.g_max as f64 { return false; }
    true
}

/// Stage 8: All pitch classes must be in the overall scale.
pub fn validate_scale_membership(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let scale = &ctx.config.scale.overall_scale;
    if scale.len() >= ET_SIZE { return true; }

    let scale_set: HashSet<i32> = scale.iter().copied().collect();
    let pitches = chord.get_pitches();
    pitches.iter().all(|p| scale_set.contains(&(p.get_pitch_class().value() as i32)))
}

/// Stage 9: Bass note must be in bass_avail; chord must be in chord library if set.
pub fn validate_bass_and_library(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let alignment = {
        let stats = ctx.ensure_stats(chord);
        stats.alignment.clone()
    };

    if !alignment.is_empty() && !ctx.config.bass.bass_avail.is_empty() {
        let bass_align = alignment[0];
        if !ctx.config.bass.bass_avail.contains(&bass_align) { return false; }
    }

    let library = &ctx.config.chord_library.chord_library;
    if !library.is_empty() {
        let pitches = chord.get_pitches();
        let mut seen: HashSet<i32> = HashSet::new();
        let mut set_id = 0i32;
        for p in &pitches {
            let pc = p.get_pitch_class().value() as i32;
            if seen.insert(pc) {
                set_id += 1 << pc;
            }
        }
        if !library.contains(&set_id) { return false; }
    }
    true
}

/// Stage 10: RemoveDupType uniqueness (each pitch-class set only once).
pub fn validate_uniqueness(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    if ctx.config.uniqueness.unique_mode != UniqueMode::RemoveDupType {
        return true;
    }
    let pitches = chord.get_pitches();
    let mut seen: HashSet<i32> = HashSet::new();
    let mut set_id = 0i32;
    for p in &pitches {
        let pc = p.get_pitch_class().value() as i32;
        if seen.insert(pc) {
            set_id += 1 << pc;
        }
    }
    ctx.rec_ids.insert(set_id)
}

/// Stage 11: Voice-leading vector constraints (range, direction, common notes, sv).
pub fn validate_voice_leading(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    ctx.vl_result = find_voice_leading(ctx.prev_chord, chord);

    let vl = &ctx.config.voice_leading;
    let vec = ctx.vl_result.vec.clone();
    let v_size = vec.len() as i32;

    for &v in &vec {
        let abs_v = v.abs();
        if abs_v < vl.vl_min || abs_v > vl.vl_max { return false; }
    }

    let ascending_count = vec.iter().filter(|&&v| v > 0).count() as f64;
    let steady_count = vec.iter().filter(|&&v| v == 0).count() as f64;
    let descending_count = vec.iter().filter(|&&v| v < 0).count() as f64;

    match vl.vl_setting {
        VLSetting::Default => {
            if v_size >= 2 {
                let mut first_sign = 0i32;
                let mut all_same_dir = true;
                for &v in &vec {
                    if v != 0 {
                        let s = if v > 0 { 1 } else { -1 };
                        if first_sign == 0 { first_sign = s; }
                        else if s != first_sign { all_same_dir = false; break; }
                    } else {
                        all_same_dir = false;
                        break;
                    }
                }
                if all_same_dir && first_sign != 0 { return false; }
            }
        }
        VLSetting::Number => {
            if steady_count < vl.steady_min || steady_count > vl.steady_max { return false; }
            if ascending_count < vl.ascending_min || ascending_count > vl.ascending_max { return false; }
            if descending_count < vl.descending_min || descending_count > vl.descending_max { return false; }
        }
        VLSetting::Percentage => {
            let n = v_size as f64;
            if steady_count < vl.steady_min * n || steady_count > vl.steady_max * n { return false; }
            if ascending_count < vl.ascending_min * n || ascending_count > vl.ascending_max * n { return false; }
            if descending_count < vl.descending_min * n || descending_count > vl.descending_max * n { return false; }
        }
    }

    // Common note check
    let harmonic = &ctx.config.harmonic;
    let prev_midi: Vec<i32> = {
        let mut v: Vec<i32> = ctx.prev_chord.get_pitches().iter()
            .map(|p| p.get_number() as i32).collect();
        v.sort_unstable();
        v
    };
    let curr_midi: Vec<i32> = {
        let mut v: Vec<i32> = chord.get_pitches().iter()
            .map(|p| p.get_number() as i32).collect();
        v.sort_unstable();
        v
    };
    let common = set_intersect(&prev_midi, &curr_midi).len() as i32;
    if common < harmonic.c_min || common > harmonic.c_max { return false; }

    let sv = ctx.vl_result.sv;
    if sv < harmonic.sv_min || sv > harmonic.sv_max { return false; }

    // Root movement check
    if ctx.config.root_movement.enabled {
        let curr_root = {
            let stats = ctx.ensure_stats(chord);
            stats.root
        };
        if let (Some(prev_r), Some(curr_r)) = (ctx.prev_stats.root, curr_root) {
            let rm = ((curr_r.value() as i32 - prev_r.value() as i32)
                .rem_euclid(ET_SIZE as i32))
                .min(ET_SIZE as i32 - (curr_r.value() as i32 - prev_r.value() as i32)
                    .rem_euclid(ET_SIZE as i32));
            let priority = &ctx.config.root_movement.rm_priority;
            if (rm as usize) < priority.len() && priority[rm as usize] == -1 {
                return false;
            }
        }
    }

    true
}

/// Stage 12: Similarity constraint.
pub fn validate_similarity(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let harmonic = ctx.config.harmonic.clone();
    let vl_max = ctx.config.voice_leading.vl_max;
    let prev_n = ctx.prev_stats.num_of_pitches;
    let prev_root = ctx.prev_stats.root;
    let (curr_n, curr_root) = {
        let stats = ctx.ensure_stats(chord);
        (stats.num_of_pitches, stats.root)
    };

    let max_sv = vl_max as f64 * prev_n.max(curr_n) as f64;
    let sv = ctx.vl_result.sv as f64;

    let mut sim_temp = 0.0f64;
    if max_sv > 0.0 {
        sim_temp = (1.0 - sv / max_sv).powf(1.0);
    }
    if let (Some(pr), Some(cr)) = (prev_root, curr_root) {
        if pr == cr {
            sim_temp = sim_temp.sqrt();
        }
    }
    let similarity = (100.0 * sim_temp).round() as i32;
    if similarity < harmonic.x_min || similarity > harmonic.x_max { return false; }

    // Extended similarity check
    if ctx.config.similarity.enabled {
        let sim_cfg = ctx.config.similarity.clone();
        for i in 0..sim_cfg.sim_period.len() {
            let period = sim_cfg.sim_period[i] as usize;
            if ctx.record.len() >= period {
                let lookback_chord = &ctx.record[ctx.record.len() - period];
                let lookback_stats = calculate_statistics(lookback_chord);
                let vl = find_voice_leading(lookback_chord, chord);
                let lb_max_sv = vl_max as f64
                    * lookback_stats.num_of_pitches.max(curr_n) as f64;
                let mut lb_sim = 0.0f64;
                if lb_max_sv > 0.0 {
                    lb_sim = (1.0 - vl.sv as f64 / lb_max_sv).powf(1.0);
                }
                if let (Some(lr), Some(cr2)) = (lookback_stats.root, curr_root) {
                    if lr == cr2 { lb_sim = lb_sim.sqrt(); }
                }
                let lb_similarity = (100.0 * lb_sim).round() as i32;
                if lb_similarity < sim_cfg.sim_min[i] || lb_similarity > sim_cfg.sim_max[i] {
                    return false;
                }
            }
        }
    }
    true
}

/// Stage 13: Circle of Fifths span and super-span constraints.
pub fn validate_span(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let harmonic = &ctx.config.harmonic;

    // Use cached chroma (sorted, deduped)
    let sorted_chroma = ctx.ensure_chroma(chord).to_vec();

    let n = sorted_chroma.len() as i32;
    if n <= 1 {
        if 0 < harmonic.s_min { return false; }
        return true;
    }

    let mut min_span = sorted_chroma[n as usize - 1] - sorted_chroma[0];
    for i in 1..n as usize {
        let rotated = sorted_chroma[i - 1] + ET_SIZE as i32 - sorted_chroma[i];
        if rotated < min_span { min_span = rotated; }
    }
    if min_span < harmonic.s_min || min_span > harmonic.s_max { return false; }

    if !ctx.prev_single_chroma.is_empty() {
        let merged = set_union(ctx.prev_single_chroma, &sorted_chroma);
        let mn = merged.len() as i32;
        let mut sspan = merged[mn as usize - 1] - merged[0];
        for i in 1..mn as usize {
            let rotated = merged[i - 1] + ET_SIZE as i32 - merged[i];
            if rotated < sspan { sspan = rotated; }
        }
        if sspan < harmonic.ss_min || sspan > harmonic.ss_max { return false; }
    }
    true
}

/// Stage 14: Q indicator constraint.
pub fn validate_q_indicator(ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
    let harmonic = ctx.config.harmonic.clone();
    let prev_tension = ctx.prev_stats.tension;
    let prev_n = ctx.prev_stats.num_of_pitches;
    let (curr_tension, curr_n) = {
        let stats = ctx.ensure_stats(chord);
        (stats.tension, stats.num_of_pitches)
    };

    // Use cached chroma (sorted, deduped)
    let sorted_unique = ctx.ensure_chroma(chord).to_vec();

    let sum: i32 = sorted_unique.iter().sum();
    let mut curr_chroma_old = (sum as f64 / sorted_unique.len() as f64 * 100.0).floor() / 100.0;

    if curr_chroma_old - ctx.prev_chroma_old < -6.0 {
        curr_chroma_old += ET_SIZE as f64;
    } else if curr_chroma_old - ctx.prev_chroma_old > 6.0 {
        curr_chroma_old -= ET_SIZE as f64;
    }

    let chroma = if !ctx.prev_single_chroma.is_empty() {
        let mut prev_unique = ctx.prev_single_chroma.to_vec();
        prev_unique.sort_unstable();
        prev_unique.dedup();

        let a_only = set_complement(&prev_unique, &sorted_unique);
        let b_only = set_complement(&sorted_unique, &prev_unique);

        let val: i32 = a_only.iter()
            .flat_map(|&a| b_only.iter().map(move |&b| (a - b).abs()))
            .sum();

        let s = sign(curr_chroma_old - ctx.prev_chroma_old) as f64;
        s * 2.0 / 3.1416 * (val as f64 / 54.0).atan() * 100.0
    } else {
        0.0
    };

    let q = chroma * (prev_tension + curr_tension)
        / 2.0 / prev_n.max(curr_n) as f64;

    q >= harmonic.q_min && q <= harmonic.q_max
}

/// Stage 15: Movement vector uniqueness.
pub fn validate_vec_uniqueness(ctx: &mut ValidationContext, _chord: &OrderedChord) -> bool {
    let vec = &ctx.vl_result.vec;
    let mut vec_id: i64 = 0;
    let mut base: i64 = 1;
    for &v in vec {
        vec_id += (v as i64 + 100) * base;
        base *= 200;
    }
    ctx.vec_ids.insert(vec_id)
}

// ── Validation Pipeline ──────────────────────────────────────────────────────

type ValidatorFn = fn(&mut ValidationContext, &OrderedChord) -> bool;

/// Chains all 15 validators with short-circuit evaluation.
///
/// Port of `ChordValidationPipeline` from `ChordNova/src/include/algorithm/validation.h`.
pub struct ChordValidationPipeline {
    validators: Vec<ValidatorFn>,
}

impl ChordValidationPipeline {
    /// Constructs the optimized pipeline.
    ///
    /// Stages removed (now done as early pruning in the mutation loop):
    /// - Range (Stage 2): checked during pitch construction
    /// - Cardinality (Stage 6): m is deterministic from expansion; n checked early
    /// - Scale membership (Stage 8): checked during pitch construction
    ///
    /// The chord library subset of bass_and_library (Stage 9) is also checked early,
    /// but bass_avail and alignment checks still require the full pipeline.
    pub fn new() -> Self {
        ChordValidationPipeline {
            validators: vec![
                validate_monotonicity,       // Stage 1
                validate_alignment,          // Stage 3
                validate_exclusion,          // Stage 4
                validate_pedal,              // Stage 5
                validate_single_chord_stats, // Stage 7
                validate_bass_and_library,   // Stage 9 (bass_avail only; library checked early)
                validate_uniqueness,         // Stage 10
                validate_voice_leading,      // Stage 11
                validate_similarity,         // Stage 12
                validate_span,               // Stage 13
                validate_q_indicator,        // Stage 14
                validate_vec_uniqueness,     // Stage 15
            ],
        }
    }

    /// Runs the pipeline on a candidate chord.
    ///
    /// Returns `true` if all stages pass.
    pub fn validate(&self, ctx: &mut ValidationContext, chord: &OrderedChord) -> bool {
        self.validators.iter().all(|f| f(ctx, chord))
    }
}

impl Default for ChordValidationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::config::ProgressionConfig;
    use crate::model::pitch::Pitch;
    use crate::model::pitch_iterable::PitchIterable;

    fn make_chord(notes: &[u8]) -> OrderedChord {
        OrderedChord::new(notes.iter().copied().map(Pitch::new).collect())
    }

    fn make_context<'a>(
        config: &'a ProgressionConfig,
        prev_chord: &'a OrderedChord,
        prev_stats: &'a OrderedChordStatistics,
        rec_ids: &'a mut HashSet<i32>,
        vec_ids: &'a mut HashSet<i64>,
    ) -> ValidationContext<'a> {
        ValidationContext {
            config,
            prev_chord,
            prev_stats,
            vl_result: VoiceLeadingResult::default(),
            candidate_stats: None,
            candidate_single_chroma: None,
            rec_ids,
            vec_ids,
            prev_single_chroma: &[],
            prev_chroma_old: 0.0,
            record: &[],
        }
    }

    #[test]
    fn monotonicity_pass() {
        let chord = make_chord(&[60, 64, 67]);
        let config = ProgressionConfig::default();
        let prev = make_chord(&[60, 64, 67]);
        let prev_stats = calculate_statistics(&prev);
        let mut rec_ids = HashSet::new();
        let mut vec_ids = HashSet::new();
        let mut ctx = make_context(&config, &prev, &prev_stats, &mut rec_ids, &mut vec_ids);
        assert!(validate_monotonicity(&mut ctx, &chord));
    }

    #[test]
    fn monotonicity_fail() {
        let chord = make_chord(&[67, 60, 64]); // descending
        let config = ProgressionConfig::default();
        let prev = make_chord(&[60, 64, 67]);
        let prev_stats = calculate_statistics(&prev);
        let mut rec_ids = HashSet::new();
        let mut vec_ids = HashSet::new();
        let mut ctx = make_context(&config, &prev, &prev_stats, &mut rec_ids, &mut vec_ids);
        assert!(!validate_monotonicity(&mut ctx, &chord));
    }

    #[test]
    fn range_pass() {
        let chord = make_chord(&[60, 64, 67]);
        let config = ProgressionConfig::default(); // lowest=0, highest=127
        let prev = make_chord(&[60, 64, 67]);
        let prev_stats = calculate_statistics(&prev);
        let mut rec_ids = HashSet::new();
        let mut vec_ids = HashSet::new();
        let mut ctx = make_context(&config, &prev, &prev_stats, &mut rec_ids, &mut vec_ids);
        assert!(validate_range(&mut ctx, &chord));
    }

    #[test]
    fn pipeline_accepts_valid_chord() {
        // C major to G major
        let prev = make_chord(&[60, 64, 67]);
        let prev_stats = calculate_statistics(&prev);
        let config = ProgressionConfig::default();
        let mut rec_ids = HashSet::new();
        let mut vec_ids = HashSet::new();
        let mut ctx = make_context(&config, &prev, &prev_stats, &mut rec_ids, &mut vec_ids);

        // C4 F4 A4: voices move [0, +1, +2] — not all same direction, passes Default VL
        let candidate = make_chord(&[60, 65, 69]);
        let pipeline = ChordValidationPipeline::new();
        assert!(pipeline.validate(&mut ctx, &candidate));
    }

    #[test]
    fn range_rejection_via_validate_range() {
        // Range checking is now done early in the mutation loop (progression.rs),
        // but validate_range still works standalone for direct callers.
        use crate::model::config::RangeConstraints;
        let prev = make_chord(&[60, 64, 67]);
        let prev_stats = calculate_statistics(&prev);
        let mut config = ProgressionConfig::default();
        config.range = RangeConstraints { highest: 65, ..RangeConstraints::default() };

        let mut rec_ids = HashSet::new();
        let mut vec_ids = HashSet::new();
        let mut ctx = make_context(&config, &prev, &prev_stats, &mut rec_ids, &mut vec_ids);

        let candidate = make_chord(&[60, 64, 67]); // G4=67 > highest=65
        assert!(!validate_range(&mut ctx, &candidate));
    }
}
