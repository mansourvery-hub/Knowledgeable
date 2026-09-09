use axum::{extract::State, Json};
use domain::ConceptNode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::routes::AppState;

#[derive(Serialize, Deserialize)]
pub struct CreateConceptRequest {
    pub name: String,
    pub statement: String,
}

pub async fn handle_create_concept(
    State(state): State<AppState>,
    Json(payload): Json<CreateConceptRequest>,
) -> Json<ConceptNode> {
    let node = ConceptNode {
        id: Uuid::new_v4(),
        canonical_name: payload.name,
        canonical_statement: payload.statement,
        learner_statement: None,
        world_confidence: 1.0,
        status: domain::ConceptStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    let pool = state.pool.clone().unwrap();
    let graph = application::graph_service::GraphService::new(std::sync::Arc::new(pool));
    graph.create_concept(&node).await.unwrap();
    Json(node)
}
