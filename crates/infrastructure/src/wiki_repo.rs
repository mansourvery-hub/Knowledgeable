//! Personal Knowledge Wiki cache (M8).
//!
//! The graph stays authoritative; these rows are derived artifacts. Reads
//! never generate — generation happens in `application::WikiService`, which
//! also owns staleness policy.

use chrono::{DateTime, Utc};
use domain::{ConceptWikiPage, PrerequisiteAnchor, RelatedAnchor};
use sqlx::SqlitePool;
use uuid::Uuid;

fn parse_dt(s: &str) -> DateTime<Utc> {
    s.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now())
}

fn parse_anchors<T>(raw: &str) -> Vec<T>
where
    T: for<'de> serde::Deserialize<'de>,
{
    serde_json::from_str(raw).unwrap_or_default()
}

/// Stored wiki page row.
#[allow(clippy::type_complexity)]
type WikiRow =
    (String, String, String, String, String, String, String, String, f32, i64, i64, String, String);

pub async fn get_page(
    pool: &SqlitePool,
    learner_id: Uuid,
    concept_id: Uuid,
) -> Result<Option<ConceptWikiPage>, sqlx::Error> {
    let row: Option<WikiRow> = sqlx::query_as(
        "SELECT id, learner_id, concept_id, title, summary, personalized_content, known_prerequisites, related_concepts, learner_confidence_at_generation, version, is_stale, created_at, updated_at
         FROM concept_wiki_pages WHERE learner_id = ? AND concept_id = ?",
    )
    .bind(learner_id.to_string())
    .bind(concept_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row.map(
        |(
            id,
            learner,
            concept,
            title,
            summary,
            content,
            prereqs,
            related,
            confidence,
            version,
            stale,
            created,
            updated,
        )| {
            ConceptWikiPage {
                id: id.parse().unwrap(),
                learner_id: learner.parse().unwrap(),
                concept_id: concept.parse().unwrap(),
                title,
                summary,
                personalized_content: content,
                known_prerequisites: parse_anchors::<PrerequisiteAnchor>(&prereqs),
                related_concepts: parse_anchors::<RelatedAnchor>(&related),
                learner_confidence_at_generation: confidence,
                version: version as u32,
                is_stale: stale != 0,
                created_at: parse_dt(&created),
                updated_at: parse_dt(&updated),
            }
        },
    ))
}

/// Insert or replace a page on regeneration. `version` must already be
/// bumped by the caller; `updated_at` always moves.
pub async fn save_page(pool: &SqlitePool, page: &ConceptWikiPage) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO concept_wiki_pages (id, learner_id, concept_id, title, summary, personalized_content, known_prerequisites, related_concepts, learner_confidence_at_generation, version, is_stale, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(learner_id, concept_id) DO UPDATE SET
           title = excluded.title, summary = excluded.summary,
           personalized_content = excluded.personalized_content,
           known_prerequisites = excluded.known_prerequisites,
           related_concepts = excluded.related_concepts,
           learner_confidence_at_generation = excluded.learner_confidence_at_generation,
           version = excluded.version, is_stale = excluded.is_stale, updated_at = excluded.updated_at",
    )
    .bind(page.id.to_string())
    .bind(page.learner_id.to_string())
    .bind(page.concept_id.to_string())
    .bind(&page.title)
    .bind(&page.summary)
    .bind(&page.personalized_content)
    .bind(serde_json::to_string(&page.known_prerequisites).unwrap_or_else(|_| "[]".into()))
    .bind(serde_json::to_string(&page.related_concepts).unwrap_or_else(|_| "[]".into()))
    .bind(page.learner_confidence_at_generation)
    .bind(page.version as i64)
    .bind(if page.is_stale { 1 } else { 0 })
    .bind(page.created_at.to_rfc3339())
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Flag a page stale after graph evidence changes it. No-op when no page
/// exists yet; generation stays lazy on next view.
pub async fn mark_stale(
    pool: &SqlitePool,
    learner_id: Uuid,
    concept_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE concept_wiki_pages SET is_stale = 1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE learner_id = ? AND concept_id = ?",
    )
    .bind(learner_id.to_string())
    .bind(concept_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}
