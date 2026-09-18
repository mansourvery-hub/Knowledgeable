use crate::routes::AppState;
use axum::{extract::State, Json};
use domain::ConceptNode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct CreateConceptRequest {
    pub name: String,
    pub statement: String,
}

pub async fn handle_get_graph(State(state): State<AppState>) -> Json<serde_json::Value> {
    let pool = state.pool.clone().unwrap();
    let graph = application::graph_service::GraphService::new(std::sync::Arc::new(pool));
    // Retrieve all concepts and relations
    let concepts = graph.find_concepts("", 100).await.unwrap();
    let mut all_relations = Vec::new();
    for concept in &concepts {
        let relations = graph.get_related_concepts(concept.id, 100).await.unwrap();
        all_relations.extend(relations);
    }

    Json(serde_json::json!({
        "nodes": concepts,
        "links": all_relations
    }))
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
