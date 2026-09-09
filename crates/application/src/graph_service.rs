use domain::{ConceptNode, ConceptRelation};
use infrastructure::{graph_repo};
use sqlx::SqlitePool;
use uuid::Uuid;
use std::sync::Arc;

pub struct GraphService {
    pool: Arc<SqlitePool>,
}

impl GraphService {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn get_concept(&self, id: Uuid) -> Result<Option<ConceptNode>, anyhow::Error> {
        Ok(graph_repo::get_concept(&self.pool, id).await?)
    }

    pub async fn get_dependencies(&self, id: Uuid) -> Result<Vec<ConceptRelation>, anyhow::Error> {
        Ok(graph_repo::get_dependencies(&self.pool, id).await?)
    }

    pub async fn find_concepts(&self, query: &str, limit: i64) -> Result<Vec<ConceptNode>, anyhow::Error> {
        Ok(graph_repo::find_concepts(&self.pool, query, limit).await?)
    }

    pub async fn get_related_concepts(&self, id: Uuid, limit: i64) -> Result<Vec<ConceptRelation>, anyhow::Error> {
        Ok(graph_repo::get_related_concepts(&self.pool, id, limit).await?)
    }

    pub async fn create_concept(&self, node: &ConceptNode) -> Result<ConceptNode, anyhow::Error> {
        Ok(graph_repo::create_concept(&self.pool, node).await?)
    }
}
