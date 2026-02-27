use serde::{Serialize, Deserialize};

use crate::constant::ET_SIZE;

// ── Enums ─────────────────────────────────────────────────────────────────────

/// Output format mode.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OutputMode {
    /// Output both text and MIDI files.
    #[default]
    Both,
    /// Output MIDI file only.
    MidiOnly,
    /// Output text file only.
    TextOnly,
}

/// Deduplication mode for generated chords.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UniqueMode {
    /// No deduplication.
    #[default]
    Disabled,
    /// Remove exact pitch duplicates.
    RemoveDup,
    /// Remove chords with identical pitch-class set type.
    RemoveDupType,
}

/// Voice alignment validation mode.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AlignMode {
    /// Validate by interval range constraints.
    Interval,
    /// Validate against a whitelist of allowed alignments.
    List,
    /// No alignment constraints.
    #[default]
    Unlimited,
}

/// Voice-leading direction constraint mode.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VLSetting {
    /// Direction counts as percentage of total voices.
    Percentage,
    /// Direction counts as absolute numbers.
    Number,
    /// Default: reject parallel motion only.
    #[default]
    Default,
}

/// Which chord(s) to substitute in chord substitution mode.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SubstituteObj {
    /// Substitute the post-chord only.
    #[default]
    Postchord,
    /// Substitute the ante-chord only.
    Antechord,
    /// Substitute both chords.
    BothChords,
}

// ── Constraint structs ────────────────────────────────────────────────────────

/// Interval constraint data for exclusion rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalConstraint {
    pub interval: i32,
    pub octave_min: i32,
    pub octave_max: i32,
    pub num_min: i32,
    pub num_max: i32,
}

/// Voice-leading movement constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceLeadingConstraints {
    pub vl_min: i32,
    pub vl_max: i32,
    pub vl_setting: VLSetting,
    pub steady_min: f64,
    pub steady_max: f64,
    pub ascending_min: f64,
    pub ascending_max: f64,
    pub descending_min: f64,
    pub descending_max: f64,
}

impl Default for VoiceLeadingConstraints {
    fn default() -> Self {
        VoiceLeadingConstraints {
            vl_min: 0,
            vl_max: 4,
            vl_setting: VLSetting::Default,
            steady_min: 0.0,
            steady_max: 100.0,
            ascending_min: 0.0,
            ascending_max: 100.0,
            descending_min: 0.0,
            descending_max: 100.0,
        }
    }
}

/// Pitch range constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeConstraints {
    pub lowest: i32,
    pub highest: i32,
    pub m_min: i32,
    pub m_max: i32,
    pub n_min: i32,
    pub n_max: i32,
    pub h_min: f64,
    pub h_max: f64,
    pub r_min: i32,
    pub r_max: i32,
    pub g_min: i32,
    pub g_max: i32,
}

impl Default for RangeConstraints {
    fn default() -> Self {
        RangeConstraints {
            lowest: 0,
            highest: 127,
            m_min: 1,
            m_max: 15,
            n_min: 1,
            n_max: 12,
            h_min: 0.0,
            h_max: 50.0,
            r_min: 0,
            r_max: 11,
            g_min: 0,
            g_max: 100,
        }
    }
}

/// Harmonic/bigram statistic range constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarmonicConstraints {
    pub k_min: f64,
    pub k_max: f64,
    pub kk_min: f64,
    pub kk_max: f64,
    pub t_min: f64,
    pub t_max: f64,
    pub c_min: i32,
    pub c_max: i32,
    pub sv_min: i32,
    pub sv_max: i32,
    pub s_min: i32,
    pub s_max: i32,
    pub ss_min: i32,
    pub ss_max: i32,
    pub q_min: f64,
    pub q_max: f64,
    pub x_min: i32,
    pub x_max: i32,
}

impl Default for HarmonicConstraints {
    fn default() -> Self {
        HarmonicConstraints {
            k_min: 0.0,
            k_max: 100.0,
            kk_min: 0.0,
            kk_max: 100.0,
            t_min: 0.0,
            t_max: 100.0,
            c_min: 0,
            c_max: 15,
            sv_min: 0,
            sv_max: 100,
            s_min: 0,
            s_max: 12,
            ss_min: 0,
            ss_max: 12,
            q_min: -500.0,
            q_max: 500.0,
            x_min: 0,
            x_max: 100,
        }
    }
}

