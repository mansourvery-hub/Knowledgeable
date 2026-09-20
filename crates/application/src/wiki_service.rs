//! Personal Knowledge Wiki orchestration (M8).
//!
//! The graph stays authoritative: pages are cached projections, generated
//! lazily on view once the learner reaches mastery, served from SQLite
//! afterwards, and regenerated at most daily once stale. Generation failures
//! never write half-pages.

use domain::{ConceptWikiPage, PrerequisiteAnchor, RelatedAnchor};
use infrastructure::{graph_repo, learner_repo, wiki_repo};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum WikiError {
    #[error("concept not found")]
    ConceptNotFound,
    #[error("wiki not ready: {0}")]
    NotReady(String),
    #[error("wiki generation failed: {0}")]
    GenerationFailed(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

#[derive(Debug, Deserialize)]
struct WikiDraft {
    title: String,
    summary: String,
    personalized_content: String,
}

pub struct WikiService {
    pool: Arc<SqlitePool>,
    llm: Arc<dyn llm::LlmClient>,
    model: String,
}

impl WikiService {
    pub fn new(pool: Arc<SqlitePool>, llm: Arc<dyn llm::LlmClient>, model: String) -> Self {
        Self { pool, llm, model }
    }

    /// Load the learner's wiki page, generating or refreshing it lazily:
    /// - fresh cached page → served, no LLM call;
    /// - stale page newer than the regeneration interval → served stale;
    /// - stale old page, or no page with mastery (`>= 0.70`) → regenerate;
    /// - no page below mastery → [`WikiError::NotReady`].
    pub async fn get_wiki(
        &self,
        learner_id: Uuid,
        concept_id: Uuid,
    ) -> Result<ConceptWikiPage, WikiError> {
        let concept = graph_repo::get_concept(&self.pool, concept_id)
            .await?
            .ok_or(WikiError::ConceptNotFound)?;
        let confidence = learner_repo::get_confidence(&self.pool, learner_id, concept_id).await?;

        match wiki_repo::get_page(&self.pool, learner_id, concept_id).await? {
            Some(page) if !page.is_stale => Ok(page),
            Some(page)
                if page.updated_at
                    + chrono::Duration::hours(domain::WIKI_MIN_REGENERATION_INTERVAL_HOURS)
                    > chrono::Utc::now() =>
            {
                Ok(page)
            }
            Some(page) => self.regenerate(learner_id, &concept, confidence, page.version + 1).await,
            None => match confidence {
                Some(c) if c >= domain::WIKI_MASTERY_THRESHOLD => {
                    self.regenerate(learner_id, &concept, confidence, 1).await
                }
                _ => Err(WikiError::NotReady("concept below wiki mastery threshold".into())),
            },
        }
    }

    async fn regenerate(
        &self,
        learner_id: Uuid,
        concept: &domain::ConceptNode,
        confidence: Option<f32>,
        version: u32,
    ) -> Result<ConceptWikiPage, WikiError> {
        let at_generation = confidence.unwrap_or(0.0);
        let prerequisites = self.prerequisite_anchors(learner_id, concept.id).await?;
        let related = self.related_anchors(concept.id).await?;
        let prompt = wiki_prompt(concept, at_generation, &prerequisites, &related);

        let draft: WikiDraft = self
            .llm
            .generate_structured(llm::LlmStructuredRequest {
                model: self.model.clone(),
                prompt,
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "summary": { "type": "string" },
                        "personalized_content": { "type": "string" },
                    },
                    "required": ["title", "summary", "personalized_content"],
                }),
            })
            .await
            .map_err(|e| WikiError::GenerationFailed(e.to_string()))
            .and_then(|value| {
                serde_json::from_value(value)
                    .map_err(|e| WikiError::GenerationFailed(format!("draft shape: {e}")))
            })?;

        let now = chrono::Utc::now();
        let page = ConceptWikiPage {
            id: Uuid::new_v4(),
            learner_id,
            concept_id: concept.id,
            title: draft.title,
            summary: draft.summary,
            personalized_content: draft.personalized_content,
            known_prerequisites: prerequisites,
            related_concepts: related,
            learner_confidence_at_generation: at_generation,
            version,
            is_stale: false,
            created_at: now,
            updated_at: now,
        };
        page.validate().map_err(|e| WikiError::GenerationFailed(e.to_string()))?;
        wiki_repo::save_page(&self.pool, &page).await?;
        Ok(page)
    }

    /// Known (state-bearing) direct dependencies, capped for prompt size.
    async fn prerequisite_anchors(
        &self,
        learner_id: Uuid,
        concept_id: Uuid,
    ) -> Result<Vec<PrerequisiteAnchor>, WikiError> {
        let mut anchors = Vec::new();
        for rel in graph_repo::get_dependencies(&self.pool, concept_id).await? {
            if anchors.len() >= 8 {
                break;
            }
            let (Some(node), Some(confidence)) = (
                graph_repo::get_concept(&self.pool, rel.to_concept_id).await?,
                learner_repo::get_confidence(&self.pool, learner_id, rel.to_concept_id).await?,
            ) else {
                continue;
            };
            anchors.push(PrerequisiteAnchor {
                concept_id: node.id,
                name: node.canonical_name,
                learner_confidence: confidence,
            });
        }
        Ok(anchors)
    }

    async fn related_anchors(&self, concept_id: Uuid) -> Result<Vec<RelatedAnchor>, WikiError> {
        let mut anchors = Vec::new();
        for rel in graph_repo::get_related_concepts(&self.pool, concept_id, 8).await? {
            let other = if rel.from_concept_id == concept_id {
                rel.to_concept_id
            } else {
                rel.from_concept_id
            };
            let Some(node) = graph_repo::get_concept(&self.pool, other).await? else {
                continue;
            };
            anchors.push(RelatedAnchor {
                concept_id: node.id,
                name: node.canonical_name,
                relation_type: rel.relation_type,
            });
        }
        Ok(anchors)
    }
}

