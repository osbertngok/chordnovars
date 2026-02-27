# ChordNova Rust Port — Implementation Plan & Progress

## Overview

This is a full Rust rewrite of the C++ `revamp-phase-2` branch of ChordNova, located at `/Users/osbertngok/git/ChordNova`.

- **C++ source:** ~10k lines of source + ~3.6k lines of tests across 264 test cases
- **Rust repo:** This repo — `rust-port` branch
- **Goal:** Full feature parity with C++ revamp, idiomatic Rust

## What Was Kept vs Rewritten

- **Kept:** `src/parser/pitch.pest` (PEG grammar) and `src/parser/pitch_parser.rs`
- **Rewritten:** Everything else. The old `src/chordnova/` directory (pitch.rs, chord.rs, etc.) conflated concerns that the C++ revamp properly separates. The old files remain in `src/chordnova/` but are **not used** — all new code lives in the new module structure.

---

## Module Structure

```
src/
  lib.rs                         -- Library crate root
  main.rs                        -- Binary entry point (thin)
  constant.rs                    -- ET_SIZE=12, FIFTH_IN_SEMITONE=7
  model/
    mod.rs
    pitch_class.rs               -- PitchClass, Chroma, Semitone, COFUnit ✅
    octave.rs                    -- Octave enum ✅
    pitch.rs                     -- Pitch(u8), pest-based FromStr ✅
    pitch_iterable.rs            -- PitchIterable trait ✅
    ordered_chord.rs             -- OrderedChord (impl PitchIterable) ✅
    pitch_set.rs                 -- PitchSet (impl PitchIterable) ✅ (1 test needs fix)
    chord_statistics.rs          -- OrderedChordStatistics, calculate_statistics()
    bigram_statistics.rs         -- OverflowState, BigramChordStatistics, calculate_bigram_statistics()
    config.rs                    -- All constraint/config structs + ProgressionConfig
    substitution_config.rs       -- ParamTolerance, SubstitutionConfig
  parser/
    mod.rs
    pitch.pest                   -- (kept from original)
    pitch_parser.rs              -- (kept from original)
  utility/
    mod.rs
    combinatorics.rs             -- comb(), ExpansionIndexCache
    mixed_radix.rs               -- MixedRadixRange (impl Iterator)
    general.rs                   -- normal_form, set ops, sign ✅
    midi_encoding.rs             -- to_vlq, swap_int
  service/
    mod.rs
    expansion.rs                 -- expand(), expand_single()
    voice_leading.rs             -- VoiceLeadingResult, find_voice_leading()
    chord_library.rs             -- ChordLibrary (lazy singleton)
  algorithm/
    mod.rs
    validation.rs                -- 15-stage ChordValidationPipeline
    sorting.rs                   -- CandidateEntry, sort_candidates()
    progression.rs               -- ProgressionResult, generate_single()
    substitution.rs              -- substitute() with 3 modes
    analysis.rs                  -- AnalysisResult, analyse()
  io/
    mod.rs
    note_parser.rs               -- parse_notes(), nametonum()
    formatter.rs                 -- format_candidates()
    midi.rs                      -- MidiConfig, write_midi()
    database.rs                  -- read_chord_database(), read_alignment_database()
```

---

## Phase Progress

| # | Phase | Status | Tests |
|---|-------|--------|-------|
| 1 | Project Setup & Constants | ✅ Done | builds |
| 2 | PitchClass, Chroma, Semitone, COFUnit | ✅ Done | 6 pass |
| 3 | Octave & Pitch | ✅ Done | 5 pass |
| 4 | General Utility & Chord Constants | ✅ Done | 5 pass |
| 5 | PitchIterable, OrderedChord, PitchSet | ✅ Done | 8/8 pass |
| 6 | OrderedChordStatistics | ✅ Done | 4 pass |
| 7 | BigramChordStatistics | ✅ Done | 9 pass |
| 8 | Config Enums & Structs | ✅ Done | 2 pass |
| 9 | Combinatorics & MixedRadixRange | ✅ Done | 16 pass |
| 10 | Expansion & Voice Leading | ✅ Done | 10 pass |
| 11 | ChordLibrary & MIDI Encoding | ✅ Done | 10 pass |
| 12 | Validation Pipeline | ✅ Done | 5 pass |
| 13 | Sorting & Progression | ✅ Done | 6 pass |
| 14 | Substitution & Analysis | ✅ Done | 14 pass |
| 15 | I/O Layer | ✅ Done | 15 pass |

