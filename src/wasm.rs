//! # ChordNova WASM API
//!
//! Three functions are exposed to JavaScript via `wasm-bindgen`. All use a
//! **JSON string** interface: inputs are JSON strings, outputs are JSON strings.
//! Parse results in JS with `JSON.parse(result)`.
//!
//! ## Chords
//!
//! Chords are represented as JSON arrays of MIDI note numbers (0-127).
//!
//! ```js
//! const cMajor = "[60,64,67]";  // C4 E4 G4
//! const fMajor = "[60,65,69]";  // C4 F4 A4
//! ```
//!
//! ## Errors
//!
//! On invalid input, all functions return `{"error": "...message..."}` instead
//! of a normal result. Always check for the `error` key before processing.
//!
//! ---
//!
//! ## `generate(initial_chord_json, config_json) -> String`
//!
//! Generate candidate next chords from an initial chord, subject to
//! voice-leading, range, harmonic, and other constraints.
//!
//! **Parameters:**
//! - `initial_chord_json`: `string` — JSON array of MIDI note numbers, e.g. `"[60,64,67]"`
//! - `config_json`: `string` — JSON `ProgressionConfig` (see below)
//!
//! **Returns:** JSON `ProgressionResult`:
//! ```json
//! {
//!   "candidates": [
//!     {
//!       "chord": [60, 65, 69],         // MIDI note numbers
//!       "stats": { ... }               // BigramChordStatistics (see below)
//!     }
//!   ],
//!   "total_evaluated": 729             // total mutation vectors tested
//! }
//! ```
//!
//! ---
//!
//! ## `analyse_chords(ante_json, post_json) -> String`
//!
//! Analyse the voice-leading and harmonic relationship between two chords.
//!
//! **Parameters:**
//! - `ante_json`: `string` — JSON array of MIDI notes for the antecedent (previous) chord
//! - `post_json`: `string` — JSON array of MIDI notes for the consequent (next) chord
//!
//! **Returns:** JSON `AnalysisResult`:
//! ```json
//! {
//!   "ante_stats": { ... },             // OrderedChordStatistics
//!   "post_stats": { ... },             // OrderedChordStatistics
//!   "vl_result": {
//!     "vec": [0, 1, 2],                // signed semitone movement per voice
//!     "sv": 3                          // total voice-leading distance
//!   },
//!   "bigram_stats": { ... }            // BigramChordStatistics
//! }
//! ```
//!
//! ---
//!
//! ## `substitute_chords(ante_json, post_json, config_json) -> String`
//!
//! Search for chord substitutions that preserve harmonic characteristics
//! within configurable tolerances.
//!
//! **Parameters:**
//! - `ante_json`: `string` — JSON array of MIDI notes for the antecedent chord
//! - `post_json`: `string` — JSON array of MIDI notes for the consequent chord
//! - `config_json`: `string` — JSON `SubstitutionConfig` (see below)
//!
//! **Returns:** JSON `SubstitutionResult`:
//! ```json
//! {
//!   "entries": [                       // single-chord substitutes (Postchord/Antechord mode)
//!     {
//!       "chord": [60, 65, 69],
//!       "stats": { ... },              // BigramChordStatistics
//!       "sim_orig": 85                 // similarity to original (0-100)
//!     }
//!   ],
//!   "pairs": [                         // paired substitutes (BothChords mode)
//!     {
//!       "ante": { "chord": [...], "stats": {...}, "sim_orig": 90 },
//!       "post": { "chord": [...], "stats": {...}, "sim_orig": 80 }
//!     }
//!   ],
//!   "total_evaluated": 4095
//! }
//! ```
//!
//! ---
//!
//! ## Type Reference
//!
//! ### `ProgressionConfig` (input to `generate`)
//!
//! All fields are required (no defaults are applied by the JSON deserialiser).
//!
//! ```json
//! {
//!   "voice_leading": {
//!     "vl_min": 0,                     // min voice-leading distance per voice
//!     "vl_max": 4,                     // max voice-leading distance per voice
//!     "vl_setting": "Default",         // "Default" | "Percentage" | "Number"
//!     "steady_min": 0.0,               // min % voices staying on same pitch
//!     "steady_max": 100.0,
//!     "ascending_min": 0.0,            // min % voices moving up
//!     "ascending_max": 100.0,
//!     "descending_min": 0.0,           // min % voices moving down
//!     "descending_max": 100.0
//!   },
//!   "range": {
//!     "lowest": 0,                     // lowest allowed MIDI note
//!     "highest": 127,                  // highest allowed MIDI note
//!     "m_min": 1,                      // min number of pitches
//!     "m_max": 15,                     // max number of pitches
//!     "n_min": 1,                      // min unique pitch classes
//!     "n_max": 12,                     // max unique pitch classes
//!     "h_min": 0.0,                    // min thickness
//!     "h_max": 50.0,                   // max thickness
//!     "r_min": 0,                      // min root pitch class (0-11)
//!     "r_max": 11,                     // max root pitch class
//!     "g_min": 0,                      // min geometrical center (0-100)
//!     "g_max": 100                     // max geometrical center
//!   },
//!   "harmonic": {
//!     "k_min": 0.0,                    // min chroma (harmonic distance)
//!     "k_max": 100.0,
//!     "kk_min": 0.0,                   // min chroma_old (CoF position)
//!     "kk_max": 100.0,
//!     "t_min": 0.0,                    // min tension
//!     "t_max": 100.0,
//!     "c_min": 0,                      // min common notes
//!     "c_max": 15,
//!     "sv_min": 0,                     // min total voice-leading distance
//!     "sv_max": 100,
//!     "s_min": 0,                      // min CoF span
//!     "s_max": 12,
//!     "ss_min": 0,                     // min CoF super-span
//!     "ss_max": 12,
//!     "q_min": -500.0,                 // min Q indicator
//!     "q_max": 500.0,
//!     "x_min": 0,                      // min similarity (0-100)
//!     "x_max": 100
//!   },
//!   "alignment": {
//!     "align_mode": "Unlimited",       // "Unlimited" | "Interval" | "List"
//!     "i_min": 0,                      // min interval between adjacent voices
//!     "i_max": 24,
//!     "i_low": 0,                      // min lowest interval
//!     "i_high": 24,
//!     "alignment_list": []             // whitelist of allowed alignments (List mode)
//!   },
//!   "exclusion": {
//!     "enabled": false,
//!     "exclusion_notes": [],           // MIDI pitch classes to exclude (0-11)
//!     "exclusion_roots": [],           // root pitch classes to exclude
//!     "exclusion_intervals": []        // IntervalConstraint objects
//!   },
//!   "pedal": {
//!     "enabled": false,
//!     "pedal_notes": [],               // MIDI notes that must be held
//!     "pedal_notes_set": [],
//!     "in_bass": false,
//!     "realign": false,
//!     "period": 1,
//!     "connect_pedal": false
//!   },
//!   "uniqueness": {
//!     "unique_mode": "Disabled"        // "Disabled" | "RemoveDup" | "RemoveDupType"
//!   },
//!   "scale": {
//!     "overall_scale": [0,1,2,3,4,5,6,7,8,9,10,11]  // allowed pitch classes
//!   },
//!   "similarity": {
//!     "enabled": false,
//!     "sim_period": [],
//!     "sim_min": [],
//!     "sim_max": []
//!   },
//!   "root_movement": {
//!     "enabled": false,
//!     "rm_priority": []                // priority per root movement 0-6; -1 = disabled
//!   },
//!   "bass": {
//!     "bass_avail": [1,3,5,7,9,11,13] // allowed bass scale degrees
//!   },
//!   "chord_library": {
//!     "chord_library": []              // pitch-class bitmask IDs to restrict to
//!   },
//!   "sort": {
//!     "sort_order": ""                 // sort key string (read right-to-left)
//!   },
//!   "continual": false,
//!   "loop_count": 1,
//!   "output_mode": "Both"              // "Both" | "MidiOnly" | "TextOnly"
//! }
//! ```
//!
//! **Sort key codes** (for `sort_order` and SubstitutionConfig `sort_order`):
//! `P`=sim_orig, `N`=pitch-class count, `T`=tension, `K`=chroma, `C`=common_note,
//! `S`=sv, `a`=span, `A`=sspan, `Q`=Q-indicator, `X`=similarity, `k`=chroma_old,
//! `R`=root movement. Append `+` for ascending (default is descending).
//!
//! ### `SubstitutionConfig` (input to `substitute_chords`)
//!
//! ```json
//! {
//!   "object": "Postchord",             // "Postchord" | "Antechord" | "BothChords"
//!   "test_all": false,                 // BothChords: enumerate all 4095^2 pairs
//!   "sample_size": 100000,             // BothChords random sample size
//!   "sort_order": "",                  // sort key string (same codes as above)
//!   "reset_list": "",                  // params using fixed reset values
//!   "percentage_list": "",             // params using percentage-based radius
//!   "sim_orig":   { "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "cardinality": { "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "tension":    { "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "chroma":     { "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "common_note":{ "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "span":       { "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "sspan":      { "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "sv":         { "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "q_indicator":{ "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "similarity": { "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "chroma_old": { "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "root":       { "center": 0, "radius": 0, "use_percentage": false, "min_sub": 0, "max_sub": 0 },
//!   "rm_priority": []                  // root movement priority (index 0-6), -1 = disabled
//! }
//! ```
//!
//! ### `BigramChordStatistics` (in output results)
//!
//! ```json
//! {
//!   "chroma_old": 0.0,                 // mean Circle of Fifths position (kk)
//!   "prev_chroma_old": 0.0,            // previous chord's chroma_old
//!   "chroma": 0.0,                     // harmonic distance on CoF (k)
//!   "q_indicator": 0.0,                // combined harmonic/VL indicator (Q)
//!   "common_note": 1,                  // exact MIDI pitches shared (c)
//!   "sv": 3,                           // total voice-leading distance
//!   "span": 4,                         // CoF span of this chord (s)
//!   "sspan": 5,                        // CoF span of union of both chords (ss)
//!   "similarity": 75,                  // voice-leading similarity 0-100 (x)
//!   "sim_orig": 100,                   // baseline similarity (p)
//!   "steady_count": 1,                 // voices staying on same pitch
//!   "ascending_count": 2,              // voices moving up
//!   "descending_count": 0,             // voices moving down
//!   "root_movement": 5,                // shortest chromatic distance between roots (0-6)
//!   "root_name": "F",                  // human-readable root name
//!   "hide_octave": false,
//!   "name": "C F A",                   // note names without octave
//!   "name_with_octave": "C4 F4 A4",   // note names with octave
//!   "overflow_state": "NoOverflow",    // "NoOverflow" | "Single" | "Total"
//!   "overflow_amount": 0,
//!   "notes": [60, 65, 69],             // MIDI note numbers (sorted)
//!   "pitch_class_set": [0, 5, 9],      // unique pitch classes (0-11)
//!   "single_chroma": [0, -1, -3],      // CoF position per note
//!   "vec": [0, 1, 2],                  // voice-leading vector (signed semitones)
//!   "self_diff": [5, 4],               // consecutive intervals of pitch-class normal form
//!   "count_vec": [0, 0, 1, 1, 1, 0],  // interval-class frequency vector (ic1-ic6)
//!   "alignment": [1, 5, 3]             // scale-degree of each note relative to root
//! }
//! ```
//!
//! ### `OrderedChordStatistics` (in `AnalysisResult`)
//!
//! ```json
//! {
//!   "num_of_pitches": 3,               // number of pitches (n)
//!   "num_of_unique_pitch_classes": 3,   // unique pitch objects (m)
//!   "tension": 4.0,                    // tension score (t)
//!   "thickness": 3.5,                  // thickness score (h)
//!   "root": 0,                         // root pitch class 0-11, or null
//!   "geometrical_center": 0.5,         // geometrical center ratio 0-1 (g)
//!   "alignment": [1, 3, 5],            // scale-degree position per note
//!   "self_diff": [4, 3],               // consecutive intervals of normal form
//!   "count_vec": [0, 0, 1, 1, 1, 0]   // interval-class frequency vector
//! }
//! ```
//!
//! ## Quick Start (JavaScript)
//!
//! ```js
//! import init, { generate, analyse_chords, substitute_chords } from './pkg/chordnovars.js';
//!
//! await init();
//!
//! // Analyse C major -> F major
//! const result = JSON.parse(analyse_chords("[60,64,67]", "[60,65,69]"));
//! console.log(result.bigram_stats.root_name);  // "F"
//! console.log(result.vl_result.sv);            // 3
//! ```