fn wiki_prompt(
    concept: &domain::ConceptNode,
    confidence: f32,
    prerequisites: &[PrerequisiteAnchor],
    related: &[RelatedAnchor],
) -> String {
    let prereqs = if prerequisites.is_empty() {
        "none recorded".to_string()
    } else {
        prerequisites
            .iter()
            .map(|p| format!("- {} (confidence {:.0}%)", p.name, p.learner_confidence * 100.0))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let related_list = if related.is_empty() {
        "none recorded".to_string()
    } else {
        related.iter().map(|r| format!("- {}", r.name)).collect::<Vec<_>>().join("\n")
    };
    format!(
        "Write a concise personal reference article for a learner studying \"{name}\" (their confidence: {confidence:.0}%).\n\
         Canonical truth (never contradict): {statement}\n\n\
         Prerequisites they know:\n{prereqs}\n\nRelated concepts: {related_list}\n\n\
         Return JSON with exactly: title (short), summary (one or two sentences), \
         personalized_content (markdown reference article: third-person declarative definition, \
         key facts, and where it fits, one worked example; anchor it in the known prerequisites \
         by name). House rules: no second-person tutoring patter (never \"think back\", \
         \"imagine you\", \"as you know\"), no questions to the reader, no check-for-understanding \
         questions — dialogue belongs in chat, this page is the durable record. \
         Personalize the representation, not the truth.",
        name = concept.canonical_name,
        confidence = confidence,
        statement = concept.canonical_statement,
        prereqs = prereqs,
        related_list = related_list,
    )
}

#[cfg(test)]
mod prompt_tests {
    use super::*;

    fn sample_prompt() -> String {
        let concept = domain::ConceptNode {
            id: uuid::Uuid::new_v4(),
            canonical_name: "Prime Number".into(),
            canonical_statement: "A natural number greater than 1 with exactly two divisors."
                .into(),
            learner_statement: None,
            world_confidence: 1.0,
            status: domain::ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        wiki_prompt(&concept, 0.98, &[], &[])
    }

    /// Wiki pages are durable reference articles, not tutor dialogue: the
    /// prompt must demand third-person declarative prose and ban Socratic
    /// patter and reader questions outright.
    #[test]
    fn wiki_prompt_demands_reference_article_not_dialogue() {
        let prompt = sample_prompt();
        for required in [
            "reference article",
            "third-person declarative",
            "no questions to the reader",
            "dialogue belongs in chat",
        ] {
            assert!(prompt.contains(required), "prompt missing: {required}");
        }
        for banned in ["Socratic-leaning", "then one check-for-understanding question"] {
            assert!(!prompt.contains(banned), "prompt still invites dialogue: {banned}");
        }
    }
}
