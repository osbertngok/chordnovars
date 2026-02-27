use crate::model::ordered_chord::OrderedChord;
use crate::model::pitch::Pitch;
use crate::model::pitch_iterable::PitchIterable;

/// Convert a note name token or MIDI number string to a MIDI note number.
///
/// Accepted formats:
/// - Pure integer: `"60"` → 60
/// - `Letter[accidental][octave]`: `"C4"`, `"Bb3"`, `"G#5"`
/// - `[accidental]Letter[octave]`: `"bB4"`, `"#C3"`
///
/// Default octave is 4 when no octave digit is present.
///
/// Port of `nametonum()` from `ChordNova/src/io/noteparser.cpp`.
pub fn nametonum(token: &str) -> Option<u8> {
    if token.is_empty() { return None; }

    let bytes = token.as_bytes();

    // Pure MIDI number
    if bytes[0].is_ascii_digit() {
        let val: i32 = token.parse().ok()?;
        if val >= 0 && val <= 127 { return Some(val as u8); }
        return None;
    }

    // Path 1: Letter[accidental][octave]
    let ch = (bytes[0] as char).to_ascii_uppercase();
    let base_opt = match ch {
        'C' => Some(12i32),
        'D' => Some(14),
        'E' => Some(16),
        'F' => Some(17),
        'G' => Some(19),
        'A' => Some(21),
        'B' => Some(23),
        _ => None,
    };

    if let Some(mut val) = base_opt {
        let mut pos = 1usize;
        if pos < bytes.len() {
            if bytes[pos] == b'#' { val += 1; pos += 1; }
            else if bytes[pos] == b'b' { val -= 1; pos += 1; }
        }
        let (octave, has_octave) = parse_octave(bytes, pos);
        if has_octave && pos + (if has_octave { 1 } else { 0 }) <= bytes.len() {
            let result = val + 12 * octave;
            if result >= 0 && result <= 127 { return Some(result as u8); }
            return None;
        } else if !has_octave && pos == bytes.len() {
            // No octave specified: default octave = 4
            let result = val + 12 * 4;
            if result >= 0 && result <= 127 { return Some(result as u8); }
            return None;
        }
    }

    // Path 2: [accidental]Letter[octave]
    let mut pos = 0usize;
    let mut val = 0i32;
    if bytes[0] == b'#' { val += 1; pos += 1; }
    else if bytes[0] == b'b' { val -= 1; pos += 1; }
    else { return None; }

    if pos >= bytes.len() { return None; }
    let ch2 = (bytes[pos] as char).to_ascii_uppercase();
    let base2 = match ch2 {
        'C' => Some(12i32),
        'D' => Some(14),
        'E' => Some(16),
        'F' => Some(17),
        'G' => Some(19),
        'A' => Some(21),
        'B' => Some(23),
        _ => None,
    }?;
    val += base2;
    pos += 1;

    let (octave, _) = parse_octave(bytes, pos);
    let result = val + 12 * octave;
    if result >= 0 && result <= 127 { Some(result as u8) } else { None }
}

/// Parse an optional octave suffix from `bytes[pos..]`.
/// Returns `(octave, has_octave)`. Default octave is 4.
fn parse_octave(bytes: &[u8], pos: usize) -> (i32, bool) {
    if pos >= bytes.len() { return (4, false); }
    if bytes[pos] == b'-' && pos + 1 < bytes.len() && bytes[pos + 1].is_ascii_digit() {
        let octave = -((bytes[pos + 1] - b'0') as i32);
        return (octave, true);
    }
    if bytes[pos].is_ascii_digit() {
        return ((bytes[pos] - b'0') as i32, true);
    }
    (4, false)
}

/// Parse a space-separated list of note tokens into an `OrderedChord`.
///
/// When no octave digits are given, auto-assigns octaves to keep the chord
/// ascending and centred in the valid MIDI range.
///
/// Port of `parse_notes()` from `ChordNova/src/io/noteparser.cpp`.
pub fn parse_notes(input: &str) -> Option<OrderedChord> {
    if input.is_empty() || input.len() >= 500 { return None; }

    let tokens: Vec<&str> = input.split_whitespace().collect();
    if tokens.is_empty() { return None; }

    let mut midi_notes: Vec<i32> = Vec::new();
    let mut no_octave = true;
    for tok in &tokens {
        let note = nametonum(tok)?;
        midi_notes.push(note as i32);

        let tb = tok.as_bytes();
        // If ends with a digit or starts with a digit → has octave info
        if !tb.is_empty() && (tb.last().unwrap().is_ascii_digit() || tb[0].is_ascii_digit()) {
            no_octave = false;
        }
    }

    if no_octave {
        // Auto-assign octaves: fold down to ascending, then centre
        let sz = midi_notes.len();
        for i in (1..sz).rev() {
            if midi_notes[i - 1] > midi_notes[i] {
                let diff = midi_notes[i - 1] - midi_notes[i];
                let octave_diff = diff / 12;
                midi_notes[i - 1] -= (octave_diff + 1) * 12;
            }
        }
        let octave_h = (127 - midi_notes[sz - 1]) / 12;
        let octave_l = (midi_notes[0] as f64 / 12.0).floor() as i32;
        if octave_h + octave_l < 0 { return None; }
        let octave_shift = (octave_h - octave_l) / 2;
        for n in &mut midi_notes { *n += octave_shift * 12; }
    } else {
        midi_notes.sort_unstable();
        midi_notes.dedup();
    }

    let pitches: Vec<Pitch> = midi_notes.iter()
        .map(|&m| {
            if m < 0 || m > 127 { return Err(()); }
            Ok(Pitch::new(m as u8))
        })
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    Some(OrderedChord::new(pitches))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nametonum_midi_number() {
        assert_eq!(nametonum("60"), Some(60));
        assert_eq!(nametonum("0"), Some(0));
        assert_eq!(nametonum("127"), Some(127));
        assert_eq!(nametonum("128"), None);
    }

    #[test]
    fn nametonum_letter_octave() {
        assert_eq!(nametonum("C4"), Some(60));
        assert_eq!(nametonum("A4"), Some(69));
        assert_eq!(nametonum("G#4"), Some(68));
        assert_eq!(nametonum("Bb3"), Some(58));
    }

    #[test]
    fn nametonum_accidental_first() {
        assert_eq!(nametonum("#C4"), Some(61));
    }

    #[test]
    fn nametonum_default_octave() {
        // "C" without octave → C4 = 60
        assert_eq!(nametonum("C"), Some(60));
    }

    #[test]
    fn parse_notes_with_octave() {
        let chord = parse_notes("C4 E4 G4").unwrap();
        let pitches = chord.get_pitches();
        assert_eq!(pitches.len(), 3);
        assert_eq!(pitches[0].get_number(), 60);
        assert_eq!(pitches[1].get_number(), 64);
        assert_eq!(pitches[2].get_number(), 67);
    }

    #[test]
    fn parse_notes_midi_numbers() {
        let chord = parse_notes("60 64 67").unwrap();
        let pitches = chord.get_pitches();
        assert_eq!(pitches.len(), 3);
        assert_eq!(pitches[0].get_number(), 60);
    }

    #[test]
    fn parse_notes_invalid_returns_none() {
        assert!(parse_notes("").is_none());
        assert!(parse_notes("XY").is_none());
    }
}
