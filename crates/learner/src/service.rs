use domain::LearnerConceptState;

pub fn is_review_eligible(state: &LearnerConceptState) -> bool {
    state.is_review_eligible()
}

pub fn apply_delta(
    state: &mut LearnerConceptState,
    delta: f32,
    reason: &str,
) -> Result<(), domain::DomainError> {
    domain::confidence::validate_delta(delta)?;
    let new = (state.learner_confidence + delta).clamp(0.0, 1.0);
    tracing::info!(
        learner_id = %state.learner_id,
        concept_id = %state.concept_id,
        old = state.learner_confidence,
        new,
        reason,
        "learner confidence updated"
    );
    state.learner_confidence = new;
    Ok(())
}
