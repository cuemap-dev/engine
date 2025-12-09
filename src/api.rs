use crate::engine::CueMapEngine;
use crate::multi_tenant::{MultiTenantEngine, validate_project_id};
use axum::{
    extract::{Path, State},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    routing::{get, patch, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct AddMemoryRequest {
    content: String,
    cues: Vec<String>,
    #[serde(default)]
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
pub struct AddMemoryResponse {
    id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
pub struct RecallRequest {
    cues: Vec<String>,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    auto_reinforce: bool,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Deserialize)]
pub struct ReinforceRequest {
    cues: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ReinforceResponse {
    status: String,
    memory_id: String,
}

#[derive(Clone)]
pub enum EngineState {
    SingleTenant(CueMapEngine),
    MultiTenant(Arc<MultiTenantEngine>),
}

pub fn routes(engine: std::sync::Arc<CueMapEngine>, multi_tenant: bool) -> Router {
    if multi_tenant {
        let mt_engine = Arc::new(MultiTenantEngine::new());
        Router::new()
            .route("/", get(root))
            .route("/memories", post(add_memory_mt))
            .route("/recall", post(recall_mt))
            .route("/memories/:id/reinforce", patch(reinforce_memory_mt))
            .route("/memories/:id", get(get_memory_mt))
            .route("/stats", get(get_stats_mt))
            .route("/projects", get(list_projects))
            .route("/projects/:id", delete(delete_project))
            .with_state(EngineState::MultiTenant(mt_engine))
    } else {
        Router::new()
            .route("/", get(root))
            .route("/memories", post(add_memory))
            .route("/recall", post(recall))
            .route("/memories/:id/reinforce", patch(reinforce_memory))
            .route("/memories/:id", get(get_memory))
            .route("/stats", get(get_stats))
            .with_state(EngineState::SingleTenant(CueMapEngine::clone(&engine)))
    }
}

async fn root() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "CueMap Rust Engine",
        "version": "0.2.1",
        "description": "High-performance Temporal-Associative Memory Store"
    }))
}

async fn add_memory(
    State(state): State<EngineState>,
    Json(req): Json<AddMemoryRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let EngineState::SingleTenant(engine) = state {
        let memory_id = engine.add_memory(req.content, req.cues, req.metadata);
        
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "id": memory_id,
                "status": "stored"
            })),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "id": "",
                "status": "error"
            })),
        )
    }
}

async fn recall(
    State(state): State<EngineState>,
    Json(req): Json<RecallRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let EngineState::SingleTenant(engine) = state {
        let results = engine.recall(req.cues, req.limit, req.auto_reinforce);
        (StatusCode::OK, Json(serde_json::json!({ "results": results })))
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Invalid state"})),
        )
    }
}

async fn reinforce_memory(
    State(state): State<EngineState>,
    Path(memory_id): Path<String>,
    Json(req): Json<ReinforceRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let EngineState::SingleTenant(engine) = state {
        let success = engine.reinforce_memory(&memory_id, req.cues);
        
        if success {
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "reinforced",
                    "memory_id": memory_id
                })),
            )
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "status": "not_found",
                    "memory_id": memory_id
                })),
            )
        }
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "memory_id": ""
            })),
        )
    }
}

async fn get_memory(
    State(state): State<EngineState>,
    Path(memory_id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let EngineState::SingleTenant(engine) = state {
        match engine.get_memory(&memory_id) {
            Some(memory) => (StatusCode::OK, Json(serde_json::json!(memory))),
            None => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Memory not found"})),
            ),
        }
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Invalid state"})),
        )
    }
}

async fn get_stats(State(state): State<EngineState>) -> (StatusCode, Json<serde_json::Value>) {
    if let EngineState::SingleTenant(engine) = state {
        let stats = engine.get_stats();
        (StatusCode::OK, Json(serde_json::Value::Object(stats.into_iter().collect())))
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Invalid state"})),
        )
    }
}

