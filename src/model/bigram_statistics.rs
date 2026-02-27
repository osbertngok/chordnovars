use crate::constant::ET_SIZE;
use crate::model::chord_statistics::OrderedChordStatistics;
use crate::model::ordered_chord::OrderedChord;
use crate::model::pitch_iterable::PitchIterable;
use crate::utility::general::{set_complement, set_intersect, set_union, sign};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum OverflowState {
    #[default]
    NoOverflow,
    Single,
    Total,
}

#[derive(Debug, Clone, Default)]
pub struct BigramChordStatistics {
    /// Average Circle of Fifths position of unique pitch classes (kk).
    pub chroma_old: f64,
    /// Previous chord's chroma_old.
    pub prev_chroma_old: f64,
    /// Harmonic distance on the Circle of Fifths (k).
    pub chroma: f64,
    /// Combined harmonic-complexity / voice-leading indicator (Q).
    pub q_indicator: f64,
    /// Exact MIDI pitches shared between both chords (c).
    pub common_note: i32,
    /// Total voice-leading distance in semitones (sv).
    pub sv: i32,
    /// Span of this chord on the Circle of Fifths (s).
    pub span: i32,
    /// Super-span: span of the union of both chords (ss).
    pub sspan: i32,
    /// Voice-leading similarity 0–100 (x).
    pub similarity: i32,
    /// Baseline similarity, always 100 (p).
    pub sim_orig: i32,
    /// Voices staying on the same pitch.
    pub steady_count: i32,
    /// Voices moving up.
    pub ascending_count: i32,
    /// Voices moving down.
    pub descending_count: i32,
    /// Shortest chromatic distance between roots, 0–6.
    pub root_movement: i32,
    /// Human-readable root name (e.g. "C", "F#").
    pub root_name: String,
    /// Whether to suppress octave numbers.
    pub hide_octave: bool,
    /// Note names without octave (e.g. "C E G").
    pub name: String,
    /// Note names with octave (e.g. "C4 E4 G4").
    pub name_with_octave: String,
    /// Type of Circle of Fifths overflow adjustment.
    pub overflow_state: OverflowState,
    /// The ±12 adjustment applied during overflow correction.
    pub overflow_amount: i32,
    /// MIDI note numbers of the chord, sorted.
    pub notes: Vec<i32>,
    /// Unique pitch classes (0–11), sorted.
    pub pitch_class_set: Vec<i32>,
    /// Circle of Fifths position for each note (in MIDI order).
    pub single_chroma: Vec<i32>,
    /// Voice-leading vector (signed semitone movements).
    pub vec: Vec<i32>,
    /// Consecutive intervals of the pitch-class normal form.
    pub self_diff: Vec<i32>,
    /// Interval-class frequency vector (length 6).
    pub count_vec: Vec<i32>,
    /// Scale-degree alignment of each note relative to root.
    pub alignment: Vec<i32>,
}

// Port of midi_to_cof (bigramchordstatistics.cpp)
// 6 - (5 * (midi_note % 12) + 6) % 12
fn midi_to_cof(midi_note: i32) -> i32 {
    6 - (5 * midi_note.rem_euclid(ET_SIZE as i32) + 6) % ET_SIZE as i32
}

// Port of chroma_to_midi_pc (bigramchordstatistics.cpp)
fn chroma_to_midi_pc(chroma: i32) -> i32 {
    const TABLE: [i32; 7] = [5, 0, 7, 2, 9, 4, 11];
    TABLE[((chroma + 36).rem_euclid(7)) as usize] + (chroma + 36) / 7 - 5
}

// Port of chroma_to_name (bigramchordstatistics.cpp)
fn chroma_to_name(chroma: i32) -> String {
    const TABLE: [char; 7] = ['F', 'C', 'G', 'D', 'A', 'E', 'B'];
    let mut result = String::new();
    result.push(TABLE[((chroma + 36).rem_euclid(7)) as usize]);
    let accidental = (chroma + 36) / 7 - 5;
    match accidental {
        -2 => result.push_str("bb"),
        -1 => result.push('b'),
        1 => result.push('#'),
        2 => result.push('x'),
        _ => {}
    }
    result
}

