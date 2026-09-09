use chrono::{DateTime, Utc};
use domain::{ConceptNode, ConceptRelation, ConceptStatus, RelationType};
use sqlx::SqlitePool;
use uuid::Uuid;

fn parse_dt(s: &str) -> DateTime<Utc> {
    s.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now())
}

pub async fn create_concept(
    pool: &SqlitePool,
    node: &ConceptNode,
) -> Result<ConceptNode, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let id = node.id.to_string();
    
    sqlx::query(
        "INSERT INTO concept_nodes (id, canonical_name, canonical_statement, learner_statement, world_confidence, status, created_at, updated_at) 
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&node.canonical_name)
    .bind(&node.canonical_statement)
    .bind(&node.learner_statement)
    .bind(node.world_confidence)
    .bind(match node.status { ConceptStatus::Active => "active", ConceptStatus::Archived => "archived" })
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(node.clone())
}

pub async fn get_concept(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<ConceptNode>, sqlx::Error> {
    let id_s = id.to_string();
    let row = sqlx::query_as::<_, (String, String, String, Option<String>, f32, String, String, String)>(
        "SELECT id, canonical_name, canonical_statement, learner_statement, world_confidence, status, created_at, updated_at 
         FROM concept_nodes WHERE id = ?",
    )
    .bind(&id_s)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, name, stmt, l_stmt, conf, status, created, updated)| ConceptNode {
        id: id.parse().unwrap(),
        canonical_name: name,
        canonical_statement: stmt,
        learner_statement: l_stmt,
        world_confidence: conf,
        status: if status == "active" { ConceptStatus::Active } else { ConceptStatus::Archived },
        created_at: parse_dt(&created),
        updated_at: parse_dt(&updated),
    }))
}

pub async fn find_concepts(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
) -> Result<Vec<ConceptNode>, sqlx::Error> {
    let q = format!("%{}%", query);
    let rows = sqlx::query_as::<_, (String, String, String, Option<String>, f32, String, String, String)>(
        "SELECT id, canonical_name, canonical_statement, learner_statement, world_confidence, status, created_at, updated_at 
         FROM concept_nodes 
         WHERE canonical_name LIKE ? OR canonical_statement LIKE ?
         LIMIT ?",
    )
    .bind(&q)
    .bind(&q)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name, stmt, l_stmt, conf, status, created, updated)| ConceptNode {
            id: id.parse().unwrap(),
            canonical_name: name,
            canonical_statement: stmt,
            learner_statement: l_stmt,
            world_confidence: conf,
            status: if status == "active" { ConceptStatus::Active } else { ConceptStatus::Archived },
            created_at: parse_dt(&created),
            updated_at: parse_dt(&updated),
        })
        .collect())
}

// ... existing functions

pub async fn get_dependencies(
    pool: &SqlitePool,
    concept_id: Uuid,
) -> Result<Vec<ConceptRelation>, sqlx::Error> {
    let cid = concept_id.to_string();
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id, from_concept_id, to_concept_id, relation_type, created_at, updated_at 
         FROM concept_relations 
         WHERE from_concept_id = ? AND relation_type = 'dependency'",
    )
    .bind(&cid)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, from, to, rel, created, updated)| ConceptRelation {
            id: id.parse().unwrap(),
            from_concept_id: from.parse().unwrap(),
            to_concept_id: to.parse().unwrap(),
            relation_type: if rel == "semantic" { RelationType::Semantic } else { RelationType::Dependency },
            created_at: parse_dt(&created),
            updated_at: parse_dt(&updated),
        })
        .collect())
}

pub async fn get_related_concepts(
    pool: &SqlitePool,
    concept_id: Uuid,
    limit: i64,
) -> Result<Vec<ConceptRelation>, sqlx::Error> {
    let cid = concept_id.to_string();
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id, from_concept_id, to_concept_id, relation_type, created_at, updated_at 
         FROM concept_relations 
         WHERE (from_concept_id = ? OR to_concept_id = ?) AND relation_type = 'semantic'
         LIMIT ?",
    )
    .bind(&cid)
    .bind(&cid)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, from, to, rel, created, updated)| ConceptRelation {
            id: id.parse().unwrap(),
            from_concept_id: from.parse().unwrap(),
            to_concept_id: to.parse().unwrap(),
            relation_type: if rel == "semantic" { RelationType::Semantic } else { RelationType::Dependency },
            created_at: parse_dt(&created),
            updated_at: parse_dt(&updated),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use domain::{ConceptNode, ConceptStatus};
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn test_create_and_get() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        // Run migrations for the memory db. 
        // Need to ensure the relative path is correct from the test execution context.
        // During `cargo test -p infrastructure`, it's `crates/infrastructure`.
        // So `../../migrations` should be correct if it points to root/migrations.
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

        let node = ConceptNode {
            id: Uuid::new_v4(),
            canonical_name: "Entropy".to_string(),
            canonical_statement: "Entropy measures disorder.".to_string(),
            learner_statement: None,
            world_confidence: 1.0,
            status: ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        create_concept(&pool, &node).await.unwrap();
        let fetched = get_concept(&pool, node.id).await.unwrap().unwrap();
        
        assert_eq!(fetched.canonical_name, "Entropy");
    }
}
