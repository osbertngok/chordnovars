use std::fs;

use crate::constant::ET_SIZE;

// Scale degree position lookup (same as NOTE_POS in general.rs, used by C++ legacy).
const NOTE_POS: [i32; 12] = [1, 9, 9, 3, 3, 11, 11, 5, 13, 13, 7, 7];

// Which scale degrees are essential (non-omissible) for chords of size 3-7.
const OMISSION: [&[i32]; 8] = [
    &[],            // 0
    &[],            // 1
    &[],            // 2
    &[1, 3, 5],     // 3-note: root, 3rd, 5th
    &[1, 3, 7],     // 4-note: root, 3rd, 7th
    &[1, 3, 7],     // 5-note
    &[1, 3, 7],     // 6-note
    &[1, 3, 7],     // 7-note
];

/// Simple root-finding for a pitch-class set.
fn find_root_local(note_set: &[i32]) -> i32 {
    if note_set.is_empty() { return 0; }
    let mut best_root = note_set[0];
    let mut best_score = 0;
    for &candidate in note_set {
        let mut score = 0;
        for &n in note_set {
            let interval = (n - candidate).rem_euclid(ET_SIZE as i32);
            score += match interval {
                7 => 3,
                4 | 3 => 2,
                0 => 1,
                _ => 0,
            };
        }
        if score > best_score {
            best_score = score;
            best_root = candidate;
        }
    }
    best_root
}

/// Generate all 12 transpositions of a pitch-class set as bitmask set_ids.
fn note_set_to_ids(note_set: &[i32], rec: &mut Vec<i32>) {
    for j in 0..ET_SIZE as i32 {
        let mut val = 0i32;
        for &n in note_set {
            val += 1 << ((n + j).rem_euclid(ET_SIZE as i32));
        }
        rec.push(val);
    }
}

/// Read a chord database file and return all set_id bitmask values.
///
/// Each non-comment line contains space-separated pitch class integers (0–11).
/// Lines beginning with `'/'` or `'t'` are skipped as comments.
///
/// For each chord, all 12 transpositions are generated, and for 3–7 note chords,
/// omissible notes are removed in all subsets, generating additional set_ids.
///
/// Port of `read_chord_database()` from `ChordNova/src/io/database.cpp`.
pub fn read_chord_database(filename: &str) -> Vec<i32> {
    let content = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut chord_library: Vec<i32> = Vec::new();

    for line in content.lines() {
        if line.is_empty() { continue; }
        let first = line.bytes().next().unwrap();
        if first == b'/' || first == b't' { continue; }

        let note_set: Vec<i32> = line
            .split_whitespace()
            .filter_map(|s| s.parse::<i32>().ok())
            .collect();
        if note_set.is_empty() { continue; }

        let mut note_set = note_set;
        note_set.sort_unstable();

        let s_size = note_set.len();
        let root = find_root_local(&note_set);

        // Identify omissible notes
        let mut omit_choice: Vec<i32> = Vec::new();
        if s_size >= 3 && s_size <= 7 {
            for &n in &note_set {
                let diff = (n - root).rem_euclid(ET_SIZE as i32) as usize;
                let pos = NOTE_POS[diff];
                let essential = OMISSION[s_size].contains(&pos);
                if !essential {
                    omit_choice.push(n);
                }
            }
        }

        // Full chord
        note_set_to_ids(&note_set, &mut chord_library);

        // All non-empty subsets of omissible notes (each subset = notes to remove)
        let max_id = (1 << omit_choice.len()) as i32 - 1;
        for id in 1..=max_id {
            let omitted: Vec<i32> = note_set.iter()
                .filter(|&&n| {
                    let in_subset = omit_choice.iter().enumerate()
                        .any(|(bit, &oc)| (id & (1 << bit)) != 0 && oc == n);
                    !in_subset
                })
                .copied()
                .collect();
            if !omitted.is_empty() && omitted.len() < s_size {
                note_set_to_ids(&omitted, &mut chord_library);
            }
        }
    }

    chord_library.sort_unstable();
    chord_library.dedup();
    chord_library
}

/// Read an alignment database file.
///
/// Skips 5 header lines, then reads one alignment per line as space-separated
/// voice indices. All cyclic rotations are pre-expanded.
///
/// Port of `read_alignment_database()` from `ChordNova/src/io/database.cpp`.
pub fn read_alignment_database(filename: &str) -> Vec<Vec<i32>> {
    let content = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut alignment_list: Vec<Vec<i32>> = Vec::new();
    let mut lines = content.lines();

    // Skip 5 header lines
    for _ in 0..5 { lines.next(); }

    for line in lines {
        if line.is_empty() { continue; }
        let mut single: Vec<i32> = line
            .split_whitespace()
            .filter_map(|s| s.parse::<i32>().ok())
            .collect();
        if single.is_empty() { continue; }

        let len = single.len();
        for _ in 0..len {
            alignment_list.push(single.clone());
            // Rotate: move first element to end
            let first = single.remove(0);
            single.push(first);
        }
    }

    alignment_list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_missing_file_returns_empty() {
        let result = read_chord_database("/nonexistent/path/to/db.txt");
        assert!(result.is_empty());
    }

    #[test]
    fn read_alignment_missing_returns_empty() {
        let result = read_alignment_database("/nonexistent/path.txt");
        assert!(result.is_empty());
    }

    #[test]
    fn note_set_to_ids_generates_12_entries() {
        let mut rec: Vec<i32> = Vec::new();
        note_set_to_ids(&[0, 4, 7], &mut rec); // C major
        assert_eq!(rec.len(), 12);
    }

    #[test]
    fn find_root_c_major() {
        // C major: {0, 4, 7} → root should be C (0)
        assert_eq!(find_root_local(&[0, 4, 7]), 0);
    }
}