struct SpanResult {
    span: i32,
    sspan: i32,
    adjusted_single_chroma: Vec<i32>,
}

// Port of compute_span_and_adjust (bigramchordstatistics.cpp).
// Finds the minimal CoF span representation and computes the sspan
// (union span with the previous chord).
fn compute_span_and_adjust(
    mut curr_single_chroma: Vec<i32>,
    prev_single_chroma: &[i32],
) -> SpanResult {
    let n = curr_single_chroma.len();
    let mut copy: Vec<i32> = curr_single_chroma.clone();
    copy.sort();

    let mut min_diff1 = copy[n - 1] - copy[0];
    let mut min_bound = copy[0].abs().max(copy[n - 1].abs());
    let mut index: i32 = 0;

    let initial = prev_single_chroma.is_empty();

    if initial {
        for i in 1..n {
            let diff1 = copy[i - 1] + ET_SIZE as i32 - copy[i];
            if diff1 < min_diff1 {
                min_diff1 = diff1;
                min_bound = (copy[i - 1] + ET_SIZE as i32).abs().max(copy[i].abs());
                index = i as i32;
            } else if diff1 == min_diff1 {
                let bound = (copy[i - 1] + ET_SIZE as i32).abs().max(copy[i].abs());
                if bound < min_bound {
                    min_bound = bound;
                    index = i as i32;
                }
            }
        }

        copy = curr_single_chroma.clone();
        copy.sort();
        if index > 0 {
            for sc in &mut curr_single_chroma {
                if *sc <= copy[(index - 1) as usize] {
                    *sc += ET_SIZE as i32;
                }
            }
        } else if index < 0 {
            for sc in &mut curr_single_chroma {
                if *sc >= copy[(-index - 1) as usize] {
                    *sc -= ET_SIZE as i32;
                }
            }
        }

        return SpanResult {
            span: min_diff1,
            sspan: 0,
            adjusted_single_chroma: curr_single_chroma,
        };
    }

    // Non-initial path: sort prev for set_union (std::set_union requires sorted inputs)
    let mut prev_sorted = prev_single_chroma.to_vec();
    prev_sorted.sort();

    let merged = set_union(&prev_sorted, &copy);
    let mut min_diff2 = merged.last().unwrap() - merged.first().unwrap();

    // Forward rotations: try shifting elements +12 one by one
    for i in 1..=(n as i32) {
        copy[(i - 1) as usize] += ET_SIZE as i32;
        let diff1 = copy[(i - 1) as usize] - copy[(i as usize % n)];
        if diff1 < min_diff1 {
            min_diff1 = diff1;
            let mut sorted_copy = copy.clone();
            sorted_copy.sort();
            let merged = set_union(&prev_sorted, &sorted_copy);
            min_diff2 = merged.last().unwrap() - merged.first().unwrap();
            min_bound = copy[(i - 1) as usize].abs().max(copy[(i as usize % n)].abs());
            index = i;
        } else if diff1 == min_diff1 {
            let mut sorted_copy = copy.clone();
            sorted_copy.sort();
            let merged = set_union(&prev_sorted, &sorted_copy);
            let diff2 = merged.last().unwrap() - merged.first().unwrap();
            if diff2 < min_diff2 {
                min_diff2 = diff2;
                min_bound = copy[(i - 1) as usize].abs().max(copy[(i as usize % n)].abs());
                index = i;
            } else if diff2 == min_diff2 {
                let bound = copy[(i - 1) as usize].abs().max(copy[(i as usize % n)].abs());
                if bound < min_bound {
                    min_bound = bound;
                    index = i;
                }
            }
        }
    }

    // Reset copy for backward rotations
    copy = curr_single_chroma.clone();
    copy.sort();

    // Backward rotations: try shifting elements -12 one by one
    for i in (1..=(n as i32)).rev() {
        let j = (i - 2 + n as i32).rem_euclid(n as i32) as usize;
        copy[(i - 1) as usize] -= ET_SIZE as i32;
        let diff1 = copy[j] - copy[(i - 1) as usize];
        if diff1 < min_diff1 {
            min_diff1 = diff1;
            let mut sorted_copy = copy.clone();
            sorted_copy.sort();
            let merged = set_union(&prev_sorted, &sorted_copy);
            min_diff2 = merged.last().unwrap() - merged.first().unwrap();
            min_bound = copy[j].abs().max(copy[(i - 1) as usize].abs());
            index = -i;
        } else if diff1 == min_diff1 {
            let mut sorted_copy = copy.clone();
            sorted_copy.sort();
            let merged = set_union(&prev_sorted, &sorted_copy);
            let diff2 = merged.last().unwrap() - merged.first().unwrap();
            if diff2 < min_diff2 {
                min_diff2 = diff2;
                min_bound = copy[j].abs().max(copy[(i - 1) as usize].abs());
                index = -i;
            } else if diff2 == min_diff2 {
                let bound = copy[j].abs().max(copy[(i - 1) as usize].abs());
                if bound < min_bound {
                    min_bound = bound;
                    index = -i;
                }
            }
        }
    }

    // Apply adjustment based on best index
    copy = curr_single_chroma.clone();
    copy.sort();
    if index > 0 {
        for sc in &mut curr_single_chroma {
            if *sc <= copy[(index - 1) as usize] {
                *sc += ET_SIZE as i32;
            }
        }
    } else if index < 0 {
        for sc in &mut curr_single_chroma {
            if *sc >= copy[(-index - 1) as usize] {
                *sc -= ET_SIZE as i32;
            }
        }
    }

    SpanResult {
        span: min_diff1,
        sspan: min_diff2,
        adjusted_single_chroma: curr_single_chroma,
    }
}

