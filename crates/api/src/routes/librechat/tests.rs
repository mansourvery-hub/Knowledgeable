//! Router-level tests for the LibreChat adapter.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn test_state(pool: sqlx::SqlitePool) -> AppState {
    AppState {
        pool: Some(pool),
        llm: std::sync::Arc::new(llm::FakeLlmClient::new("test")),
        version: "test".into(),
        streams: crate::routes::new_registry(),
        web_dist_dir: None,
    }
}

async fn setup() -> (axum::Router, sqlx::SqlitePool) {
    setup_with_llm(std::sync::Arc::new(llm::FakeLlmClient::new("test"))).await
}

async fn setup_with_llm(
    llm: std::sync::Arc<dyn llm::LlmClient>,
) -> (axum::Router, sqlx::SqlitePool) {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
    application::conversation_service::ensure_default_learner(&pool).await.unwrap();
    let app = crate::routes::create_router(AppState {
        pool: Some(pool.clone()),
        llm,
        version: "test".into(),
        streams: crate::routes::new_registry(),
        web_dist_dir: None,
    });
    (app, pool)
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn sse_data(body: &str) -> Vec<serde_json::Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim)
        .filter(|data| !data.is_empty())
        .filter_map(|data| serde_json::from_str(data).ok())
        .collect()
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn body_string(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn config_advertises_knowledgeable_endpoint() {
    let (app, _pool) = setup().await;
    let response = app.oneshot(get("/api/config")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let config = body_json(response).await;
    assert_eq!(config["appTitle"], "Knowledgeable");
    assert_eq!(config["emailLoginEnabled"], false);
    assert!(config["endpoints"]["knowledgeable"].is_object());
}

#[tokio::test]
async fn config_disables_parameters_panel() {
    // Phase 1 seam: the client gates the Parameters side panel on
    // `interfaceConfig.parameters === true` (useSideNavLinks.ts), so the
    // adapter hides it with the existing switch — no UI rewrite.
    let (app, _pool) = setup().await;
    let response = app.oneshot(get("/api/config")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let config = body_json(response).await;
    assert_eq!(config["interface"]["parameters"], false);
}

#[tokio::test]
async fn config_interface_matches_phase1_policy() {
    // Locks the Phase 1 config-only end state: only `modelSelect` (model
    // picker) and `sidePanel` (hosts the Knowledgeable graph) stay visible;
    // every other interface surface stays hidden behind the existing switch.
    let (app, _pool) = setup().await;
    let response = app.oneshot(get("/api/config")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let config = body_json(response).await;
    let interface = &config["interface"];
    for visible in ["modelSelect", "sidePanel"] {
        assert_eq!(interface[visible], true, "{visible} must stay visible");
    }
    for hidden in [
        "parameters",
        "presets",
        "prompts",
        "bookmarks",
        "multiConvo",
        "agents",
        "temporaryChat",
        "runCode",
        "webSearch",
        "fileSearch",
        "contextUsage",
        "feedback",
    ] {
        assert_eq!(interface[hidden], false, "{hidden} must stay hidden");
    }
}

#[tokio::test]
async fn refresh_mints_local_session() {
    let (app, _pool) = setup().await;
    let response = app
        .oneshot(
            Request::builder().method("POST").uri("/api/auth/refresh").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let session = body_json(response).await;
    assert!(session["token"].is_string());
    assert!(session["user"]["id"].is_string());
}

#[tokio::test]
async fn endpoints_and_models_expose_knowledgeable() {
    let (app, _pool) = setup().await;
    let response = app.clone().oneshot(get("/api/endpoints")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let endpoints = body_json(response).await;
    assert_eq!(endpoints["knowledgeable"]["name"], "Knowledgeable");
    assert_eq!(endpoints["knowledgeable"]["type"], "custom");

    let response = app.oneshot(get("/api/models")).await.unwrap();
    let models = body_json(response).await;
    assert!(models["knowledgeable"].as_array().is_some_and(|m| !m.is_empty()));
}

#[tokio::test]
async fn convos_list_is_empty_initially() {
    let (app, _pool) = setup().await;
    let response = app.oneshot(get("/api/convos")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let list = body_json(response).await;
    assert_eq!(list["conversations"].as_array().unwrap().len(), 0);
    assert!(list["nextCursor"].is_null());
}

#[tokio::test]
async fn chat_stream_creates_conversation_and_persists_messages() {
    let (app, _pool) = setup().await;

    let payload = serde_json::json!({
        "conversationId": "new",
        "parentMessageId": NO_PARENT,
        "text": "What is a Prime Number?",
        "endpoint": ENDPOINT_NAME,
        "messageId": uuid::Uuid::new_v4().to_string(),
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let events = sse_data(&body_string(response).await);
    assert!(!events.is_empty(), "stream must emit at least one event");

    let created = events.iter().find(|e| e["created"] == true).expect("created event");
    let conversation_id = created["message"]["conversationId"].as_str().unwrap().to_string();
    assert_eq!(created["message"]["isCreatedByUser"], true);

    let final_event = events.iter().find(|e| e["final"] == true).expect("final event");
    assert_eq!(final_event["conversation"]["conversationId"], conversation_id);
    assert_eq!(final_event["responseMessage"]["isCreatedByUser"], false);
    assert!(
        final_event["responseMessage"]["text"].as_str().is_some_and(|t| !t.is_empty()),
        "assistant response must carry text"
    );

    // The turn is durable: history and sidebar reflect it.
    let response =
        app.clone().oneshot(get(&format!("/api/messages/{conversation_id}"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let messages = body_json(response).await;
    let messages = messages.as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["isCreatedByUser"], true);
    assert_eq!(messages[1]["isCreatedByUser"], false);
    // The synthesized chain wires each message to its predecessor.
    assert_eq!(messages[0]["parentMessageId"], NO_PARENT);
    assert_eq!(messages[1]["parentMessageId"], messages[0]["messageId"]);

    let response = app.clone().oneshot(get("/api/convos")).await.unwrap();
    let list = body_json(response).await;
    assert_eq!(list["conversations"].as_array().unwrap().len(), 1);
    assert_eq!(list["conversations"][0]["conversationId"], conversation_id);
}

#[tokio::test]
async fn gen_title_derives_from_first_user_message() {
    let (app, _pool) = setup().await;

    let payload = serde_json::json!({
        "conversationId": "new",
        "parentMessageId": NO_PARENT,
        "text": "Explain entropy to me",
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let events = sse_data(&body_string(response).await);
    let conversation_id = events
        .iter()
        .find(|e| e["created"] == true)
        .and_then(|e| e["message"]["conversationId"].as_str())
        .unwrap()
        .to_string();

    let response =
        app.oneshot(get(&format!("/api/convos/gen_title/{conversation_id}"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let title = body_json(response).await;
    assert_eq!(title["title"], "Explain entropy to me");
}

#[tokio::test]
async fn chat_rejects_empty_text() {
    let (app, _pool) = setup().await;
    let payload = serde_json::json!({ "conversationId": "new", "text": "   " });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn chat_rejects_malformed_conversation_id() {
    let (app, _pool) = setup().await;
    let payload = serde_json::json!({ "conversationId": "not-a-uuid", "text": "hello" });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// T9: the Vite-proxied `/api/graph/neighborhood` alias serves the same
/// controller as canonical `/v1/graph/neighborhood` with the same envelope.
#[tokio::test]
async fn graph_neighborhood_api_alias_matches_v1() {
    let (app, _pool) = setup().await;

    // Malformed UUID -> 400 validation_failed on both paths.
    for uri in [
        "/v1/graph/neighborhood?concept_id=not-a-uuid".to_string(),
        "/api/graph/neighborhood?concept_id=not-a-uuid".to_string(),
    ] {
        let response = app.clone().oneshot(get(&uri)).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "for {uri}");
        let body = body_json(response).await;
        assert_eq!(body["code"], "validation_failed", "for {uri}");
    }

    // Unknown UUID -> 404 not_found on both paths.
    let missing = uuid::Uuid::new_v4().to_string();
    for uri in [
        format!("/v1/graph/neighborhood?concept_id={missing}"),
        format!("/api/graph/neighborhood?concept_id={missing}"),
    ] {
        let response = app.clone().oneshot(get(&uri)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "for {uri}");
        let body = body_json(response).await;
        assert_eq!(body["code"], "not_found", "for {uri}");
    }
}

/// M7/A3: a completed turn emits a `concept_annotations` frame for graph
/// terms in the assistant reply, shaped for the TS `ConceptAnnotation`
/// contract. The frame carries no `text`/`final` keys so legacy clients
/// ignore it; annotation failure must never break the `final` event.
#[tokio::test]
async fn chat_stream_emits_concept_annotations_for_known_terms() {
    let (app, pool) = setup().await;

    // The fake LLM echoes "Prime Number" back when asked about it.
    let node = domain::ConceptNode {
        id: uuid::Uuid::new_v4(),
        canonical_name: "Prime Number".into(),
        canonical_statement: "Prime Number statement.".into(),
        learner_statement: None,
        world_confidence: 1.0,
        status: domain::ConceptStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    infrastructure::graph_repo::create_concept(&pool, &node).await.unwrap();

    let payload = serde_json::json!({
        "conversationId": "new",
        "parentMessageId": NO_PARENT,
        "text": "What is a Prime Number?",
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let events = sse_data(&body_string(response).await);
    let annotations = events
        .iter()
        .find(|e| e.get("concept_annotations").is_some())
        .expect("stream must include a concept_annotations frame")["concept_annotations"]
        .as_array()
        .expect("concept_annotations must be an array");
    assert!(!annotations.is_empty(), "expected at least one annotation");

    let prime = annotations
        .iter()
        .find(|a| a["name"] == "Prime Number")
        .expect("expected a Prime Number annotation");
    assert_eq!(prime["concept_id"], node.id.to_string());
    // No learner state seeded -> renders as New.
    assert_eq!(prime["status"], "new");
    assert!(prime["learner_confidence"].is_null());

    // The turn still terminates normally after the annotations frame.
    assert!(events.iter().any(|e| e["final"] == true));
}

/// Roles gate client UI and the ChatRoute boot gate (`roles.USER != null`).
/// Single-user local mode grants the USER role every gated capability;
/// unknown roles 404 instead of silently hiding UI.
/// Phase 2 baseline: this locks the EXACT granted set, because several
/// `useSideNavLinks` entries gate on these permissions alone — so shrinking
/// this set is what hides the removed surfaces centrally.
/// Step 1: MEMORIES revoked (policy §9.6) — the Memories panel, its settings
/// toggle, and chat-input memory affordances now read as denied.
/// Step 2: PROMPTS revoked (prompt management removed, slash commands a
/// disabled FUTURE) — Prompts panel, slash-command setting, and slash popover
/// now read as denied.
/// Step 3: MCP_SERVERS revoked (MCP UI hidden, future tutor-tool seam kept) —
/// side-panel entry, tools-dropdown row, and dialogs hide, and `useAppStartup`
/// suppresses all MCP queries without USE.
/// Step 4: SKILLS revoked (generic skills outside the product surface) —
/// side-panel entry, chat-input rows, and the `$` mention popover (which
/// early-returns without access) now read as denied.
/// Step 5: AGENTS revoked (agent builder/marketplace outside the product
/// surface) — endpoint picker, new-convo fallback, and provider-key listing
/// verified unaffected for the `knowledgeable` endpoint; `/agents` route
/// gating is a later step.
/// Step 6: MARKETPLACE revoked (agent marketplace outside the product
/// surface) — nav entry hidden (already false via AGENTS); direct `/agents`
/// nav renders null and redirects to `/c/new` by the component's own gate.
/// Step 7: REMOTE_AGENTS revoked (remote-agent sharing + generic agent API
/// keys outside the product surface) — settings API-keys entry, admin
/// permission editor, and footer share action now read as denied.
/// Step 8: WEB_SEARCH revoked (external retrieval a FUTURE/OPTIONAL, never a
/// core-tutor dependency) — badge-row toggle and tools-dropdown row hide via
/// the grant && capability pairing at every consumer.
/// Step 9: RUN_CODE revoked (execution FUTURE/OPTIONAL, display KEEP) —
/// `canRunCode` feeds only `CodeBlock allowExecution`, so highlighting, copy,
/// Mermaid, and math rendering are untouched while execution UI hides.
/// Step 10: FILE_SEARCH revoked (document RAG a FUTURE for learner-provided
/// material) — badge-row toggle and tools-dropdown row hide via the same
/// null-return / grant && capability pairing; no `/api/files/*` backend.
/// Step 11: FILE_CITATIONS revoked (dead grant — no client code gates on it;
/// only the unrelated `FileCitation` data type exists).
/// Step 12: SCHEDULES revoked (scheduled chats outside the product surface)
/// — all consumers live inside the Schedules surface itself, already hidden
/// via the absent interface flag; revocation double-locks it.
/// Step 13: MULTI_CONVO revoked (side-by-side compare outside the product
/// surface) — header button, header-menu item, `+` popover handler, and
/// settings toggle are all strict conditionals on the grant.
/// Step 14: TEMPORARY_CHAT revoked (ephemeral chats outside the product
/// surface) — header toggle/indicator, menu item, shortcut, and (via the
/// central settings-registry `show` gate) the `defaultTemporaryChat` toggle
/// hide; normal path untouched (backend never returns
/// `isTemporary`/`expiredAt`).
/// Step 15: PEOPLE_PICKER revoked (principal picking serves per-resource
/// sharing dialogs with no backend; future minimal share is link-based so
/// SHARED_LINKS stays held) — admin section hides; principal search was
/// already denied via never-granted VIEW_* sub-permissions.
#[tokio::test]
async fn roles_grant_user_everything_and_404_unknown() {
    let (app, _pool) = setup().await;

    let response = app.clone().oneshot(get("/api/roles/USER")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let role = body_json(response).await;
    assert_eq!(role["name"], "USER");
    let expected = ["BOOKMARKS", "SHARED_LINKS"];
    let permissions = role["permissions"].as_object().unwrap();
    assert_eq!(permissions.len(), expected.len(), "exact permission set");
    for permission_type in expected {
        assert_eq!(role["permissions"][permission_type]["USE"], true, "for {permission_type}");
    }

    let response = app.oneshot(get("/api/roles/BOGUS")).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body_json(response).await;
    assert_eq!(body["code"], "not_found");
}

/// Conversations must carry a client-known `endpointType` fallback schema:
/// without it `buildDefaultConvo -> parseConvo` throws `Unknown endpoint`
/// and chat boot dies with an empty main panel.
#[tokio::test]
async fn convos_carry_openai_endpoint_type_fallback() {
    let (app, _pool) = setup().await;
    let response = app.clone().oneshot(get("/api/convos")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let list = body_json(response).await;
    assert_eq!(list["conversations"].as_array().unwrap().len(), 0);

    // Create one via chat, then read it back through the sidebar shape.
    let payload = serde_json::json!({
        "conversationId": "new",
        "parentMessageId": NO_PARENT,
        "text": "What is a Prime Number?",
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let events = sse_data(&body_string(response).await);
    let conversation_id = events
        .iter()
        .find(|e| e["created"] == true)
        .and_then(|e| e["message"]["conversationId"].as_str())
        .unwrap()
        .to_string();

    let response = app.oneshot(get(&format!("/api/convos/{conversation_id}"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let convo = body_json(response).await;
    assert_eq!(convo["endpoint"], "knowledgeable");
    assert_eq!(convo["endpointType"], "custom");
}

/// v2 generation protocol (T12a): the POST start handshake returns a JSON
/// ticket instead of SSE when the client negotiates v2 via body or header.
#[tokio::test]
async fn v2_start_returns_json_ticket() {
    for with_header in [false, true] {
        let (app, _pool) = setup().await;
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/agents/chat/knowledgeable")
            .header("content-type", "application/json");
        if with_header {
            builder = builder.header("X-LibreChat-Generation-Protocol", "2");
        }
        let payload = if with_header {
            serde_json::json!({ "conversationId": "new", "text": "hello" })
        } else {
            serde_json::json!({
                "conversationId": "new",
                "text": "hello",
                "generationProtocolVersion": 2,
            })
        };
        let response =
            app.oneshot(builder.body(Body::from(payload.to_string())).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "header={with_header}");
        assert!(
            !response
                .headers()
                .get("content-type")
                .is_some_and(|v| v.as_bytes().starts_with(b"text/event-stream")),
            "v2 start must be JSON, not SSE (header={with_header})"
        );
        let ticket = body_json(response).await;
        assert_eq!(ticket["generationProtocolVersion"], 2, "header={with_header}");
        assert_eq!(ticket["status"], "stream", "header={with_header}");
        assert!(ticket["streamId"].is_string(), "header={with_header}");
        assert!(ticket["conversationId"].is_string(), "header={with_header}");
        assert!(ticket["generationCreatedAt"].is_number(), "header={with_header}");
    }
}

/// v2 stream attach (T12a): `GET .../stream/:id` replays the snapshot and
/// forwards live frames through `final`, including `concept_annotations`.
#[tokio::test]
async fn v2_stream_serves_turn_through_final() {
    let (app, pool) = setup().await;

    let node = domain::ConceptNode {
        id: uuid::Uuid::new_v4(),
        canonical_name: "Prime Number".into(),
        canonical_statement: "Prime Number statement.".into(),
        learner_statement: None,
        world_confidence: 1.0,
        status: domain::ConceptStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    infrastructure::graph_repo::create_concept(&pool, &node).await.unwrap();

    let payload = serde_json::json!({
        "conversationId": "new",
        "parentMessageId": NO_PARENT,
        "text": "What is a Prime Number?",
        "generationProtocolVersion": 2,
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let ticket = body_json(response).await;
    let stream_id = ticket["streamId"].as_str().unwrap().to_string();

    let response = app.oneshot(get(&format!("/api/agents/chat/stream/{stream_id}"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let events = sse_data(&body_string(response).await);
    assert!(events.iter().any(|e| e["created"] == true), "missing created");
    assert!(
        events.iter().any(|e| e.get("concept_annotations").is_some()),
        "missing annotations frame"
    );
    assert!(events.iter().any(|e| e["final"] == true), "missing final");
}

/// Unknown stream ids 404 with the stable envelope.
#[tokio::test]
async fn v2_stream_unknown_id_is_not_found() {
    let (app, _pool) = setup().await;
    let response = app.oneshot(get("/api/agents/chat/stream/does-not-exist")).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body_json(response).await;
    assert_eq!(body["code"], "not_found");
}

/// M10/H2: the built web client is served with an `index.html` fallback for
/// SPA routes, while API routes keep precedence. A missing build explains
/// itself instead of 500ing.
#[tokio::test]
async fn static_serving_serves_app_and_falls_back_for_spa() {
    let dir =
        std::env::temp_dir().join(format!("knowledgeable-dist-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("index.html"), "<html>app</html>").unwrap();
    std::fs::write(dir.join("assets/app.js"), "console.log(1)").unwrap();

    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
    application::conversation_service::ensure_default_learner(&pool).await.unwrap();
    let app = crate::routes::create_router(AppState {
        pool: Some(pool),
        llm: std::sync::Arc::new(llm::FakeLlmClient::new("test")),
        version: "test".into(),
        streams: crate::routes::new_registry(),
        web_dist_dir: Some(dir.clone()),
    });

    let response = app.clone().oneshot(get("/")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_string(response).await.contains("app"));

    let response = app.clone().oneshot(get("/assets/app.js")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_string(response).await.contains("console.log(1)"));

    // SPA route resolves to the app shell, not a 404.
    let response = app.clone().oneshot(get("/c/new")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_string(response).await.contains("app"));

    // API routes keep precedence over static files.
    let response = app.oneshot(get("/api/config")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    std::fs::remove_dir_all(&dir).unwrap();
}

/// M10/H2: without a build, unknown paths explain the missing client.
#[tokio::test]
async fn static_serving_missing_build_explains_itself() {
    let (app, _pool) = setup().await;
    let response = app.oneshot(get("/some-client-route")).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(response).await["code"], "web_build_missing");
}

/// M10 regression: unknown API paths must stay JSON 404s even when a web
/// build is mounted — the SPA fallback serves HTML only for non-API paths.
/// (A 200 HTML here once poisoned the projects list with undefined entries
/// and crashed boot with `Cannot read properties of undefined`.)
#[tokio::test]
async fn unknown_api_paths_stay_json_404_with_build_mounted() {
    let dir =
        std::env::temp_dir().join(format!("knowledgeable-dist-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<html>app</html>").unwrap();

    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
    let app = crate::routes::create_router(AppState {
        pool: Some(pool),
        llm: std::sync::Arc::new(llm::FakeLlmClient::new("test")),
        version: "test".into(),
        streams: crate::routes::new_registry(),
        web_dist_dir: Some(dir.clone()),
    });

    for uri in ["/api/projects", "/api/anything-missing", "/v1/nope"] {
        let response = app.clone().oneshot(get(uri)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "for {uri}");
        assert_eq!(body_json(response).await["code"], "not_found", "for {uri}");
    }
    // ...while real client routes still serve the shell.
    let response = app.oneshot(get("/c/new")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    std::fs::remove_dir_all(&dir).unwrap();
}

/// M3/F1: `/api/models` is truthful — with no provider keys only the
/// offline fallback is advertised. Env is restored afterwards so parallel
/// tests never observe the scrubbed state.
#[tokio::test]
async fn models_advertise_only_fallback_without_keys() {
    struct Guard {
        gemini: Option<String>,
        openai: Option<String>,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            if let Some(v) = self.gemini.take() {
                std::env::set_var("GEMINI_API_KEY", v);
            }
            if let Some(v) = self.openai.take() {
                std::env::set_var("OPENAI_API_KEY", v);
            }
        }
    }
    let _guard = Guard {
        gemini: std::env::var("GEMINI_API_KEY").ok(),
        openai: std::env::var("OPENAI_API_KEY").ok(),
    };
    std::env::remove_var("GEMINI_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");

    let (app, _pool) = setup().await;
    let response = app.oneshot(get("/api/models")).await.unwrap();
    let models = body_json(response).await;
    assert_eq!(models["knowledgeable"], serde_json::json!(["local-tutor"]));
}

/// M3/F2+F4: model dispatch. `local-tutor` turns run offline; a provider
/// model without any key is an explicit 400; a request BYOK key is accepted
/// for dispatch (provider auth happens downstream, still terminating cleanly).
#[tokio::test]
async fn chat_dispatches_on_model_and_byok() {
    let gemini = std::env::var("GEMINI_API_KEY").ok();
    let openai = std::env::var("OPENAI_API_KEY").ok();
    std::env::remove_var("GEMINI_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");

    let (app, _pool) = setup().await;

    // local-tutor always works.
    let payload = serde_json::json!({
        "conversationId": "new",
        "parentMessageId": NO_PARENT,
        "text": "hello",
        "model": "local-tutor",
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let events = sse_data(&body_string(response).await);
    assert!(events.iter().any(|e| e["final"] == true));

    // Provider model with no key anywhere: explicit 400, stable envelope.
    let payload = serde_json::json!({
        "conversationId": "new",
        "text": "hello",
        "model": "gemini-probe-123",
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = body_json(response).await;
    assert_eq!(body["code"], "validation_failed");

    // Request BYOK key: dispatch accepts (provider rejects downstream, and
    // the turn still terminates with a flagged final instead of hanging).
    let payload = serde_json::json!({
        "conversationId": "new",
        "text": "hello",
        "model": "gemini-probe-123",
        "apiKey": "bogus-byok-key",
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let events = sse_data(&body_string(response).await);
    let final_event = events.iter().find(|e| e["final"] == true).expect("byok turn must terminate");
    assert_eq!(final_event["responseMessage"]["error"], true);

    if let Some(v) = gemini {
        std::env::set_var("GEMINI_API_KEY", v);
    }
    if let Some(v) = openai {
        std::env::set_var("OPENAI_API_KEY", v);
    }
}

/// A rejected turn (missing provider key) must not persist an empty
/// conversation shell: credential validation precedes any database write.
#[tokio::test]
async fn chat_rejected_turn_persists_no_conversation() {
    let gemini = std::env::var("GEMINI_API_KEY").ok();
    let openai = std::env::var("OPENAI_API_KEY").ok();
    std::env::remove_var("GEMINI_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");

    let (app, pool) = setup().await;
    let count = || async {
        application::conversation_service::list_conversations(&pool).await.unwrap().len()
    };
    assert_eq!(count().await, 0);

    let payload = serde_json::json!({
        "conversationId": "new",
        "text": "hello",
        "model": "gemini-probe-123",
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(count().await, 0);

    if let Some(v) = gemini {
        std::env::set_var("GEMINI_API_KEY", v);
    }
    if let Some(v) = openai {
        std::env::set_var("OPENAI_API_KEY", v);
    }
}

/// v2 status (T12a): running while the turn streams, complete after `final`,
/// unknown conversations 404. This is what authorizes terminal teardown.
#[tokio::test]
async fn v2_status_tracks_turn_lifecycle() {
    let (app, _pool) = setup().await;

    let payload = serde_json::json!({
        "conversationId": "new",
        "parentMessageId": NO_PARENT,
        "text": "What is a Prime Number?",
        "generationProtocolVersion": 2,
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let ticket = body_json(response).await;
    let conversation_id = ticket["conversationId"].as_str().unwrap().to_string();

    let response = app
        .clone()
        .oneshot(get(&format!("/api/agents/chat/status/{conversation_id}")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let status = body_json(response).await;
    assert_eq!(status["generationProtocolVersion"], 2);
    // The fake streams for ~a second; either state is acceptable mid-flight,
    // but the shape must authorize correctly once complete.
    assert!(status["active"].is_boolean());

    // Drain the stream to completion, then teardown must be authorized.
    let stream_id = ticket["streamId"].as_str().unwrap().to_string();
    let response =
        app.clone().oneshot(get(&format!("/api/agents/chat/stream/{stream_id}"))).await.unwrap();
    let events = sse_data(&body_string(response).await);
    assert!(events.iter().any(|e| e["final"] == true));

    let response = app
        .clone()
        .oneshot(get(&format!("/api/agents/chat/status/{conversation_id}")))
        .await
        .unwrap();
    let status = body_json(response).await;
    assert_eq!(status["active"], false);
    assert_eq!(status["status"], "complete");
    let missing = uuid::Uuid::new_v4().to_string();
    let response = app.oneshot(get(&format!("/api/agents/chat/status/{missing}"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// M8/G3: `GET /api/concepts/:id/wiki` serves cached pages, distinguishes
/// readiness, and validates ids with the stable envelope.
#[tokio::test]
async fn wiki_serves_cached_page_and_readiness_states() {
    let (app, pool) = setup().await;
    let learner = application::conversation_service::ensure_default_learner(&pool).await.unwrap();

    async fn seed_concept(pool: &sqlx::SqlitePool, name: &str, confidence: Option<f32>) -> String {
        let id = uuid::Uuid::new_v4();
        let node = domain::ConceptNode {
            id,
            canonical_name: name.into(),
            canonical_statement: format!("{name} statement."),
            learner_statement: None,
            world_confidence: 1.0,
            status: domain::ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        infrastructure::graph_repo::create_concept(pool, &node).await.unwrap();
        if let Some(conf) = confidence {
            let learner =
                application::conversation_service::ensure_default_learner(pool).await.unwrap();
            sqlx::query(
                "INSERT INTO learner_concept_states (learner_id, concept_id, learner_confidence) VALUES (?, ?, ?)",
            )
            .bind(learner.to_string())
            .bind(id.to_string())
            .bind(conf)
            .execute(pool)
            .await
            .unwrap();
        }
        id.to_string()
    }

    // Malformed id -> 400.
    let response = app.clone().oneshot(get("/api/concepts/not-a-uuid/wiki")).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(response).await["code"], "validation_failed");

    // Unknown concept -> 404 not_found.
    let missing = uuid::Uuid::new_v4().to_string();
    let response =
        app.clone().oneshot(get(&format!("/api/concepts/{missing}/wiki"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(response).await["code"], "not_found");

    // Below mastery without a page -> 404 wiki_not_ready.
    let weak = seed_concept(&pool, "Weak Wiki Concept", Some(0.3)).await;
    let response = app.clone().oneshot(get(&format!("/api/concepts/{weak}/wiki"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(response).await["code"], "wiki_not_ready");

    // Cached page serves without invoking any LLM.
    let strong = seed_concept(&pool, "Strong Wiki Concept", Some(0.98)).await;
    sqlx::query(
        "INSERT INTO concept_wiki_pages (id, learner_id, concept_id, title, summary, personalized_content, known_prerequisites, related_concepts, learner_confidence_at_generation, version, is_stale)
         VALUES (?, ?, ?, 'Strong Wiki Concept', 'Cached summary.', 'Cached content.', '[]', '[]', 0.98, 1, 0)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(learner.to_string())
    .bind(&strong)
    .execute(&pool)
    .await
    .unwrap();
    let response = app.oneshot(get(&format!("/api/concepts/{strong}/wiki"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let page = body_json(response).await;
    assert_eq!(page["title"], "Strong Wiki Concept");
    assert_eq!(page["summary"], "Cached summary.");
    assert_eq!(page["version"], 1);
    assert_eq!(page["is_stale"], false);
}
/// M4/B3: tool execution surfaces `tool_progress` start/finish frames around
/// the call so the client can render tutor activity without breaking text.
#[tokio::test]
async fn chat_stream_emits_tool_progress_frames() {
    let concept_id = uuid::Uuid::new_v4();
    let node = domain::ConceptNode {
        id: concept_id,
        canonical_name: "Tool Progress Concept".into(),
        canonical_statement: "Tool Progress Concept statement.".into(),
        learner_statement: None,
        world_confidence: 1.0,
        status: domain::ConceptStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let stub = std::sync::Arc::new(llm::StubLlmClient::with_tools(vec![(
        "get_dependencies",
        serde_json::json!({ "concept_id": concept_id.to_string() }),
    )]));
    let (app, pool) = setup_with_llm(stub).await;
    infrastructure::graph_repo::create_concept(&pool, &node).await.unwrap();

    let payload = serde_json::json!({
        "conversationId": "new",
        "parentMessageId": NO_PARENT,
        "text": "What depends on this?",
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let events = sse_data(&body_string(response).await);

    let progress: Vec<&serde_json::Value> =
        events.iter().filter(|e| e.get("tool_progress").is_some()).collect();
    assert_eq!(progress.len(), 2, "expected start+finish, got {progress:?}");
    assert_eq!(progress[0]["tool_progress"]["phase"], "started");
    assert_eq!(progress[0]["tool_progress"]["tool_name"], "get_dependencies");
    assert_eq!(progress[1]["tool_progress"]["phase"], "finished");
    assert_eq!(progress[0]["tool_progress"]["call_id"], progress[1]["tool_progress"]["call_id"]);
    let final_pos = events.iter().position(|e| e["final"] == true).unwrap();
    let finish_pos = events.iter().position(|e| e == progress[1]).unwrap();
    assert!(finish_pos < final_pos, "progress must precede final");
}

/// Concept search (graph explorer picker): substring hits, empty query
/// returns [], results carry id + names for loading neighborhoods.
#[tokio::test]
async fn concept_search_finds_by_name_or_statement() {
    let (app, pool) = setup().await;
    for name in ["Prime Number", "Composite Number"] {
        let node = domain::ConceptNode {
            id: uuid::Uuid::new_v4(),
            canonical_name: name.into(),
            canonical_statement: format!("{name} statement."),
            learner_statement: None,
            world_confidence: 1.0,
            status: domain::ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        infrastructure::graph_repo::create_concept(&pool, &node).await.unwrap();
    }

    let response = app.clone().oneshot(get("/api/concepts/search?q=prime")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let hits = body_json(response).await;
    let hits = hits.as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["canonical_name"], "Prime Number");
    assert!(hits[0]["id"].is_string());

    let response = app.clone().oneshot(get("/api/concepts/search?q=")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await.as_array().unwrap().len(), 0);

    let response = app.oneshot(get("/api/concepts/search?q=zzz-no-match")).await.unwrap();
    assert_eq!(body_json(response).await.as_array().unwrap().len(), 0);
}