/// Alignment/interval spacing constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentConfig {
    pub align_mode: AlignMode,
    pub i_min: i32,
    pub i_max: i32,
    pub i_low: i32,
    pub i_high: i32,
    pub alignment_list: Vec<Vec<i32>>,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        AlignmentConfig {
            align_mode: AlignMode::Unlimited,
            i_min: 0,
            i_max: 24,
            i_low: 0,
            i_high: 24,
            alignment_list: vec![],
        }
    }
}

/// Note/root/interval exclusion rules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExclusionConfig {
    pub enabled: bool,
    pub exclusion_notes: Vec<i32>,
    pub exclusion_roots: Vec<i32>,
    pub exclusion_intervals: Vec<IntervalConstraint>,
}

/// Pedal note constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PedalConfig {
    pub enabled: bool,
    pub pedal_notes: Vec<i32>,
    pub pedal_notes_set: Vec<i32>,
    pub in_bass: bool,
    pub realign: bool,
    pub period: i32,
    pub connect_pedal: bool,
}

impl Default for PedalConfig {
    fn default() -> Self {
        PedalConfig {
            enabled: false,
            pedal_notes: vec![],
            pedal_notes_set: vec![],
            in_bass: false,
            realign: false,
            period: 1,
            connect_pedal: false,
        }
    }
}

/// Deduplication settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UniquenessConfig {
    pub unique_mode: UniqueMode,
}

/// Overall scale constraint (pitch class filter).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleConfig {
    pub overall_scale: Vec<i32>,
}

impl Default for ScaleConfig {
    fn default() -> Self {
        ScaleConfig {
            overall_scale: (0..ET_SIZE as i32).collect(),
        }
    }
}

/// Extended similarity checking configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimilarityConfig {
    pub enabled: bool,
    pub sim_period: Vec<i32>,
    pub sim_min: Vec<i32>,
    pub sim_max: Vec<i32>,
}

/// Root movement constraint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RootMovementConfig {
    pub enabled: bool,
    /// Priority for each root movement 0–6. -1 = disabled.
    pub rm_priority: Vec<i32>,
}

/// Bass note availability constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BassConfig {
    pub bass_avail: Vec<i32>,
}

impl Default for BassConfig {
    fn default() -> Self {
        BassConfig {
            bass_avail: vec![1, 3, 5, 7, 9, 11, 13],
        }
    }
}

/// Chord library (database) configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChordLibraryConfig {
    pub chord_library: Vec<i32>,
}

/// Sort order configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SortConfig {
    /// Sort string read right-to-left; '+' suffix = ascending.
    pub sort_order: String,
}

/// Aggregated progression generation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressionConfig {
    pub voice_leading: VoiceLeadingConstraints,
    pub range: RangeConstraints,
    pub harmonic: HarmonicConstraints,
    pub alignment: AlignmentConfig,
    pub exclusion: ExclusionConfig,
    pub pedal: PedalConfig,
    pub uniqueness: UniquenessConfig,
    pub scale: ScaleConfig,
    pub similarity: SimilarityConfig,
    pub root_movement: RootMovementConfig,
    pub bass: BassConfig,
    pub chord_library: ChordLibraryConfig,
    pub sort: SortConfig,
    pub continual: bool,
    pub loop_count: i32,
    pub output_mode: OutputMode,
}

impl Default for ProgressionConfig {
    fn default() -> Self {
        ProgressionConfig {
            voice_leading: VoiceLeadingConstraints::default(),
            range: RangeConstraints::default(),
            harmonic: HarmonicConstraints::default(),
            alignment: AlignmentConfig::default(),
            exclusion: ExclusionConfig::default(),
            pedal: PedalConfig::default(),
            uniqueness: UniquenessConfig::default(),
            scale: ScaleConfig::default(),
            similarity: SimilarityConfig::default(),
            root_movement: RootMovementConfig::default(),
            bass: BassConfig::default(),
            chord_library: ChordLibraryConfig::default(),
            sort: SortConfig::default(),
            continual: false,
            loop_count: 1,
            output_mode: OutputMode::Both,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progression_config_defaults() {
        let cfg = ProgressionConfig::default();
        assert_eq!(cfg.voice_leading.vl_max, 4);
        assert_eq!(cfg.range.lowest, 0);
        assert_eq!(cfg.range.highest, 127);
        assert_eq!(cfg.harmonic.sv_max, 100);
        assert_eq!(cfg.scale.overall_scale.len(), 12);
        assert_eq!(cfg.bass.bass_avail, vec![1, 3, 5, 7, 9, 11, 13]);
        assert_eq!(cfg.output_mode, OutputMode::Both);
        assert_eq!(cfg.loop_count, 1);
    }
}
