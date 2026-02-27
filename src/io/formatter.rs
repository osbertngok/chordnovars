use std::io::Write;

use crate::algorithm::sorting::CandidateEntry;

/// Format progression results to a writer.
///
/// Each line: `[N. ]name_with_octave  (sv=X k=Y.Y s=Z x=W Q=V.V)[ root=R]`
///
/// Port of `format_candidates()` from `ChordNova/src/io/formatter.cpp`.
///
/// # Arguments
/// * `out`          – Destination writer.
/// * `candidates`   – Sorted candidate entries.
/// * `start_index`  – 1-based line prefix (0 = no numbering).
pub fn format_candidates<W: Write>(
    out: &mut W,
    candidates: &[CandidateEntry],
    start_index: i32,
) -> std::io::Result<()> {
    for (i, entry) in candidates.iter().enumerate() {
        let s = &entry.stats;

        if start_index > 0 {
            write!(out, "{}. ", start_index + i as i32)?;
        }

        write!(out, "{}", s.name_with_octave)?;

        write!(
            out,
            "  (sv={} k={:.1} s={} x={} Q={:.1})",
            s.sv, s.chroma, s.span, s.similarity, s.q_indicator
        )?;

        if !s.root_name.is_empty() {
            write!(out, " root={}", s.root_name)?;
        }

        writeln!(out)?;
    }
    Ok(())
}

/// Format to a `String`.
pub fn format_candidates_to_string(candidates: &[CandidateEntry], start_index: i32) -> String {
    let mut buf: Vec<u8> = Vec::new();
    format_candidates(&mut buf, candidates, start_index).unwrap();
    String::from_utf8(buf).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::sorting::CandidateEntry;
    use crate::model::bigram_statistics::BigramChordStatistics;
    use crate::model::ordered_chord::OrderedChord;
    use crate::model::pitch::Pitch;

    fn make_entry(midi: &[u8]) -> CandidateEntry {
        let chord = OrderedChord::new(midi.iter().copied().map(Pitch::new).collect());
        let mut stats = BigramChordStatistics::default();
        stats.name_with_octave = "C4 E4 G4".to_string();
        stats.sv = 3;
        stats.chroma = 1.5;
        stats.span = 4;
        stats.similarity = 80;
        stats.q_indicator = 2.3;
        stats.root_name = "C".to_string();
        CandidateEntry { chord, stats }
    }

    #[test]
    fn format_numbered() {
        let entries = vec![make_entry(&[60, 64, 67])];
        let s = format_candidates_to_string(&entries, 1);
        assert!(s.starts_with("1. C4 E4 G4"));
        assert!(s.contains("sv=3"));
        assert!(s.contains("root=C"));
    }

    #[test]
    fn format_no_numbering() {
        let entries = vec![make_entry(&[60, 64, 67])];
        let s = format_candidates_to_string(&entries, 0);
        assert!(s.starts_with("C4 E4 G4"));
        assert!(!s.starts_with("1."));
    }
}
