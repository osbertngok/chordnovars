use std::io::{self, Seek, SeekFrom, Write};

use crate::model::ordered_chord::OrderedChord;
use crate::model::pitch_iterable::PitchIterable;
use crate::utility::midi_encoding::{swap_int, to_vlq};

/// Configuration for MIDI output.
///
/// Port of `MidiConfig` from `ChordNova/src/include/io/midi.h`.
#[derive(Debug, Clone)]
pub struct MidiConfig {
    /// Ticks per quarter note.
    pub ticks_per_quarter: i32,
    /// Beats per minute.
    pub tempo_bpm: i32,
    /// Note On velocity (0–127).
    pub note_on_velocity: u8,
    /// Note Off velocity (0–127).
    pub note_off_velocity: u8,
    /// Duration per chord in beats.
    pub beat_duration: i32,
    /// Interleave original chord with each candidate.
    pub interlace: bool,
}

impl Default for MidiConfig {
    fn default() -> Self {
        MidiConfig {
            ticks_per_quarter: 480,
            tempo_bpm: 60,
            note_on_velocity: 80,
            note_off_velocity: 64,
            beat_duration: 1,
            interlace: false,
        }
    }
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn write_be32<W: Write>(out: &mut W, value: i32) -> io::Result<()> {
    let be = swap_int(value, 4);
    out.write_all(&be.to_ne_bytes())
}

fn write_be16<W: Write>(out: &mut W, value: i32) -> io::Result<()> {
    let be = swap_int(value, 2) as i16;
    out.write_all(&be.to_ne_bytes())
}

fn write_header<W: Write>(out: &mut W, format: i32, num_tracks: i32, ticks: i32) -> io::Result<()> {
    out.write_all(b"MThd")?;
    write_be32(out, 6)?;
    write_be16(out, format)?;
    write_be16(out, num_tracks)?;
    write_be16(out, ticks)
}

fn write_track_preamble<W: Write>(out: &mut W, tempo_bpm: i32) -> io::Result<()> {
    // Copyright text
    let copyright = b"(c) 2020 Wenge Chen, Ji-woon Sim.";
    out.write_all(b"\x00\xff\x02")?;
    out.write_all(&[copyright.len() as u8])?;
    out.write_all(copyright)?;

    // Instrument name: "Piano"
    out.write_all(b"\x00\xff\x04\x05Piano")?;

    // Tempo
    let us_per_beat = 60_000_000 / tempo_bpm;
    out.write_all(b"\x00\xff\x51\x03")?;
    out.write_all(&[
        ((us_per_beat >> 16) & 0xFF) as u8,
        ((us_per_beat >> 8) & 0xFF) as u8,
        (us_per_beat & 0xFF) as u8,
    ])?;

    // Time signature: 4/4
    out.write_all(b"\x00\xff\x58\x04\x04\x02\x18\x08")?;
    // Key signature: C major
    out.write_all(b"\x00\xff\x59\x02\x00\x00")?;
    // Program change: piano
    out.write_all(b"\x00\xc0\x00")?;

    Ok(())
}

fn write_chord_events<W: Write>(
    out: &mut W,
    chord: &OrderedChord,
    beat: i32,
    ticks_per_quarter: i32,
    note_on_vel: u8,
    note_off_vel: u8,
) -> io::Result<()> {
    let pitches = chord.get_pitches();
    // Note On (all at delta=0)
    for p in &pitches {
        out.write_all(&[0x00, 0x90, p.get_number(), note_on_vel])?;
    }
    // Note Off
    let ticks = (beat * ticks_per_quarter) as u32;
    for (i, p) in pitches.iter().enumerate() {
        if i == 0 {
            let vlq = to_vlq(ticks);
            out.write_all(&vlq)?;
        } else {
            out.write_all(&[0x00])?;
        }
        out.write_all(&[0x80, p.get_number(), note_off_vel])?;
    }
    Ok(())
}

fn write_end_of_track<W: Write>(out: &mut W) -> io::Result<()> {
    out.write_all(b"\x00\xff\x2f\x00")
}

/// Write a sequence of chords as a Standard MIDI File (Format 0).
///
/// Port of `write_midi()` from `ChordNova/src/io/midi.cpp`.
pub fn write_midi(
    path: &str,
    chords: &[OrderedChord],
    config: &MidiConfig,
) -> bool {
    write_midi_impl(path, config, |out| {
        for chord in chords {
            write_chord_events(out, chord, config.beat_duration,
                               config.ticks_per_quarter,
                               config.note_on_velocity, config.note_off_velocity)?;
        }
        Ok(())
    })
}

/// Write single-mode results: initial chord followed by all candidates.
///
/// If `interlace` is true, writes: initial, cand1, initial, cand2, …
///
/// Port of `write_midi_single()` from `ChordNova/src/io/midi.cpp`.
pub fn write_midi_single(
    path: &str,
    initial: &OrderedChord,
    candidates: &[OrderedChord],
    config: &MidiConfig,
) -> bool {
    write_midi_impl(path, config, |out| {
        if config.interlace {
            for cand in candidates {
                write_chord_events(out, initial, config.beat_duration,
                                   config.ticks_per_quarter,
                                   config.note_on_velocity, config.note_off_velocity)?;
                write_chord_events(out, cand, config.beat_duration,
                                   config.ticks_per_quarter,
                                   config.note_on_velocity, config.note_off_velocity)?;
            }
        } else {
            write_chord_events(out, initial, config.beat_duration,
                               config.ticks_per_quarter,
                               config.note_on_velocity, config.note_off_velocity)?;
            for cand in candidates {
                write_chord_events(out, cand, config.beat_duration,
                                   config.ticks_per_quarter,
                                   config.note_on_velocity, config.note_off_velocity)?;
            }
        }
        Ok(())
    })
}

/// Generic MIDI writer: creates file, writes header+preamble, calls body fn, fixes track length.
fn write_midi_impl<F>(path: &str, config: &MidiConfig, body: F) -> bool
where
    F: FnOnce(&mut std::fs::File) -> io::Result<()>,
{
    let mut file = match std::fs::File::create(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let result: io::Result<()> = (|| {
        write_header(&mut file, 0, 1, config.ticks_per_quarter)?;

        // Write track header with placeholder length
        file.write_all(b"MTrk")?;
        let len_pos = file.stream_position()?;
        write_be32(&mut file, 0)?; // placeholder

        write_track_preamble(&mut file, config.tempo_bpm)?;
        body(&mut file)?;
        write_end_of_track(&mut file)?;

        // Fix up track length
        let end_pos = file.stream_position()?;
        let track_len = (end_pos - len_pos - 4) as i32;
        file.seek(SeekFrom::Start(len_pos))?;
        write_be32(&mut file, track_len)?;

        Ok(())
    })();

    result.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::pitch::Pitch;
    use std::fs;

    fn chord(notes: &[u8]) -> OrderedChord {
        OrderedChord::new(notes.iter().copied().map(Pitch::new).collect())
    }

    #[test]
    fn write_midi_creates_valid_file() {
        let path = "/tmp/test_chordnovars_midi.mid";
        let chords = vec![chord(&[60, 64, 67]), chord(&[65, 69, 72])];
        let config = MidiConfig::default();
        assert!(write_midi(path, &chords, &config));

        let data = fs::read(path).unwrap();
        // Check MIDI header magic
        assert_eq!(&data[0..4], b"MThd");
        // Check track magic
        let header_len = 14; // MThd(4) + len(4) + format(2) + ntracks(2) + ticks(2)
        assert_eq!(&data[header_len..header_len + 4], b"MTrk");

        fs::remove_file(path).ok();
    }

    #[test]
    fn write_midi_fails_on_invalid_path() {
        let chords = vec![chord(&[60])];
        let config = MidiConfig::default();
        assert!(!write_midi("/nonexistent/dir/out.mid", &chords, &config));
    }
}