struct ChromaOldResult {
    chroma_old: f64,
    overflow_state: OverflowState,
    adjusted_single_chroma: Vec<i32>,
}

// Port of compute_chroma_old (bigramchordstatistics.cpp).
// Computes the mean CoF position of unique pitch classes, with ±12 wrap if
// the value jumps more than 6 units from the previous chord.
fn compute_chroma_old(
    mut single_chroma: Vec<i32>,
    prev_chroma_old: f64,
) -> ChromaOldResult {
    let mut copy = single_chroma.clone();
    copy.sort();
    copy.dedup();

    let s_size = copy.len() as f64;
    let sum: i32 = copy.iter().sum();
    let mut chroma_old = (sum as f64 / s_size * 100.0).floor() / 100.0;

    let diff = chroma_old - prev_chroma_old;
    let val: i32 = if diff < -18.0 {
        ET_SIZE as i32 * 2
    } else if diff < -6.0 {
        ET_SIZE as i32
    } else if diff > 18.0 {
        -(ET_SIZE as i32 * 2)
    } else if diff > 6.0 {
        -(ET_SIZE as i32)
    } else {
        0
    };

    let overflow_state = if val != 0 {
        for sc in &mut single_chroma {
            *sc += val;
        }
        chroma_old += val as f64;
        OverflowState::Total
    } else {
        OverflowState::NoOverflow
    };

    ChromaOldResult {
        chroma_old,
        overflow_state,
        adjusted_single_chroma: single_chroma,
    }
}

// Port of compute_chroma (bigramchordstatistics.cpp).
// Harmonic distance between two chords via atan of pairwise CoF distances.
fn compute_chroma(
    prev_single_chroma: &[i32],
    curr_single_chroma: &[i32],
    prev_chroma_old: f64,
    curr_chroma_old: f64,
) -> f64 {
    let mut a = prev_single_chroma.to_vec();
    let mut b = curr_single_chroma.to_vec();
    a.sort();
    b.sort();
    a.dedup();
    b.dedup();

    let a_unique = set_complement(&a, &b);
    let b_unique = set_complement(&b, &a);

    let val: i32 = a_unique
        .iter()
        .flat_map(|&ai| b_unique.iter().map(move |&bj| (ai - bj).abs()))
        .sum();

    let s = sign(curr_chroma_old - prev_chroma_old);
    // C++ uses 3.1416 as π approximation
    s as f64 * 2.0 / 3.1416 * (val as f64 / 54.0).atan() * 100.0
}

