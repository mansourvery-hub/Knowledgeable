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

pub async fn get_concept(pool: &SqlitePool, id: Uuid) -> Result<Option<ConceptNode>, sqlx::Error> {
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
            status: if status == "active" {
                ConceptStatus::Active
            } else {
                ConceptStatus::Archived
            },
            created_at: parse_dt(&created),
            updated_at: parse_dt(&updated),
        })
        .collect())
}

pub async fn get_weak_dependencies(
    pool: &SqlitePool,
    concept_id: Uuid,
    threshold: f32,
    _depth: u8,
) -> Result<Vec<(ConceptNode, f32)>, sqlx::Error> {
    let cid = concept_id.to_string();

    let rows = sqlx::query_as::<_, (String, String, String, Option<String>, f32, String, String, String, f32)>(
        "SELECT n.id, n.canonical_name, n.canonical_statement, n.learner_statement, n.world_confidence, n.status, n.created_at, n.updated_at, s.learner_confidence
         FROM concept_relations r
         JOIN concept_nodes n ON r.to_concept_id = n.id
         JOIN learner_concept_states s ON n.id = s.concept_id
         WHERE r.from_concept_id = ? 
           AND r.relation_type = 'dependency'
           AND s.learner_confidence < ?"
    )
    .bind(&cid)
    .bind(threshold)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name, stmt, l_stmt, conf, status, created, updated, learner_conf)| {
            (
                ConceptNode {
                    id: id.parse().unwrap(),
                    canonical_name: name,
                    canonical_statement: stmt,
                    learner_statement: l_stmt,
                    world_confidence: conf,
                    status: if status == "active" {
                        ConceptStatus::Active
                    } else {
                        ConceptStatus::Archived
                    },
                    created_at: parse_dt(&created),
                    updated_at: parse_dt(&updated),
                },
                learner_conf,
            )
        })
        .collect())
}

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
            relation_type: if rel == "semantic" {
                RelationType::Semantic
            } else {
                RelationType::Dependency
            },
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
            relation_type: if rel == "semantic" {
                RelationType::Semantic
            } else {
                RelationType::Dependency
            },
            created_at: parse_dt(&created),
            updated_at: parse_dt(&updated),
        })
        .collect())
}

