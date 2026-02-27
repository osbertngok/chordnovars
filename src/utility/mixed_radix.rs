/// Mixed-radix counter that generates all mutation vectors in
/// `[-vl_max, vl_max]` per voice, with an optional dead zone `(-vl_min, vl_min)`.
///
/// Port of `MixedRadixRange` / `MixedRadixIterator` from
/// `ChordNova/src/include/utility/mixedradix.h`.
///
/// When `vl_min == 0`, each voice has `2*vl_max + 1` choices.
/// When `vl_min > 0`, values in the open interval `(-vl_min, vl_min)` are
/// skipped, giving each voice `2*(vl_max - vl_min + 1)` choices.
///
/// Iteration order: voice 0 changes fastest (least-significant digit first).
pub struct MixedRadixRange {
    vl_max: i32,
    width: usize,
    vl_min: i32,
}

impl MixedRadixRange {
    /// Create a new range.
    ///
    /// # Arguments
    /// * `vl_max`  – maximum absolute movement per voice (>= 0)
    /// * `width`   – number of voices
    /// * `vl_min`  – dead-zone size (0 = no dead zone; must satisfy 0 <= vl_min <= vl_max)
    pub fn new(vl_max: i32, width: usize, vl_min: i32) -> Self {
        MixedRadixRange { vl_max, width, vl_min }
    }

    /// Total number of vectors this range will produce.
    pub fn total_count(&self) -> i64 {
        let choice = if self.vl_min == 0 {
            2 * self.vl_max + 1
        } else {
            2 * (self.vl_max - self.vl_min + 1)
        } as i64;
        let mut count: i64 = 1;
        for _ in 0..self.width {
            count *= choice;
        }
        count
    }

    pub fn iter(&self) -> MixedRadixIterator {
        MixedRadixIterator::new(self.vl_max, self.width, self.vl_min)
    }
}

impl<'a> IntoIterator for &'a MixedRadixRange {
    type Item = Vec<i32>;
    type IntoIter = MixedRadixIterator;

    fn into_iter(self) -> MixedRadixIterator {
        self.iter()
    }
}

pub struct MixedRadixIterator {
    vl_max: i32,
    vl_min: i32,
    width: usize,
    done: bool,
    vec: Vec<i32>,
}

impl MixedRadixIterator {
    fn new(vl_max: i32, width: usize, vl_min: i32) -> Self {
        let start = -vl_max;
        let vec = vec![start; width];
        // An empty range (width == 0) immediately yields one empty vector
        // then stops, matching C++ behaviour.
        MixedRadixIterator { vl_max, vl_min, width, done: false, vec }
    }

    /// Next valid value after `val` for a single digit.
    /// Returns `None` when overflow (was at `vl_max`).
    fn next_digit(&self, val: i32) -> Option<i32> {
        if val == self.vl_max {
            return None; // carry
        }
        if self.vl_min != 0 && val == -self.vl_min {
            // Skip dead zone: jump from -vl_min to +vl_min
            Some(self.vl_min)
        } else {
            Some(val + 1)
        }
    }

    fn advance(&mut self) {
        for i in 0..self.width {
            match self.next_digit(self.vec[i]) {
                Some(next) => {
                    self.vec[i] = next;
                    return;
                }
                None => {
                    // carry: reset this digit to start value
                    self.vec[i] = -self.vl_max;
                }
            }
        }
        // All digits overflowed
        self.done = true;
    }
}

impl Iterator for MixedRadixIterator {
    type Item = Vec<i32>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let result = self.vec.clone();
        // Handle the width==0 special case: one empty vec, then done
        if self.width == 0 {
            self.done = true;
        } else {
            self.advance();
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_count_no_dead_zone() {
        // vl_max=1, width=2, no dead zone: 3^2 = 9
        assert_eq!(MixedRadixRange::new(1, 2, 0).total_count(), 9);
        // vl_max=2, width=3, no dead zone: 5^3 = 125
        assert_eq!(MixedRadixRange::new(2, 3, 0).total_count(), 125);
    }

    #[test]
    fn total_count_with_dead_zone() {
        // vl_max=4, vl_min=1, width=3: choices per voice = 2*(4-1+1)=8, total=512
        assert_eq!(MixedRadixRange::new(4, 3, 1).total_count(), 512);
        // vl_max=3, vl_min=1, width=2: choices = 2*(3-1+1)=6, total=36
        assert_eq!(MixedRadixRange::new(3, 2, 1).total_count(), 36);
    }

    #[test]
    fn iteration_count_matches_total() {
        for &(vl_max, width, vl_min) in &[(1, 2, 0), (2, 2, 1), (3, 3, 0), (4, 2, 2)] {
            let range = MixedRadixRange::new(vl_max, width, vl_min);
            let expected = range.total_count() as usize;
            let actual = range.iter().count();
            assert_eq!(actual, expected, "vl_max={vl_max}, width={width}, vl_min={vl_min}");
        }
    }

    #[test]
    fn first_vector_is_all_neg_vl_max() {
        let range = MixedRadixRange::new(3, 4, 0);
        let first = range.iter().next().unwrap();
        assert_eq!(first, vec![-3, -3, -3, -3]);
    }

    #[test]
    fn voice0_changes_fastest() {
        // With vl_max=1, width=2, no dead zone, expect:
        // [-1,-1], [0,-1], [1,-1], [-1,0], [0,0], [1,0], [-1,1], [0,1], [1,1]
        let range = MixedRadixRange::new(1, 2, 0);
        let vecs: Vec<Vec<i32>> = range.iter().collect();
        assert_eq!(vecs.len(), 9);
        assert_eq!(vecs[0], vec![-1, -1]);
        assert_eq!(vecs[1], vec![ 0, -1]);
        assert_eq!(vecs[2], vec![ 1, -1]);
        assert_eq!(vecs[3], vec![-1,  0]);
        assert_eq!(vecs[8], vec![ 1,  1]);
    }

    #[test]
    fn dead_zone_values_absent() {
        // vl_max=2, vl_min=1: valid values per voice are [-2, -1, 1, 2]
        let range = MixedRadixRange::new(2, 2, 1);
        for v in range.iter() {
            for &x in &v {
                assert!(x == -2 || x == -1 || x == 1 || x == 2,
                    "unexpected value {x}");
                // 0 must not appear
                assert_ne!(x, 0);
            }
        }
    }

    #[test]
    fn width_zero_yields_one_empty_vec() {
        let range = MixedRadixRange::new(3, 0, 0);
        let vecs: Vec<Vec<i32>> = range.iter().collect();
        assert_eq!(vecs.len(), 1);
        assert_eq!(vecs[0], vec![]);
    }

    #[test]
    fn all_vectors_in_bounds() {
        let range = MixedRadixRange::new(4, 3, 2);
        for v in range.iter() {
            for &x in &v {
                assert!(x >= -4 && x <= 4, "out of bounds: {x}");
            }
        }
    }
}