struct NameResult {
    name: String,
    name_with_octave: String,
    root_name: String,
    overflow_amount: i32,
    final_state: OverflowState,
    final_chroma_old: f64,
    final_prev_chroma_old: f64,
    final_single_chroma: Vec<i32>,
}

// Port of compute_name (bigramchordstatistics.cpp).
// Generates note names, applies a final overflow correction if chroma values
// fall outside the displayable range.
fn compute_name(
    mut single_chroma: Vec<i32>,
    midi_notes: &[i32],
    root_pc_value: u8,
    mut overflow_state: OverflowState,
    mut chroma_old: f64,
    mut prev_chroma_old: f64,
) -> NameResult {
    let n = single_chroma.len();
    let mut copy = single_chroma.clone();
    copy.sort();

    let overflow_amount: i32 = if copy[n - 1] < -6 {
        -(ET_SIZE as i32)
    } else if copy[0] > 6 {
        ET_SIZE as i32
    } else if copy[n - 1] >= 13 && copy[0] >= 4 {
        ET_SIZE as i32
    } else if copy[0] <= -9 && copy[n - 1] <= 0 {
        -(ET_SIZE as i32)
    } else {
        0
    };

    for sc in &mut single_chroma {
        *sc -= overflow_amount;
    }

    if overflow_state == OverflowState::NoOverflow && overflow_amount != 0 {
        overflow_state = OverflowState::Single;
    }

    chroma_old -= overflow_amount as f64;
    prev_chroma_old -= overflow_amount as f64;

    let mut name_parts = Vec::with_capacity(n);
    let mut name_with_octave_parts = Vec::with_capacity(n);
    for i in 0..n {
        let note_name = chroma_to_name(single_chroma[i]);
        let octave = (midi_notes[i] - chroma_to_midi_pc(single_chroma[i])) / ET_SIZE as i32 - 1;
        name_parts.push(note_name.clone());
        name_with_octave_parts.push(format!("{}{}", note_name, octave));
    }

    // Find root position: first note whose MIDI number matches root pitch class
    let mut position = 0;
    for i in 0..n {
        if (midi_notes[i] - root_pc_value as i32).rem_euclid(ET_SIZE as i32) == 0 {
            position = i;
            break;
        }
    }
    let root_name = chroma_to_name(single_chroma[position]);

    NameResult {
        name: name_parts.join(" "),
        name_with_octave: name_with_octave_parts.join(" "),
        root_name,
        overflow_amount,
        final_state: overflow_state,
        final_chroma_old: chroma_old,
        final_prev_chroma_old: prev_chroma_old,
        final_single_chroma: single_chroma,
    }
}