/// Bounded neighborhood for graph inspection (Phase 6, Brick G1).
///
/// Returns `None` when the root concept does not exist so the API can map to 404.
/// Otherwise returns `(nodes with learner confidence, edges among returned nodes)`.
///
/// Traversal budget (bounded by design, never a full-graph dump):
/// - dependency ancestors via recursive CTE up to `depth` (clamped to ≤5 via
///   `domain::bounded_depth`, default 3)
/// - semantic 1-hop neighbors of every dependency-closure node
/// - `limit` clamps total nodes to [1, 100]; root is always first so a tiny
///   limit still returns the requested concept.
pub async fn get_neighborhood(
    pool: &SqlitePool,
    learner_id: Uuid,
    concept_id: Uuid,
    depth: u8,
    limit: i64,
) -> Result<Option<(Vec<(ConceptNode, Option<f32>)>, Vec<ConceptRelation>)>, sqlx::Error> {
    let depth = domain::bounded_depth(Some(depth));
    let limit = limit.clamp(1, 100) as usize;

    if get_concept(pool, concept_id).await?.is_none() {
        return Ok(None);
    }
    let root_s = concept_id.to_string();

    // 1. Dependency closure (root + ancestors) ordered by distance.
    let dep_rows: Vec<(String, i64)> = sqlx::query_as(
        "WITH RECURSIVE deps(id, d) AS (
           SELECT ? AS id, 0 AS d
           UNION
           SELECT r.to_concept_id AS id, deps.d + 1 AS d
           FROM concept_relations r
           JOIN deps ON r.from_concept_id = deps.id
           WHERE r.relation_type = 'dependency' AND deps.d < ?
         )
         SELECT DISTINCT id, MIN(d) AS dist FROM deps GROUP BY id ORDER BY dist",
    )
    .bind(&root_s)
    .bind(i64::from(depth))
    .fetch_all(pool)
    .await?;
    let mut ordered_ids: Vec<String> = dep_rows.into_iter().map(|(id, _)| id).collect();

    // 2. Semantic 1-hop neighbors of the closure (depth 0 = root only).
    if depth >= 1 && !ordered_ids.is_empty() {
        let placeholders = vec!["?"; ordered_ids.len()].join(",");
        let sql = format!(
            "SELECT DISTINCT from_concept_id, to_concept_id FROM concept_relations \
             WHERE relation_type = 'semantic' \
             AND (from_concept_id IN ({p}) OR to_concept_id IN ({p}))",
            p = placeholders
        );
        let mut q = sqlx::query_as::<_, (String, String)>(&sql);
        for id in &ordered_ids {
            q = q.bind(id);
        }
        for id in &ordered_ids {
            q = q.bind(id);
        }
        let pairs: Vec<(String, String)> = q.fetch_all(pool).await?;
        let known: std::collections::HashSet<&str> =
            ordered_ids.iter().map(String::as_str).collect();
        let mut semantic_new: Vec<String> = Vec::new();
        for (from, to) in pairs {
            for candidate in [from, to] {
                if !known.contains(candidate.as_str())
                    && !semantic_new.iter().any(|s| s == &candidate)
                {
                    semantic_new.push(candidate);
                }
            }
        }
        semantic_new.sort();
        ordered_ids.extend(semantic_new);
    }

    // 3. Truncate to limit, root always first.
    ordered_ids.truncate(limit);

    // 4. Fetch nodes with learner confidence (LEFT JOIN: unseen -> None).
    let placeholders = vec!["?"; ordered_ids.len()].join(",");
    let sql = format!(
        "SELECT n.id, n.canonical_name, n.canonical_statement, n.learner_statement, \
                n.world_confidence, n.status, n.created_at, n.updated_at, \
                s.learner_confidence \
         FROM concept_nodes n \
         LEFT JOIN learner_concept_states s \
           ON s.concept_id = n.id AND s.learner_id = ? \
         WHERE n.id IN ({p})",
        p = placeholders
    );
    let mut q = sqlx::query_as::<
        _,
        (String, String, String, Option<String>, f32, String, String, String, Option<f32>),
    >(&sql);
    q = q.bind(learner_id.to_string());
    for id in &ordered_ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(pool).await?;
    // Re-order to match ordered_ids (root first, then BFS, then semantic).
    let position: std::collections::HashMap<&str, usize> =
        ordered_ids.iter().enumerate().map(|(i, id)| (id.as_str(), i)).collect();
    let mut nodes: Vec<(ConceptNode, Option<f32>)> = rows
        .into_iter()
        .map(|(id, name, stmt, l_stmt, conf, status, created, updated, learner_conf)| {
            (
                ConceptNode {
                    id: id.parse().unwrap(),
                    canonical_name: name,
                    canonical_statement: stmt,
                    learner_statement: l_stmt,
                    world_confidence: conf,
                    status: if status == "active" {
                        ConceptStatus::Active
                    } else {
                        ConceptStatus::Archived
                    },
                    created_at: parse_dt(&created),
                    updated_at: parse_dt(&updated),
                },
                learner_conf,
            )
        })
        .collect();
    nodes.sort_by_key(|(n, _)| {
        position.get(n.id.to_string().as_str()).copied().unwrap_or(usize::MAX)
    });

    // 5. Edges among returned nodes only (no dangling endpoints for the visualizer).
    let ids: Vec<String> = nodes.iter().map(|(n, _)| n.id.to_string()).collect();
    let edges = if ids.is_empty() {
        Vec::new()
    } else {
        let placeholders = vec!["?"; ids.len()].join(",");
        let sql = format!(
            "SELECT id, from_concept_id, to_concept_id, relation_type, created_at, updated_at \
             FROM concept_relations \
             WHERE from_concept_id IN ({p}) AND to_concept_id IN ({p})",
            p = placeholders
        );
        let mut q = sqlx::query_as::<_, (String, String, String, String, String, String)>(&sql);
        for id in &ids {
            q = q.bind(id);
        }
        for id in &ids {
            q = q.bind(id);
        }
        q.fetch_all(pool)
            .await?
            .into_iter()
            .map(|(id, from, to, rel, created, updated)| ConceptRelation {
                id: id.parse().unwrap(),
                from_concept_id: from.parse().unwrap(),
                to_concept_id: to.parse().unwrap(),
                relation_type: if rel == "semantic" {
                    RelationType::Semantic
                } else {
                    RelationType::Dependency
                },
                created_at: parse_dt(&created),
                updated_at: parse_dt(&updated),
            })
            .collect()
    };

    Ok(Some((nodes, edges)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{ConceptNode, ConceptStatus};
    use sqlx::SqlitePool;
    use uuid::Uuid;

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

    fn test_node(name: &str) -> ConceptNode {
        ConceptNode {
            id: Uuid::new_v4(),
            canonical_name: name.to_string(),
            canonical_statement: format!("{name} statement."),
            learner_statement: None,
            world_confidence: 1.0,
            status: ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    async fn seed_neighborhood(pool: &SqlitePool) -> (Uuid, Uuid, Uuid, Uuid, Uuid) {
        let learner = Uuid::new_v4();
        sqlx::query("INSERT INTO learners (id) VALUES (?)")
            .bind(learner.to_string())
            .execute(pool)
            .await
            .unwrap();

        let a = test_node("A-root");
        let b = test_node("B-dep");
        let c = test_node("C-transitive");
        let d = test_node("D-semantic");
        for n in [&a, &b, &c, &d] {
            create_concept(pool, n).await.unwrap();
        }
        // A depends_on B, B depends_on C, A semantic D
        for (from, to, rel) in
            [(a.id, b.id, "dependency"), (b.id, c.id, "dependency"), (a.id, d.id, "semantic")]
        {
            sqlx::query(
                "INSERT INTO concept_relations (id, from_concept_id, to_concept_id, relation_type) VALUES (?, ?, ?, ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(from.to_string())
            .bind(to.to_string())
            .bind(rel)
            .execute(pool)
            .await
            .unwrap();
        }
        // Learner states: B weak, C healthy, A mid; D has no state (None)
        for (concept, conf) in [(a.id, 0.9f32), (b.id, 0.3f32), (c.id, 0.98f32)] {
            sqlx::query(
                "INSERT INTO learner_concept_states (learner_id, concept_id, learner_confidence) VALUES (?, ?, ?)",
            )
            .bind(learner.to_string())
            .bind(concept.to_string())
            .bind(conf)
            .execute(pool)
            .await
            .unwrap();
        }
        (learner, a.id, b.id, c.id, d.id)
    }

    #[tokio::test]
    async fn neighborhood_depth_zero_returns_only_root() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let (learner, a, _, _, _) = seed_neighborhood(&pool).await;

        let result = get_neighborhood(&pool, learner, a, 0, 100).await.unwrap();
        assert!(result.is_some());
        let (nodes, _edges) = result.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].0.id, a);
    }

    #[tokio::test]
    async fn neighborhood_depth_one_excludes_transitive() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let (learner, a, b, c, d) = seed_neighborhood(&pool).await;

        let (nodes, edges) = get_neighborhood(&pool, learner, a, 1, 100).await.unwrap().unwrap();
        let ids: std::collections::HashSet<Uuid> = nodes.iter().map(|(n, _)| n.id).collect();
        assert!(ids.contains(&a));
        assert!(ids.contains(&b));
        assert!(ids.contains(&d), "semantic 1-hop neighbor must be included");
        assert!(!ids.contains(&c), "transitive dep must be excluded at depth 1");
        // Edge types must be distinguishable
        assert!(edges.iter().any(|e| e.relation_type == RelationType::Dependency));
        assert!(edges.iter().any(|e| e.relation_type == RelationType::Semantic));
    }

    #[tokio::test]
    async fn neighborhood_depth_two_includes_transitive_with_confidence() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let (learner, a, b, c, d) = seed_neighborhood(&pool).await;

        let (nodes, _edges) = get_neighborhood(&pool, learner, a, 2, 100).await.unwrap().unwrap();
        let map: std::collections::HashMap<Uuid, Option<f32>> =
            nodes.into_iter().map(|(n, conf)| (n.id, conf)).collect();
        assert_eq!(map.len(), 4);
        assert_eq!(map[&b], Some(0.3));
        assert_eq!(map[&c], Some(0.98));
        assert_eq!(map[&d], None, "concept without learner state must yield None");
    }

    #[tokio::test]
    async fn neighborhood_missing_concept_returns_none() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let learner = Uuid::new_v4();
        let missing = Uuid::new_v4();
        let result = get_neighborhood(&pool, learner, missing, 2, 100).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn neighborhood_clamps_depth_and_limit() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let (learner, a, _, _, _) = seed_neighborhood(&pool).await;
        // Depth 255 must clamp to 5, not error or run away
        let (nodes, _) = get_neighborhood(&pool, learner, a, 255, 100).await.unwrap().unwrap();
        assert!(!nodes.is_empty());
        // Limit 0 clamps to at least 1 (root always present)
        let (nodes, _) = get_neighborhood(&pool, learner, a, 2, 0).await.unwrap().unwrap();
        assert_eq!(nodes.len(), 1);
    }
}
