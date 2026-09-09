use domain::confidence::WORLD_CONFIDENCE_MIN;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("schema: {0}")]
    Schema(String),
    #[error("world confidence {actual} below minimum {min}")]
    WorldConfidence { actual: f32, min: f32 },
    #[error("duplicate: {0}")]
    Duplicate(String),
    #[error("relation: {0}")]
    Relation(String),
}

pub fn validate_world_confidence(world_confidence: f32) -> Result<(), ValidationError> {
    if !(0.0..=1.0).contains(&world_confidence) {
        return Err(ValidationError::Schema("world_confidence must be within [0,1]".into()));
    }
    if world_confidence < WORLD_CONFIDENCE_MIN {
        return Err(ValidationError::WorldConfidence {
            actual: world_confidence,
            min: WORLD_CONFIDENCE_MIN,
        });
    }
    Ok(())
}

pub fn validate_canonical_statement(statement: &str) -> Result<(), ValidationError> {
    let t = statement.trim();
    if t.is_empty() {
        return Err(ValidationError::Schema("canonical_statement is non-empty".into()));
    }
    if t.len() < 10 {
        return Err(ValidationError::Schema(
            "canonical_statement too short to be atomic truth-bearing".into(),
        ));
    }
    Ok(())
}

pub fn validate_canonical_name(name: &str) -> Result<(), ValidationError> {
    if name.trim().is_empty() {
        return Err(ValidationError::Schema("canonical_name is non-empty".into()));
    }
    Ok(())
}