// Multi-tenant handlers
fn extract_project_id(headers: &HeaderMap) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let project_id = headers
        .get("X-Project-ID")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing X-Project-ID header"})),
            )
        })?;
    
    if !validate_project_id(project_id) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid project ID format"})),
        ));
    }
    
    Ok(project_id.to_string())
}

async fn add_memory_mt(
    State(state): State<EngineState>,
    headers: HeaderMap,
    Json(req): Json<AddMemoryRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let project_id = match extract_project_id(&headers) {
        Ok(id) => id,
        Err(e) => return e,
    };
    
    if let EngineState::MultiTenant(mt_engine) = state {
        let engine = mt_engine.get_or_create_project(project_id);
        let memory_id = engine.add_memory(req.content, req.cues, req.metadata);
        
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "id": memory_id,
                "status": "stored"
            })),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "id": "",
                "status": "error"
            })),
        )
    }
}

async fn recall_mt(
    State(state): State<EngineState>,
    headers: HeaderMap,
    Json(req): Json<RecallRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let project_id = match extract_project_id(&headers) {
        Ok(id) => id,
        Err(e) => return e,
    };
    
    if let EngineState::MultiTenant(mt_engine) = state {
        let engine = mt_engine.get_or_create_project(project_id);
        let results = engine.recall(req.cues, req.limit, req.auto_reinforce);
        
        (StatusCode::OK, Json(serde_json::json!({ "results": results })))
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Invalid state"})),
        )
    }
}

async fn reinforce_memory_mt(
    State(state): State<EngineState>,
    headers: HeaderMap,
    Path(memory_id): Path<String>,
    Json(req): Json<ReinforceRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let project_id = match extract_project_id(&headers) {
        Ok(id) => id,
        Err(e) => return e,
    };
    
    if let EngineState::MultiTenant(mt_engine) = state {
        let engine = mt_engine.get_or_create_project(project_id);
        let success = engine.reinforce_memory(&memory_id, req.cues);
        
        if success {
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "reinforced",
                    "memory_id": memory_id
                })),
            )
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "status": "not_found",
                    "memory_id": memory_id
                })),
            )
        }
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "memory_id": ""
            })),
        )
    }
}

async fn get_memory_mt(
    State(state): State<EngineState>,
    headers: HeaderMap,
    Path(memory_id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let project_id = match extract_project_id(&headers) {
        Ok(id) => id,
        Err(e) => return e,
    };
    
    if let EngineState::MultiTenant(mt_engine) = state {
        let engine = mt_engine.get_or_create_project(project_id);
        match engine.get_memory(&memory_id) {
            Some(memory) => (StatusCode::OK, Json(serde_json::json!(memory))),
            None => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Memory not found"})),
            ),
        }
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Invalid state"})),
        )
    }
}

async fn get_stats_mt(
    State(state): State<EngineState>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    let project_id = match extract_project_id(&headers) {
        Ok(id) => id,
        Err(e) => return e,
    };
    
    if let EngineState::MultiTenant(mt_engine) = state {
        let engine = mt_engine.get_or_create_project(project_id);
        let stats = engine.get_stats();
        (StatusCode::OK, Json(serde_json::Value::Object(stats.into_iter().collect())))
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Invalid state"})),
        )
    }
}

async fn list_projects(
    State(state): State<EngineState>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let EngineState::MultiTenant(mt_engine) = state {
        let projects = mt_engine.list_projects();
        (StatusCode::OK, Json(serde_json::json!({ "projects": projects })))
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Not in multi-tenant mode"})),
        )
    }
}

async fn delete_project(
    State(state): State<EngineState>,
    Path(project_id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let EngineState::MultiTenant(mt_engine) = state {
        let deleted = mt_engine.delete_project(&project_id);
        if deleted {
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "deleted", "project_id": project_id})),
            )
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Project not found"})),
            )
        }
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Not in multi-tenant mode"})),
        )
    }
}
