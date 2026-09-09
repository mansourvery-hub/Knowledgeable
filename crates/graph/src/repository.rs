use uuid::Uuid;

/// Bounded graph repository trait — implementations use SQLx/PostgreSQL.
///
///
/// Per `architecture.md -> Graph` all queries are bounded and learner-scoped.
#[async_trait::async_trait]
pub trait GraphRepository: Send + Sync {
    async fn find_concept(
        &self,
        learner_id: Uuid,
        query: &str,
        limit: usize,
    ) -> Result<Vec<domain::ConceptNode>, anyhow::Error>;

    async fn get_concept(&self, id: Uuid) -> Result<Option<domain::ConceptNode>, anyhow::Error>;

    async fn get_dependencies(
        &self,
        concept_id: Uuid,
        depth: u8,
    ) -> Result<Vec<domain::ConceptRelation>, anyhow::Error>;
}
