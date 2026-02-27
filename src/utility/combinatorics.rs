use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

/// Binomial coefficient C(n, k). Returns 0 if k < 0 or k > n.
pub fn comb(n: i32, k: i32) -> i32 {
    if k < 0 || k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }
    let k = k.min(n - k);
    let mut result = 1;
    for i in 1..=k {
        result = result * (n + 1 - i) / i;
    }
    result
}

// Port of insert_positions() from combinatorics.cpp.
// Inserts `pos.len()` extra elements into `ar` (processed right-to-left),
// each inserted at position pos[i]+1 with value pos[i].
fn insert_positions(ar: &mut Vec<i32>, pos: &[i32]) {
    for i in (0..pos.len()).rev() {
        let len1 = ar.len();
        ar.push(0);
        let mut j = len1 as i32 - 1;
        while j > pos[i] {
            ar[(j + 1) as usize] = ar[j as usize];
            j -= 1;
        }
        ar[(pos[i] + 1) as usize] = pos[i];
    }
}

// Port of compute_expansions() from combinatorics.cpp.
// Generates all expansion index vectors for a (min_size, max_size) pair.
fn compute_expansions(min_size: i32, max_size: i32) -> Vec<Vec<i32>> {
    let diff = max_size - min_size;
    let total = comb(max_size - 1, min_size - 1) as usize;

    // Initialize: each array is [0, 1, ..., min_size-1]
    let mut result: Vec<Vec<i32>> = (0..total)
        .map(|_| (0..min_size).collect())
        .collect();

    if diff == 0 {
        return result;
    }

    let mut pos = vec![0i32; 15];
    let mut index1: i32 = 0;
    let mut index2: i32 = 0;
    let mut index3: usize = 0;

    while index1 >= 0 {
        if index2 >= min_size {
            index1 -= 1;
            if index1 < 0 {
                break; // avoid OOB read of pos[-1] (mirrors C++ UB-but-harmless exit)
            }
            index2 = pos[index1 as usize] + 1;
        } else if index1 == diff {
            let positions: Vec<i32> = pos[..diff as usize].to_vec();
            insert_positions(&mut result[index3], &positions);
            index1 -= 1;
            index2 += 1;
            index3 += 1;
        } else {
            pos[index1 as usize] = index2;
            index1 += 1;
        }
    }

    result
}

// Lazy singleton cache: (min_size, max_size) → expansion table
static EXPANSION_CACHE: Lazy<Mutex<HashMap<(i32, i32), Vec<Vec<i32>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Returns the total number of expansions for (min_size, max_size).
///
/// Equals C(max_size - 1, min_size - 1).
pub fn expansion_count(min_size: i32, max_size: i32) -> i32 {
    comb(max_size - 1, min_size - 1)
}

/// Returns the expansion index vector for a given (min_size, max_size, index).
///
/// Each returned vector has length `max_size` with values in [0, min_size-1],
/// sorted ascending. It describes which source note each voice maps to.
///
/// Returns `None` if parameters are out of range (min_size or max_size outside
/// 1..=15, or index out of [0, count)).
pub fn expansion_get(min_size: i32, max_size: i32, index: i32) -> Option<Vec<i32>> {
    if min_size < 1 || min_size > 15 || max_size < min_size || max_size > 15 {
        return None;
    }
    let total = comb(max_size - 1, min_size - 1);
    if index < 0 || index >= total {
        return None;
    }

    let mut cache = EXPANSION_CACHE.lock().unwrap();
    let key = (min_size, max_size);
    if !cache.contains_key(&key) {
        let expansions = compute_expansions(min_size, max_size);
        cache.insert(key, expansions);
    }
    Some(cache[&key][index as usize].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comb_basics() {
        assert_eq!(comb(0, 0), 1);
        assert_eq!(comb(5, 0), 1);
        assert_eq!(comb(5, 5), 1);
        assert_eq!(comb(14, 1), 14);
        assert_eq!(comb(3, 5), 0);
        assert_eq!(comb(5, -1), 0);
    }

    #[test]
    fn comb_symmetry() {
        assert_eq!(comb(10, 3), comb(10, 7));
        assert_eq!(comb(8, 2), comb(8, 6));
    }

    #[test]
    fn comb_known_values() {
        assert_eq!(comb(4, 2), 6);
        assert_eq!(comb(5, 2), 10);
        assert_eq!(comb(6, 3), 20);
        assert_eq!(comb(10, 5), 252);
    }

    #[test]
    fn expansion_identity() {
        // min_size == max_size: only 1 expansion, identity [0, 1, 2]
        assert_eq!(expansion_count(3, 3), 1);
        assert_eq!(expansion_get(3, 3, 0), Some(vec![0, 1, 2]));
    }

    #[test]
    fn expansion_three_to_five_count() {
        assert_eq!(expansion_count(3, 5), 6);
    }

    #[test]
    fn expansion_three_to_five_sorted() {
        for i in 0..6 {
            let exp = expansion_get(3, 5, i).unwrap();
            assert_eq!(exp.len(), 5);
            // sorted ascending
            for j in 1..exp.len() {
                assert!(exp[j - 1] <= exp[j]);
            }
            // all values in [0, 2]
            for &v in &exp {
                assert!(v >= 0 && v <= 2);
            }
        }
    }

    #[test]
    fn expansion_one_to_any() {
        // 1 note to 5 voices: only [0,0,0,0,0]
        assert_eq!(expansion_count(1, 5), 1);
        assert_eq!(expansion_get(1, 5, 0), Some(vec![0, 0, 0, 0, 0]));
    }

    #[test]
    fn expansion_out_of_range() {
        assert_eq!(expansion_get(0, 5, 0), None);
        assert_eq!(expansion_get(3, 5, 6), None);
        assert_eq!(expansion_get(16, 16, 0), None);
    }
}