---

## Notes

### `get_span()` vs bigram `span`

`PitchIterable::get_span()` is defined but **never called** in C++ — the `span` field in `BigramChordStatistics` uses a separate `compute_span_and_adjust()` based on chroma vectors. The C++ `get_span()` has swapped arguments (a dead-code bug). Rust port fixes this: loop uses `get_circle_of_fifth_distance(pcs[i], pcs[(i+1)%n])` (forward gap), giving span=4 for C major as the comment intends.

---

## Phase Details

### Phase 1: Project Setup & Constants
- Created `rust-port` branch from `master`
- New structure: `src/lib.rs`, `src/main.rs`, `src/constant.rs`
- Moved pest files to `src/parser/`
- Added `once_cell = "1.19"` to `Cargo.toml`
- Created all module skeleton files
- **C++ ref:** `src/include/constant.h`

### Phase 2: PitchClass, Chroma, Semitone, COFUnit
**File:** `src/model/pitch_class.rs`

Key types:
- `Chroma(i32)` — position on Circle of Fifths (C=0, G=1, F=-1, etc.)
- `Semitone(i32)` — interval in semitones
- `COFUnit(i32)` — distance on Circle of Fifths
- `PitchClass(u8)` — 0..11, with constants C, CS, DB, D, etc.

Key functions:
- `PitchClass::get_chroma()` — `ET_SIZE/2 - ((ET_SIZE/2 - 1)*v + ET_SIZE/2) % ET_SIZE`
- `get_interval(from, to) -> Semitone`
- `get_circle_of_fifth_distance(from, to) -> COFUnit` — `7 * interval % 12`