use wasm_bindgen::prelude::*;

use crate::algorithm::analysis::analyse;
use crate::algorithm::progression::generate_single;
use crate::algorithm::substitution::substitute;
use crate::model::config::ProgressionConfig;
use crate::model::ordered_chord::OrderedChord;
use crate::model::pitch::Pitch;
use crate::model::substitution_config::SubstitutionConfig;

/// Initialise panic hook for better error messages in the browser console.
/// Called automatically when the WASM module is loaded.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Parse a JSON array of MIDI note numbers into an `OrderedChord`.
fn parse_chord(json: &str) -> Result<OrderedChord, String> {
    let midi_notes: Vec<u8> =
        serde_json::from_str(json).map_err(|e| format!("invalid chord JSON: {e}"))?;
    Ok(OrderedChord::new(
        midi_notes.into_iter().map(Pitch::new).collect(),
    ))
}

/// Generate candidate next chords from an initial chord.
///
/// # Arguments
/// * `initial_chord_json` — JSON array of MIDI note numbers, e.g. `"[60,64,67]"`
/// * `config_json` — JSON-serialised `ProgressionConfig`
///
/// # Returns
/// JSON-serialised `ProgressionResult` containing candidates and total_evaluated.
/// On error, returns `{"error": "..."}`.
#[wasm_bindgen]
pub fn generate(initial_chord_json: &str, config_json: &str) -> String {
    let chord = match parse_chord(initial_chord_json) {
        Ok(c) => c,
        Err(e) => return error_json(&e),
    };
    let config: ProgressionConfig = match serde_json::from_str(config_json) {
        Ok(c) => c,
        Err(e) => return error_json(&format!("invalid config JSON: {e}")),
    };

    let result = generate_single(&chord, &config, &[], 0.0, &[], None);
    serde_json::to_string(&result).unwrap_or_else(|e| error_json(&format!("serialization error: {e}")))
}

