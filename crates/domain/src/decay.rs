use chrono::{DateTime, Utc};

/// Simple long-horizon decay per `data_models.md -> Constants` and `tech_stack_and_rules.md -> Decay Rules`.
///
/// Model:
/// - if age <= grace_period => unchanged
/// - else exponential decay with half-life
pub fn decayed_confidence(
    learner_confidence: f32,
    last_reinforced_at: Option<DateTime<Utc>>,
    last_evaluated_at: DateTime<Utc>,
    now: DateTime<Utc>,
    grace_years: f32,
    half_life_years: f32,
) -> f32 {
    let base_time = last_reinforced_at.unwrap_or(last_evaluated_at);
    let age_secs = (now - base_time).num_seconds().max(0) as f32;
    let age_years = age_secs / (365.25 * 24.0 * 3600.0);

    if age_years <= grace_years {
        return learner_confidence;
    }

    let decay_years = age_years - grace_years;
    // exponential: confidence * 0.5^(decay_years / half_life)
    let factor = 0.5_f32.powf(decay_years / half_life_years);
    (learner_confidence * factor).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn no_decay_within_grace() {
        let base = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let c = decayed_confidence(0.95, Some(base), base, now, 2.0, 12.0);
        assert!((c - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn decays_after_grace() {
        let base = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let c = decayed_confidence(1.0, Some(base), base, now, 2.0, 12.0);
        assert!(c < 1.0 && c > 0.0);
    }
}
