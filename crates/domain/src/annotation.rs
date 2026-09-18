//! Concept annotations for M7 chat rendering.
//!
//! The backend attaches these to a completed tutor turn so the client can
//! highlight recognized learner concepts without the LLM emitting brittle
//! markup. Matching is deterministic substring search over the turn text —
//! never LLM-generated — so LaTeX and code blocks pass through untouched.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Display threshold per `roadmap M7`: badges show `Known` at/above 0.80.
///
/// This is intentionally lower than [`crate::confidence::HEALTHY_THRESHOLD`]
/// (0.95), which drives review eligibility. A concept can render as "known"
/// while still being review-eligible.
pub const ANNOTATION_KNOWN_THRESHOLD: f32 = 0.80;

/// Hard cap on annotations per turn: bounds the SSE payload and client work.
pub const MAX_ANNOTATIONS_PER_TURN: usize = 20;

/// Minimum concept-name length eligible for matching. Single characters would
/// match inside nearly every word (e.g. "a" in "factor").
pub const MIN_MATCHABLE_NAME_LEN: usize = 2;

/// Frontend badge state. Serializes lowercase to match
/// `apps/web/client/src/knowledgeable/types.ts::ConceptAnnotationStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationStatus {
    Known,
    Weak,
    New,
}

/// One recognized concept mention. Field names mirror the TS
/// `ConceptAnnotation` interface (`concept_id`, `name`, `learner_confidence`,
/// `status`); `name` is the concept's `canonical_name`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptAnnotation {
    pub concept_id: Uuid,
    pub name: String,
    pub learner_confidence: Option<f32>,
    pub status: AnnotationStatus,
}