/// Analyse the relationship between two consecutive chords.
///
/// # Arguments
/// * `ante_json` — JSON array of MIDI note numbers for the antecedent chord
/// * `post_json` — JSON array of MIDI note numbers for the consequent chord
///
/// # Returns
/// JSON-serialised `AnalysisResult` with chord statistics, voice-leading, and bigram stats.
/// On error, returns `{"error": "..."}`.
#[wasm_bindgen]
pub fn analyse_chords(ante_json: &str, post_json: &str) -> String {
    let ante = match parse_chord(ante_json) {
        Ok(c) => c,
        Err(e) => return error_json(&e),
    };
    let post = match parse_chord(post_json) {
        Ok(c) => c,
        Err(e) => return error_json(&e),
    };

    let result = analyse(&ante, &post, 0.0, &[]);
    serde_json::to_string(&result).unwrap_or_else(|e| error_json(&format!("serialization error: {e}")))
}

/// Search for chord substitutions.
///
/// # Arguments
/// * `ante_json` — JSON array of MIDI note numbers for the antecedent chord
/// * `post_json` — JSON array of MIDI note numbers for the consequent chord
/// * `config_json` — JSON-serialised `SubstitutionConfig`
///
/// # Returns
/// JSON-serialised `SubstitutionResult` with substitute entries/pairs.
/// On error, returns `{"error": "..."}`.
#[wasm_bindgen]
pub fn substitute_chords(ante_json: &str, post_json: &str, config_json: &str) -> String {
    let ante = match parse_chord(ante_json) {
        Ok(c) => c,
        Err(e) => return error_json(&e),
    };
    let post = match parse_chord(post_json) {
        Ok(c) => c,
        Err(e) => return error_json(&e),
    };
    let mut config: SubstitutionConfig = match serde_json::from_str(config_json) {
        Ok(c) => c,
        Err(e) => return error_json(&format!("invalid config JSON: {e}")),
    };

    let result = substitute(&ante, &post, &mut config, None);
    serde_json::to_string(&result).unwrap_or_else(|e| error_json(&format!("serialization error: {e}")))
}

/// Build a JSON error response.
fn error_json(message: &str) -> String {
    format!("{{\"error\":{}}}", serde_json::to_string(message).unwrap())
}