String representation uses Circle of Fifths naming (F#, Bb, etc.), not C#/Db.
FromStr supports: A-, A, A#, B-, B, C, Cs, C#, D-, D, D#, E-, E, F, F#, G-, G, G#

**C++ ref:** `src/include/model/pitchclass.h`, `src/model/pitchclass.cpp`

### Phase 3: Octave & Pitch
**Files:** `src/model/octave.rs`, `src/model/pitch.rs`

- `Octave` enum: OMinus1(-1) through O7(7), `#[repr(i8)]`
- `Pitch(u8)` — MIDI note number 0–127
- `Pitch::from_pitch_class_octave(pc, oct)` — `pc.value() + (oct + 1) * 12`
- `Pitch - Pitch -> i32`
- `Display` for Pitch: e.g. `"C4"`, `"F#5"`, `"Bb3"`
- `FromStr` uses the pest parser from `src/parser/`

**C++ ref:** `src/include/model/pitch.h`, `src/model/pitch.cpp`

### Phase 4: General Utility & Chord Constants
**File:** `src/utility/general.rs`

Constants:
- `ZXS_TENSION_WEIGHT_VECTOR: [f64; 12]` = `[0,11,8,6,5,3,7,3,5,6,8,11]`
- `RESTRICTION: [i32; 12]` = `[0,53,53,51,50,51,52,39,51,50,51,52]`
- `OVERALL_SCALE: [i32; 12]` = `[0..11]`
- `NOTE_POS: [i32; 12]` = `[1,9,9,3,3,11,11,5,13,13,7,7]`

Functions: `normal_form()`, `set_intersect()`, `set_union()`, `set_complement()`, `sign()`

**C++ ref:** `src/utility.cpp`, `src/include/model/chord.h`

### Phase 5: PitchIterable, OrderedChord, PitchSet
**Files:** `src/model/pitch_iterable.rs`, `src/model/ordered_chord.rs`, `src/model/pitch_set.rs`

`PitchIterable` trait:
- `contains_pitch_class`, `contains_pitch`, `get_tension`, `get_thickness`
- `get_geometrical_center`, `find_root`, `get_pitches`
- `get_pitch_classes_ordered_by_circle_of_fifths`
- `get_span()` — default impl: `12 - max(adjacent CoF gaps)`

`PitchSet(BTreeSet<Pitch>)`:
- `find_root()` uses `interval_rank: [11,8,6,5,3,0,10,1,2,4,7,9]`
  - odd rank → lower note is root; even rank → upper note is root
- Tension: `Σ ZXS_TENSION_WEIGHT_VECTOR[diff%12] / (diff/12 + 1)` with low-register penalty, divided by 10
- Thickness: `Σ 12/diff` for octave intervals
- Geometrical center: `(mean_pitch - min) / (max - min)`

`OrderedChord(Vec<Pitch>)`:
- Delegates all analysis to `to_set()`
- `Ord` for use as map keys: by length, then lexicographic pitch order

**C++ ref:** `src/include/model/pitchiterable.h`, `src/model/orderedchord.cpp`, `src/model/pitchset.cpp`

---

## Remaining Phases (to implement)

### Phase 6: OrderedChordStatistics
**File:** `src/model/chord_statistics.rs`

```rust
pub struct OrderedChordStatistics {
    pub num_of_pitches: usize,
    pub num_of_unique_pitch_classes: usize,
    pub tension: f64,
    pub thickness: f64,
    pub root: Option<PitchClass>,
    pub geometrical_center: f64,
    pub alignment: i32,
    pub self_diff: Vec<i32>,
    pub count_vec: Vec<i32>,  // length 12
}
pub fn calculate_statistics(chord: &OrderedChord) -> OrderedChordStatistics
```

- `alignment`: sum of `NOTE_POS[interval_from_root % 12]` for each pitch
- `self_diff`: sorted intervals between adjacent pitches
- `count_vec`: 12-element histogram of pitch-class intervals from bass note

**C++ ref:** `src/model/chordstatistics.cpp`

### Phase 7: BigramChordStatistics
**File:** `src/model/bigram_statistics.rs`

```rust
pub enum OverflowState { NoOverflow, Single, Total }
pub struct BigramChordStatistics {
    // ~27 fields: chroma_old, chroma, sv, vec, common_note, similarity,
    // root_movement, overflow_state, name, name_with_octave, ...
}
pub fn calculate_bigram_statistics(
    prev: &OrderedChord, next: &OrderedChord,
    prev_stats: &OrderedChordStatistics, next_stats: &OrderedChordStatistics,
) -> BigramChordStatistics
```

**C++ ref:** `src/model/bigramchordstatistics.cpp` (~300 lines)

### Phase 8: Config Enums & Structs
**File:** `src/model/config.rs`

5 enums with Default:
- `OutputMode { Both, MidiOnly, TextOnly }`
- `UniqueMode { Disabled, ChordType, Chroma }`
- `AlignMode { Disabled, Enabled, Strict }`
- `VLSetting { Disabled, Enabled, Strict }`
- `SubstituteObj { Postchord, Antechord, BothChords }`

~13 constraint structs (each with Default):
- `MonotonicityConfig`, `RangeConfig`, `AlignmentConfig`, `ExclusionConfig`
- `PedalConfig`, `CardinalityConfig`, `SingleChordConfig`, `ScaleConfig`
- `BassLibraryConfig`, `UniquenessConfig`, `VoiceLeadingConfig`
- `SimilarityConfig`, `SpanConfig`, `QIndicatorConfig`, `VecUniquenessConfig`

`ProgressionConfig` — aggregates all constraints + output settings
`SubstitutionConfig` with `ParamTolerance`

**C++ ref:** `src/include/model/config.h`, `src/include/model/config_enums.h`, `src/include/model/substitution_config.h`

### Phase 9: Combinatorics & MixedRadixRange
**File:** `src/utility/combinatorics.rs`, `src/utility/mixed_radix.rs`

```rust
pub fn comb(n: usize, k: usize) -> usize  // binomial coefficient

// Cached: maps (n_unique_pcs, target_size) -> list of expansion index vectors
pub static EXPANSION_INDEX_CACHE: Lazy<Mutex<HashMap<(usize,usize), Vec<Vec<usize>>>>>

pub struct MixedRadixRange {
    bases: Vec<usize>,       // e.g. [3, 4, 5]
    current: Vec<usize>,
    // skip "dead zones" where adjacent values are equal
}
impl Iterator for MixedRadixRange { type Item = Vec<usize>; }
```

**C++ ref:** `src/utility/combinatorics.cpp`, `src/include/utility/mixedradix.h`

### Phase 10: Expansion & Voice Leading
**Files:** `src/service/expansion.rs`, `src/service/voice_leading.rs`

```rust
pub fn expand(chord: &OrderedChord, target_size: usize) -> Vec<OrderedChord>
pub fn expand_single(chord: &OrderedChord, indices: &[usize]) -> OrderedChord

pub struct VoiceLeadingResult { pub vec: Vec<i32>, pub sv: i32 }
pub fn find_voice_leading(prev: &OrderedChord, next: &OrderedChord) -> VoiceLeadingResult
pub fn find_voice_leading_substitution(prev: &OrderedChord, next: &OrderedChord) -> VoiceLeadingResult
```

**C++ ref:** `src/service/expansion.cpp`, `src/service/voiceleading.cpp`

### Phase 11: ChordLibrary & MIDI Encoding
**Files:** `src/service/chord_library.rs`, `src/utility/midi_encoding.rs`

```rust
pub static CHORD_LIBRARY: Lazy<Mutex<ChordLibrary>>
pub struct ChordLibrary { cache: BTreeMap<OrderedChord, OrderedChordStatistics> }

pub fn to_vlq(value: u32) -> Vec<u8>   // Variable-length quantity
pub fn swap_int(value: u32) -> u32     // Byte-swap for MIDI
```

**C++ ref:** `src/service/chordlibrary.cpp`, `src/include/utility/midi_encoding.h`

### Phase 12: Validation Pipeline
**File:** `src/algorithm/validation.rs`

15 validator stages in order:
1. `monotonicity` — pitches must be non-decreasing
2. `range` — each pitch within register limits
3. `alignment` — chord alignment score within bounds
4. `exclusion` — excluded pitch classes not present
5. `pedal` — pedal note constraints
6. `cardinality` — note count within [min, max]
7. `single_chord_stats` — tension/thickness/span limits
8. `scale` — all pitches belong to configured scale
9. `bass_library` — bass note in library
10. `uniqueness` — not duplicate of recent chord
11. `voice_leading` — SV within bounds, VL setting
12. `similarity` — similarity score within bounds
13. `span` — CoF span within bounds
14. `q_indicator` — Q indicator within bounds
15. `vec_uniqueness` — voice-leading vector not repeated

```rust
pub struct ValidationContext<'a> {
    config: &'a ProgressionConfig,
    prev_chord: &'a OrderedChord,
    prev_stats: &'a OrderedChordStatistics,
    rec_ids: &'a mut Vec<OrderedChord>,   // recently seen chords
    vec_ids: &'a mut Vec<Vec<i32>>,       // recently seen VL vectors
}
pub struct ChordValidationPipeline { validators: Vec<fn(&mut ValidationContext, &OrderedChord) -> bool> }
impl ChordValidationPipeline { pub fn validate(&mut self, ctx: &mut ValidationContext, chord: &OrderedChord) -> bool }
```

**C++ ref:** `src/algorithm/validation.cpp` (~400 lines)

### Phase 13: Sorting & Progression
**Files:** `src/algorithm/sorting.rs`, `src/algorithm/progression.rs`

```rust
pub struct CandidateEntry { pub chord: OrderedChord, pub stats: BigramChordStatistics }
pub fn sort_candidates(candidates: &mut Vec<CandidateEntry>, sort_key: &str)

pub struct ProgressionResult { pub candidates: Vec<CandidateEntry> }
pub fn generate_single(
    prev_chord: &OrderedChord,
    config: &ProgressionConfig,
) -> ProgressionResult
// Pipeline: expansion → MixedRadixRange mutation → validation → bigram stats → sort
```

**C++ ref:** `src/algorithm/sorting.cpp`, `src/algorithm/progression.cpp`

### Phase 14: Substitution & Analysis
**File:** `src/algorithm/substitution.rs`, `src/algorithm/analysis.rs`

```rust
pub fn substitute(
    chord: &OrderedChord,
    config: &SubstitutionConfig,
    mode: SubstituteObj,
) -> Vec<CandidateEntry>
// Iterates 4095 non-empty subsets of the 12 pitch classes

pub struct AnalysisResult { pub ante: Vec<CandidateEntry>, pub post: Vec<CandidateEntry> }
pub fn analyse(chord: &OrderedChord, config: &SubstitutionConfig) -> AnalysisResult
```

**C++ ref:** `src/algorithm/substitution.cpp`, `src/algorithm/analysis.h`

### Phase 15: I/O Layer
**Files:** `src/io/note_parser.rs`, `src/io/formatter.rs`, `src/io/midi.rs`, `src/io/database.rs`

```rust
pub fn nametonum(name: &str) -> Option<u8>          // pitch name → MIDI number
pub fn parse_notes(input: &str) -> Option<OrderedChord>

pub fn format_candidates(candidates: &[CandidateEntry], config: &ProgressionConfig) -> String

pub struct MidiConfig { pub tempo: u32, pub output_path: String }
pub fn write_midi(chords: &[OrderedChord], config: &MidiConfig) -> std::io::Result<()>

pub fn read_chord_database(path: &str) -> BTreeMap<OrderedChord, OrderedChordStatistics>
pub fn read_alignment_database(path: &str) -> HashMap<Vec<i32>, i32>
```

**C++ ref:** `src/io/noteparser.cpp`, `src/io/formatter.cpp`, `src/include/io/midi.h`, `src/include/io/database.h`

---

## Key Rust Idiom Mappings

| C++ | Rust |
|-----|------|
| `class PitchIterable { virtual ... = 0; }` | `trait PitchIterable { ... }` |
| `std::optional<T>` / `-1` sentinel | `Option<T>` |
| `enum class OverflowState` | `enum OverflowState` |
| `static ChordLibrary& getInstance()` | `once_cell::sync::Lazy<Mutex<ChordLibrary>>` |
| `MixedRadixIterator` (C++ InputIterator) | `impl Iterator for MixedRadixIterator` |
| `std::function<bool(...)>` validators | `fn(...) -> bool` function pointers |
| `std::bitset<12>` | `u16` bitflags or `[bool; 12]` |
| Exceptions | `Result<T, E>` or panic for invariant violations |

---

## Verification Checkpoints

After each phase:
1. `cargo build` — compiles without warnings
2. `cargo test` — all tests pass
3. Diff is scoped to the phase

End-to-end target: After Phase 13, run the golden progression test (C major, 92 candidates) to verify pipeline parity with C++.

---

## C++ Source Reference

All C++ source lives at `/Users/osbertngok/git/ChordNova` on the `revamp-phase-2` branch.

Key files by module:
- `src/include/constant.h` — ET_SIZE=12, FIFTH_IN_SEMITONE=7
- `src/include/model/pitchclass.h` + `src/model/pitchclass.cpp`
- `src/include/model/pitch.h` + `src/model/pitch.cpp`
- `src/include/model/pitchiterable.h` + `src/model/pitchiterable.cpp`
- `src/include/model/orderedchord.h` + `src/model/orderedchord.cpp`
- `src/include/model/pitchset.h` + `src/model/pitchset.cpp`
- `src/include/model/chordstatistics.h` + `src/model/chordstatistics.cpp`
- `src/include/model/bigramchordstatistics.h` + `src/model/bigramchordstatistics.cpp`
- `src/include/model/config.h` + `src/include/model/config_enums.h`
- `src/include/model/substitution_config.h`
- `src/utility.cpp` + `src/include/utility.h` (general utils)
- `src/utility/combinatorics.cpp` + `src/include/utility/mixedradix.h`
- `src/service/expansion.cpp`, `src/service/voiceleading.cpp`
- `src/service/chordlibrary.cpp`
- `src/algorithm/validation.cpp`, `src/algorithm/sorting.cpp`
- `src/algorithm/progression.cpp`, `src/algorithm/substitution.cpp`
- `src/include/algorithm/analysis.h`
- `src/io/noteparser.cpp`, `src/io/formatter.cpp`
- `src/include/io/midi.h`, `src/include/io/database.h`
- `test/` — 264 test cases mirroring the above modules