/// Derive the badge state from learner confidence.
/// `None` (unseen) renders as `New`; non-`[0,1]` values such as `NaN` fall
/// through to `Weak` rather than panicking (payloads must never crash render).
pub fn status_for(learner_confidence: Option<f32>) -> AnnotationStatus {
    match learner_confidence {
        Some(c) if c >= ANNOTATION_KNOWN_THRESHOLD => AnnotationStatus::Known,
        Some(_) => AnnotationStatus::Weak,
        None => AnnotationStatus::New,
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Deterministically match concept names against turn text.
///
/// - Case-insensitive, Unicode-aware (`to_lowercase` on both sides).
/// - Whole-word/phrase only: neighbors must not be word characters, so
///   "Factor" never matches inside "factory".
/// - Longest-name-wins on overlap ("Prime Number" beats "Number").
/// - First-mention order, deduplicated; capped at
///   `min(limit, MAX_ANNOTATIONS_PER_TURN)`.
/// - Names shorter than [`MIN_MATCHABLE_NAME_LEN`] are skipped.
///
/// Returns concept ids only; the caller attaches confidence/status.
pub fn match_mentions(text: &str, candidates: &[(Uuid, String)], limit: usize) -> Vec<Uuid> {
    let limit = limit.min(MAX_ANNOTATIONS_PER_TURN);
    if limit == 0 || text.trim().is_empty() {
        return Vec::new();
    }
    let hay: Vec<char> = text.to_lowercase().chars().collect();

    // Longest names first so they claim their spans before shorter overlaps.
    let mut ordered: Vec<(Uuid, Vec<char>)> = candidates
        .iter()
        .filter(|(_, name)| name.chars().count() >= MIN_MATCHABLE_NAME_LEN)
        .map(|(id, name)| (*id, name.to_lowercase().chars().collect::<Vec<_>>()))
        .collect();
    ordered.sort_by_key(|(_, name)| std::cmp::Reverse(name.len()));

    let mut claimed = vec![false; hay.len()];
    // (first mention position, id) in discovery order.
    let mut hits: Vec<(usize, Uuid)> = Vec::new();

    for (id, needle) in &ordered {
        if hits.len() >= limit && hits.iter().any(|(_, hit)| hit == id) {
            continue;
        }
        let Some(first) = find_first(&hay, needle, &claimed) else {
            continue;
        };
        // Claim every occurrence so shorter names cannot reuse the span.
        claim_all(&hay, needle, &mut claimed);
        if !hits.iter().any(|(_, hit)| hit == id) {
            hits.push((first, *id));
        }
        if hits.len() >= limit {
            break;
        }
    }

    hits.sort_by_key(|(pos, _)| *pos);
    hits.truncate(limit);
    hits.into_iter().map(|(_, id)| id).collect()
}

/// Byte-free first match honoring word boundaries and claimed spans.
fn find_first(hay: &[char], needle: &[char], claimed: &[bool]) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    (0..=(hay.len() - needle.len())).find(|&i| {
        (i == 0 || !is_word_char(hay[i - 1]))
            && (i + needle.len() == hay.len() || !is_word_char(hay[i + needle.len()]))
            && hay[i..i + needle.len()] == *needle
            && claimed[i..i + needle.len()].iter().all(|c| !c)
    })
}

fn claim_all(hay: &[char], needle: &[char], claimed: &mut [bool]) {
    if needle.is_empty() || needle.len() > hay.len() {
        return;
    }
    for i in 0..=(hay.len() - needle.len()) {
        if (i == 0 || !is_word_char(hay[i - 1]))
            && (i + needle.len() == hay.len() || !is_word_char(hay[i + needle.len()]))
            && hay[i..i + needle.len()] == *needle
        {
            for c in &mut claimed[i..i + needle.len()] {
                *c = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn status_thresholds_match_m7_badges() {
        assert_eq!(status_for(Some(0.80)), AnnotationStatus::Known);
        assert_eq!(status_for(Some(0.99)), AnnotationStatus::Known);
        assert_eq!(status_for(Some(0.79)), AnnotationStatus::Weak);
        assert_eq!(status_for(Some(0.0)), AnnotationStatus::Weak);
        assert_eq!(status_for(None), AnnotationStatus::New);
        // Never panic on out-of-range input; degrade to Weak.
        assert_eq!(status_for(Some(f32::NAN)), AnnotationStatus::Weak);
    }

    #[test]
    fn serializes_to_frontend_contract() {
        let ann = ConceptAnnotation {
            concept_id: Uuid::nil(),
            name: "Prime Number".into(),
            learner_confidence: Some(0.3),
            status: AnnotationStatus::Weak,
        };
        let v = serde_json::to_value(&ann).unwrap();
        assert_eq!(v["concept_id"], "00000000-0000-0000-0000-000000000000");
        assert_eq!(v["name"], "Prime Number");
        // f32 serializes with float error; the wire shape matters, not exact bits.
        let conf = v["learner_confidence"].as_f64().unwrap();
        assert!((conf - 0.3).abs() < 1e-6, "unexpected confidence {conf}");
        assert_eq!(v["status"], "weak");
    }

    #[test]
    fn matches_case_insensitively_in_first_mention_order() {
        let prime = id();
        let factor = id();
        let out = match_mentions(
            "FACTOR pairs help explain prime number ideas. Factor again.",
            &[(prime, "Prime Number".into()), (factor, "Factor".into())],
            10,
        );
        assert_eq!(out, vec![factor, prime]);
    }

    #[test]
    fn rejects_subword_matches() {
        let factor = id();
        let out = match_mentions("the factory settings", &[(factor, "Factor".into())], 10);
        assert!(out.is_empty());
    }

    #[test]
    fn longest_match_wins_overlaps() {
        let prime_number = id();
        let number = id();
        let out = match_mentions(
            "a prime number",
            &[(number, "Number".into()), (prime_number, "Prime Number".into())],
            10,
        );
        assert_eq!(out, vec![prime_number]);
    }

    #[test]
    fn skips_tiny_names_and_empty_text() {
        let a = id();
        assert!(match_mentions("a prime", &[(a, "a".into())], 10).is_empty());
        assert!(match_mentions("   ", &[(a, "Prime".into())], 10).is_empty());
        assert!(match_mentions("prime", &[(a, "Prime".into())], 0).is_empty());
    }

    #[test]
    fn caps_annotations_per_turn() {
        let cands: Vec<(Uuid, String)> =
            (0..30).map(|i| (id(), format!("Concept{i:02}"))).collect();
        let text = (0..30).map(|i| format!("Concept{i:02}")).collect::<Vec<_>>().join(" ");
        let out = match_mentions(&text, &cands, 100);
        assert_eq!(out.len(), MAX_ANNOTATIONS_PER_TURN);
    }
}
