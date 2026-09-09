use uuid::Uuid;

use crate::{ConceptNode, ConceptRelation};

/// Bounded graph repository port — domain interface, infrastructure implements.
///
/// Per `architecture.md -> Graph` all queries are bounded and learner-scoped.
/// DB-agnostic: no SQL types leak into domain; SQLite or PostgreSQL can implement later.
#[async_trait::async_trait]
pub trait GraphRepository: Send + Sync {
    async fn find_concept(
        &self,
        learner_id: Uuid,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ConceptNode>, anyhow::Error>;

    async fn get_concept(&self, id: Uuid) -> Result<Option<ConceptNode>, anyhow::Error>;

    async fn get_dependencies(
        &self,
        concept_id: Uuid,
        depth: u8,
    ) -> Result<Vec<ConceptRelation>, anyhow::Error>;
}
