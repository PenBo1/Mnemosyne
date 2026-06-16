//! Cadence policy for chapter pacing.

/// Cadence policy constants
pub struct CadencePolicy;

impl CadencePolicy {
    /// Minimum chapters between similar chapter types
    pub const MIN_GAP_BETWEEN_TYPES: u32 = 2;

    /// Maximum consecutive chapters of same type
    pub const MAX_CONSECUTIVE_SAME_TYPE: u32 = 3;

    /// Required chapter type distribution per volume
    pub fn required_distribution() -> Vec<(&'static str, f64)> {
        vec![
            ("conflict", 0.3),
            ("development", 0.3),
            ("transition", 0.2),
            ("climax", 0.1),
            ("resolution", 0.1),
        ]
    }
}
