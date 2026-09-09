/// Constants per `data_models.md -> Constants`.
pub const WORLD_CONFIDENCE_MIN: f32 = 0.80;
pub const HEALTHY_THRESHOLD: f32 = 0.95;
pub const DEFAULT_DECAY_GRACE_YEARS: f32 = 2.0;
pub const DEFAULT_DECAY_HALF_LIFE_YEARS: f32 = 12.0;
pub const DEFAULT_MAX_GRAPH_DEPTH: u8 = 3;
pub const DEFAULT_MAX_CONTEXT_CONCEPTS: usize = 64;
pub const DEFAULT_MAX_RELATED_CONCEPTS: usize = 16;
pub const DEFAULT_MAX_TOOL_RESULT_CONCEPTS: usize = 32;

/// Validate a confidence value is within [0,1].
pub fn validate_confidence(value: f32) -> Result<(), crate::errors::DomainError> {
    if !(0.0..=1.0).contains(&value) {
        return Err(crate::errors::DomainError::Validation(format!(
            "confidence {value} out of range [0,1]"
        )));
    }
    Ok(())
}

/// Validate a delta is within [-1,1].
pub fn validate_delta(delta: f32) -> Result<(), crate::errors::DomainError> {
    if !(-1.0..=1.0).contains(&delta) {
        return Err(crate::errors::DomainError::Validation(format!(
            "delta {delta} out of range [-1,1]"
        )));
    }
    Ok(())
}
