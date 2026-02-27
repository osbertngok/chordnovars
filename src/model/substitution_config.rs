use serde::{Serialize, Deserialize};

use crate::model::config::SubstituteObj;

/// Per-parameter substitution tolerance: center value, radius, and computed min/max.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParamTolerance {
    /// Reference value (computed or reset).
    pub center: f64,
    /// Tolerance radius (absolute or percentage).
    pub radius: f64,
    /// Whether radius is percentage-based.
    pub use_percentage: bool,
    /// Computed lower bound.
    pub min_sub: f64,
    /// Computed upper bound.
    pub max_sub: f64,
}

/// Configuration for chord substitution search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstitutionConfig {
    /// Which chord(s) to substitute.
    pub object: SubstituteObj,
    /// BothChords: enumerate all 4095^2 pairs.
    pub test_all: bool,
    /// BothChords random sample size.
    pub sample_size: i32,
    /// Sort/filter keys (right-to-left priority).
    pub sort_order: String,
    /// Parameters using fixed reset values (not computed from reference).
    pub reset_list: String,
    /// Parameters using percentage-based radius.
    pub percentage_list: String,

    // Per-parameter tolerances (letter code matches legacy fields)
    pub sim_orig: ParamTolerance,    // P
    pub cardinality: ParamTolerance, // N
    pub tension: ParamTolerance,     // T
    pub chroma: ParamTolerance,      // K
    pub common_note: ParamTolerance, // C
    pub span: ParamTolerance,        // a
    pub sspan: ParamTolerance,       // A
    pub sv: ParamTolerance,          // S
    pub q_indicator: ParamTolerance, // Q
    pub similarity: ParamTolerance,  // X
    pub chroma_old: ParamTolerance,  // k
    pub root: ParamTolerance,        // R

    /// Root movement priority (index 0–6). -1 = disabled.
    pub rm_priority: Vec<i32>,
}

impl Default for SubstitutionConfig {
    fn default() -> Self {
        SubstitutionConfig {
            object: SubstituteObj::Postchord,
            test_all: false,
            sample_size: 100000,
            sort_order: String::new(),
            reset_list: String::new(),
            percentage_list: String::new(),
            sim_orig: ParamTolerance::default(),
            cardinality: ParamTolerance::default(),
            tension: ParamTolerance::default(),
            chroma: ParamTolerance::default(),
            common_note: ParamTolerance::default(),
            span: ParamTolerance::default(),
            sspan: ParamTolerance::default(),
            sv: ParamTolerance::default(),
            q_indicator: ParamTolerance::default(),
            similarity: ParamTolerance::default(),
            chroma_old: ParamTolerance::default(),
            root: ParamTolerance::default(),
            rm_priority: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitution_config_defaults() {
        let cfg = SubstitutionConfig::default();
        assert_eq!(cfg.object, SubstituteObj::Postchord);
        assert!(!cfg.test_all);
        assert_eq!(cfg.sample_size, 100000);
        assert_eq!(cfg.tension.center, 0.0);
    }
}
