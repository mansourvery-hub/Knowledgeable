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
// KNOWN CAVEAT (found in review, not fixed here): `-` is not a word
// character, so a hyphenated candidate name (e.g. "Non-Euclidean") can
// register as matching *inside* a larger, unrelated hyphenated compound
// (e.g. "non-euclidean-geometry"), because the character immediately after
// the candidate's span is a hyphen and therefore counts as a boundary.
// Left as-is pending a decision on whether `-` should count as a word
// character for this domain's concept names; see
// `hyphenated_name_matches_inside_larger_compound_known_caveat` below.

/// Deterministically match concept names against turn text.
///
/// - Case-insensitive, Unicode-aware (`to_lowercase` on both sides).
/// - Longest-name-wins on overlap ("Prime Number" beats "Number"). Ties
///   (equal-length names) break by the order they appear in `candidates`,
///   since sorting is stable.
/// - Single-pass scan: all candidate names are matched against the text in
///   one left-to-right automaton pass instead of one scan per candidate.
///   Longest-first claiming, deduplication, and the cap apply afterwards
///   exactly as before, so outputs are unchanged.
/// - Every matching candidate is found and positioned *before* the cap is
///   applied, so a tight `limit` keeps the earliest mentions in the text,
///   never a later-but-longer-named concept at the expense of an earlier
///   shorter-named one.
/// - Deduplicated; capped at `min(limit, MAX_ANNOTATIONS_PER_TURN)`.
/// - Names shorter than [`MIN_MATCHABLE_NAME_LEN`] are skipped.
///
/// Returns concept ids only; the caller attaches confidence/status.
pub fn match_mentions(text: &str, candidates: &[(Uuid, String)], limit: usize) -> Vec<Uuid> {
    let limit = limit.min(MAX_ANNOTATIONS_PER_TURN);
    if limit == 0 || text.trim().is_empty() {
        return Vec::new();
    }
    let hay = text.to_lowercase();

    // Candidate indexes in discovery order: longest names first so they
    // claim their spans before shorter overlaps; ties keep `candidates`
    // order (stable sort), which is the documented tie-break.
    let mut order: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter(|(_, (_, name))| name.chars().count() >= MIN_MATCHABLE_NAME_LEN)
        .map(|(i, _)| i)
        .collect();
    order.sort_by_key(|&i| std::cmp::Reverse(candidates[i].1.chars().count()));

    let automaton =
        match aho_corasick::AhoCorasick::new(order.iter().map(|&i| candidates[i].1.to_lowercase()))
        {
            Ok(automaton) => automaton,
            // Unreachable: every needle is non-empty (length-filtered above).
            Err(_) => return Vec::new(),
        };

    // Every pattern occurrence in one pass, grouped by pattern
    // (construction order == `order` sequence).
    let mut occurrences: Vec<Vec<(usize, usize)>> = vec![Vec::new(); order.len()];
    for mat in automaton.find_iter(&hay) {
        occurrences[mat.pattern().as_usize()].push((mat.start(), mat.end()));
    }

    // Byte offset of each char start, for O(log n) byte->char mapping of
    // match spans (automaton reports byte offsets; claiming is per char).
    let char_starts: Vec<usize> = hay.char_indices().map(|(byte, _)| byte).collect();
    let to_char = |byte: usize| -> usize {
        match char_starts.binary_search(&byte) {
            Ok(i) | Err(i) => i,
        }
    };

    let mut claimed = vec![false; hay.chars().count()];
    // (first mention position, id). Collected across the *entire* candidate
    // list (bounded by the caller, so this stays cheap) before any cap is
    // applied -- see the doc comment above for why the cap must come last.
    let mut hits: Vec<(usize, Uuid)> = Vec::new();

    for (pattern_idx, &candidate_idx) in order.iter().enumerate() {
        let (id, _) = &candidates[candidate_idx];
        // A duplicate id in `candidates` (two aliases for one concept, say)
        // only needs its first successful match recorded.
        if hits.iter().any(|(_, hit)| hit == id) {
            continue;
        }
        // Leftmost-first claiming over this pattern's whole-word matches,
        // mirroring the old per-candidate scan exactly.
        let mut spans = std::mem::take(&mut occurrences[pattern_idx]);
        spans.sort_by_key(|&(start, _)| start);
        let mut first: Option<usize> = None;
        for (byte_start, byte_end) in spans {
            if !is_match_boundary(&hay, byte_start, byte_end) {
                continue;
            }
            let span = to_char(byte_start)..to_char(byte_end);
            if claimed[span.clone()].iter().all(|c| !c) {
                claimed[span].fill(true);
                first.get_or_insert(to_char(byte_start));
            }
        }
        if let Some(pos) = first {
            hits.push((pos, *id));
        }
    }

    hits.sort_by_key(|(pos, _)| *pos);
    hits.truncate(limit);
    hits.into_iter().map(|(_, id)| id).collect()
}

/// Whole-word check for one automaton match: both neighbors must not be
/// word characters, so "Factor" never matches inside "factory". `start`
/// and `end` are byte offsets on char boundaries (guaranteed: the matched
/// pattern is valid UTF-8, so its edges align with `hay`'s chars).
fn is_match_boundary(hay: &str, start: usize, end: usize) -> bool {
    let before_ok = hay[..start].chars().next_back().map_or(true, |c| !is_word_char(c));
    let after_ok = hay[end..].chars().next().map_or(true, |c| !is_word_char(c));
    before_ok && after_ok
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

    /// Regression test for a bug found in review: a tight cap must keep the
    /// *earliest-mentioned* concepts, not whichever concepts happen to have
    /// the longest names. Before the fix, "Zzzzzzzzzz" (mentioned later)
    /// would win the single slot over "Ab" (mentioned first, position 0)
    /// purely because it sorts before "Ab" in the longest-first pass.
    #[test]
    fn cap_keeps_earliest_mention_not_longest_name() {
        let early_short = id();
        let later_long = id();
        let out = match_mentions(
            "Ab is related to Zzzzzzzzzz.",
            &[(later_long, "Zzzzzzzzzz".into()), (early_short, "Ab".into())],
            1,
        );
        assert_eq!(
            out,
            vec![early_short],
            "cap should keep the first mention, not the longest name"
        );
    }

    /// Documents the tie-break for equal-length overlapping names: it
    /// follows the order the candidates were supplied in, not any other
    /// rule. This pins down current behavior since it isn't otherwise
    /// specified.
    #[test]
    fn tie_breaks_by_candidate_order_when_names_are_equal_length() {
        let first_given = id();
        let second_given = id();
        // Both names are 4 chars; whichever is listed first in `candidates`
        // is tried first and claims the span.
        let out = match_mentions(
            "abcd",
            &[(first_given, "abcd".into()), (second_given, "abcd".into())],
            10,
        );
        assert_eq!(out, vec![first_given]);
    }

    /// Known caveat found in review, not fixed here (see the comment above
    /// `is_word_char`): a hyphenated candidate name can match as a prefix of
    /// an unrelated larger hyphenated compound, because `-` counts as a word
    /// boundary. This test pins current behavior down so a future change to
    /// `is_word_char` is a deliberate, visible decision.
    #[test]
    fn hyphenated_name_matches_inside_larger_compound_known_caveat() {
        let non_euclidean = id();
        let out = match_mentions(
            "non-euclidean-geometry is a distinct topic",
            &[(non_euclidean, "Non-Euclidean".into())],
            10,
        );
        assert_eq!(
            out,
            vec![non_euclidean],
            "documents current behavior: hyphen after the candidate's span counts as a boundary"
        );
    }
}