/// Compute bigram statistics describing the relationship between two consecutive chords.
///
/// Parameters mirror the C++ `calculate_bigram_statistics`:
/// - `vec`: voice-leading movement vector (signed semitones per voice)
/// - `sv`: sum of absolute voice-leading distances
/// - `vl_max`: maximum voice-leading interval allowed (used for similarity)
/// - `prev_chroma_old`: chroma_old from the previous bigram (0.0 for first chord)
/// - `prev_single_chroma`: single_chroma from the previous bigram (empty for first chord)
pub fn calculate_bigram_statistics(
    prev_chord: &OrderedChord,
    curr_chord: &OrderedChord,
    prev_stats: &OrderedChordStatistics,
    curr_stats: &OrderedChordStatistics,
    vec: Vec<i32>,
    sv: i32,
    vl_max: i32,
    prev_chroma_old: f64,
    prev_single_chroma: &[i32],
) -> BigramChordStatistics {
    // 1. Count ascending/steady/descending voices
    let (mut ascending_count, mut steady_count, mut descending_count) = (0i32, 0i32, 0i32);
    for &v in &vec {
        if v > 0 {
            ascending_count += 1;
        } else if v == 0 {
            steady_count += 1;
        } else {
            descending_count += 1;
        }
    }

    // 2. Root movement (shortest chromatic distance, 0–6)
    let root_movement = match (prev_stats.root, curr_stats.root) {
        (Some(pr), Some(cr)) => {
            let rm = (cr.value() as i32 - pr.value() as i32 + ET_SIZE as i32) % ET_SIZE as i32;
            if rm > 6 { ET_SIZE as i32 - rm } else { rm }
        }
        _ => 0,
    };

    // 3. Common notes (exact MIDI match)
    let prev_pitches = prev_chord.get_pitches();
    let curr_pitches = curr_chord.get_pitches();
    let mut prev_midi: Vec<i32> = prev_pitches.iter().map(|p| p.get_number() as i32).collect();
    let mut curr_midi: Vec<i32> = curr_pitches.iter().map(|p| p.get_number() as i32).collect();
    prev_midi.sort();
    curr_midi.sort();
    let common_note = set_intersect(&prev_midi, &curr_midi).len() as i32;

    // 4. Similarity: (1 - sv/max_sv), boosted by sqrt if same root
    let max_sv = vl_max as f64 * prev_stats.num_of_pitches.max(curr_stats.num_of_pitches) as f64;
    let mut sim_temp = 1.0 - sv as f64 / max_sv;
    if prev_stats.root.is_some()
        && curr_stats.root.is_some()
        && prev_stats.root == curr_stats.root
    {
        sim_temp = sim_temp.sqrt();
    }
    let similarity = (100.0 * sim_temp).round() as i32;

    // 5. Build CoF positions from sorted MIDI notes
    let curr_single_chroma: Vec<i32> = curr_midi.iter().map(|&m| midi_to_cof(m)).collect();

    // 6. Span and super-span via circular CoF rotation
    let span_result = compute_span_and_adjust(curr_single_chroma, prev_single_chroma);

    // 7. chroma_old: mean CoF position of unique pitch classes, with overflow wrap
    let chroma_old_result =
        compute_chroma_old(span_result.adjusted_single_chroma, prev_chroma_old);

    // 8. Harmonic distance (chroma / k)
    let chroma = if !prev_single_chroma.is_empty() {
        compute_chroma(
            prev_single_chroma,
            &chroma_old_result.adjusted_single_chroma,
            prev_chroma_old,
            chroma_old_result.chroma_old,
        )
    } else {
        0.0
    };

    // 9. Note names and final overflow correction
    let name_result = if let Some(root_pc) = curr_stats.root {
        compute_name(
            chroma_old_result.adjusted_single_chroma,
            &curr_midi,
            root_pc.value(),
            chroma_old_result.overflow_state,
            chroma_old_result.chroma_old,
            prev_chroma_old,
        )
    } else {
        NameResult {
            name: String::new(),
            name_with_octave: String::new(),
            root_name: String::new(),
            overflow_amount: 0,
            final_state: chroma_old_result.overflow_state,
            final_chroma_old: chroma_old_result.chroma_old,
            final_prev_chroma_old: prev_chroma_old,
            final_single_chroma: chroma_old_result.adjusted_single_chroma,
        }
    };

    // 10. Q = chroma * (t1 + t2) / 2 / max(n1, n2)
    let q_indicator = chroma * (prev_stats.tension + curr_stats.tension)
        / 2.0
        / prev_stats.num_of_pitches.max(curr_stats.num_of_pitches) as f64;

    // 11. Unique pitch classes of the current chord
    let mut pc_bits = [false; ET_SIZE];
    for p in &curr_pitches {
        pc_bits[p.get_pitch_class().value() as usize] = true;
    }
    let pitch_class_set: Vec<i32> = (0..ET_SIZE)
        .filter(|&i| pc_bits[i])
        .map(|i| i as i32)
        .collect();

    BigramChordStatistics {
        chroma_old: name_result.final_chroma_old,
        prev_chroma_old: name_result.final_prev_chroma_old,
        chroma,
        q_indicator,
        common_note,
        sv,
        span: span_result.span,
        sspan: span_result.sspan,
        similarity,
        sim_orig: 100,
        steady_count,
        ascending_count,
        descending_count,
        root_movement,
        root_name: name_result.root_name,
        hide_octave: false,
        name: name_result.name,
        name_with_octave: name_result.name_with_octave,
        overflow_state: name_result.final_state,
        overflow_amount: name_result.overflow_amount,
        notes: curr_midi,
        pitch_class_set,
        single_chroma: name_result.final_single_chroma,
        vec,
        self_diff: curr_stats.self_diff.clone(),
        count_vec: curr_stats.count_vec.clone(),
        alignment: curr_stats.alignment.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::chord_statistics::calculate_statistics;

    fn oc(s: &str) -> OrderedChord {
        OrderedChord::from_str_space_separated(s).unwrap()
    }

    fn bigram(
        prev: &str,
        curr: &str,
        vec: Vec<i32>,
        sv: i32,
        vl_max: i32,
        prev_chroma_old: f64,
        prev_single_chroma: &[i32],
    ) -> BigramChordStatistics {
        let pc = oc(prev);
        let cc = oc(curr);
        let ps = calculate_statistics(&pc);
        let cs = calculate_statistics(&cc);
        calculate_bigram_statistics(&pc, &cc, &ps, &cs, vec, sv, vl_max, prev_chroma_old, prev_single_chroma)
    }

    #[test]
    fn root_movement_c_to_g() {
        // C->G: shorter way = 5 semitones
        let r = bigram("C4 E4 G4", "B3 D4 G4", vec![-1, -2, 0], 3, 4, 0.0, &[]);
        assert_eq!(r.root_movement, 5);
    }

    #[test]
    fn root_movement_c_to_fs() {
        // C->F#: tritone = 6
        let r = bigram("C4 E4 G4", "F#4 A4 D-5", vec![6, 5, 5], 16, 6, 0.0, &[]);
        assert_eq!(r.root_movement, 6);
    }

    #[test]
    fn common_notes_c_to_f() {
        // C4 E4 G4 -> C4 F4 A4: only C4 shared
        let r = bigram("C4 E4 G4", "C4 F4 A4", vec![0, 1, 2], 3, 4, 0.0, &[]);
        assert_eq!(r.common_note, 1);
    }

    #[test]
    fn voice_movement_counts() {
        let r = bigram("C4 E4 G4", "C4 F4 A4", vec![0, 1, 2], 3, 4, 0.0, &[]);
        assert_eq!(r.steady_count, 1);
        assert_eq!(r.ascending_count, 2);
        assert_eq!(r.descending_count, 0);
    }

    #[test]
    fn similarity_same_chord() {
        // Identical chord, sv=0: similarity = 100
        let r = bigram("C4 E4 G4", "C4 E4 G4", vec![0, 0, 0], 0, 4, 0.0, &[]);
        assert_eq!(r.similarity, 100);
    }

    #[test]
    fn initial_span_and_sspan() {
        // Initial bigram: sspan=0, span>0
        let r = bigram("C4 E4 G4", "C4 F4 A4", vec![0, 1, 2], 3, 4, 0.0, &[]);
        assert_eq!(r.sspan, 0);
        assert!(r.span > 0);
    }

    #[test]
    fn name_generation_f_major() {
        let r = bigram("C4 E4 G4", "C4 F4 A4", vec![0, 1, 2], 3, 4, 0.0, &[]);
        assert_eq!(r.name, "C F A");
        assert_eq!(r.name_with_octave, "C4 F4 A4");
        assert_eq!(r.root_name, "F");
    }

    #[test]
    fn integration_c_to_f() {
        // Two-step: first compute C major initial bigram to get chroma_old and single_chroma
        let c_bigram = bigram("C4 E4 G4", "C4 E4 G4", vec![0, 0, 0], 0, 4, 0.0, &[]);
        // Then C -> F using those values
        let r = bigram(
            "C4 E4 G4", "C4 F4 A4",
            vec![0, 1, 2], 3, 4,
            c_bigram.chroma_old, &c_bigram.single_chroma,
        );
        assert_eq!(r.common_note, 1);
        assert_eq!(r.sv, 3);
        assert!(r.span > 0);
        assert!(r.sspan > 0);
        assert_eq!(r.root_movement, 5);
        assert_eq!(r.name, "C F A");
        assert_eq!(r.root_name, "F");
        assert_eq!(r.pitch_class_set, vec![0, 5, 9]);
    }
}
