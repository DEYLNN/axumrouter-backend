use std::sync::Arc;
use axum::{routing::{get, post, delete}, Router, extract::{State, Path}, Json, http::StatusCode};
use serde::Deserialize;
use crate::state::AppState;
use crate::db;

pub fn unsloth_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/unsloth/models", get(api_list_unsloth_models))
        .route("/admin/api/unsloth/models", post(api_create_unsloth_model))
        .route("/admin/api/unsloth/models/:id", get(api_get_unsloth_model))
        .route("/admin/api/unsloth/models/:id", post(api_update_unsloth_model))
        .route("/admin/api/unsloth/models/:id", delete(api_delete_unsloth_model))
        .with_state(state)
}

#[derive(Deserialize)]
struct CreateUnslothReq {
    id: String,
    label: String,
    base_url: String,
    api_key: String,
    upstream_model: String,
    context_length: Option<i64>,
    supports_tools: Option<bool>,
    supports_vision: Option<bool>,
}

#[derive(Deserialize)]
struct UpdateUnslothReq {
    label: String,
    base_url: String,
    api_key: String,
    upstream_model: String,
    context_length: Option<i64>,
    supports_tools: Option<bool>,
    supports_vision: Option<bool>,
    is_active: Option<bool>,
}

async fn api_list_unsloth_models(State(state): State<Arc<AppState>>) -> Result<Json<Vec<db::UnslothModelRow>>, (StatusCode, String)> {
    let rows = db::list_unsloth_models(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows))
}

async fn api_get_unsloth_model(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<Option<db::UnslothModelRow>>, (StatusCode, String)> {
    let row = db::get_unsloth_model(&state.db, &id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(row))
}

async fn api_create_unsloth_model(State(state): State<Arc<AppState>>, Json(req): Json<CreateUnslothReq>) -> Result<StatusCode, (StatusCode, String)> {
    db::create_unsloth_model(
        &state.db, &req.id, &req.label, &req.base_url, &req.api_key,
        &req.upstream_model, req.context_length.unwrap_or(128000),
        req.supports_tools.unwrap_or(false), req.supports_vision.unwrap_or(false),
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::CREATED)
}

async fn api_update_unsloth_model(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<UpdateUnslothReq>) -> Result<StatusCode, (StatusCode, String)> {
    let updated = db::update_unsloth_model(
        &state.db, &id, &req.label, &req.base_url, &req.api_key,
        &req.upstream_model, req.context_length.unwrap_or(128000),
        req.supports_tools.unwrap_or(false), req.supports_vision.unwrap_or(false),
        req.is_active.unwrap_or(true),
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if updated { Ok(StatusCode::OK) } else { Ok(StatusCode::NOT_FOUND) }
}

async fn api_delete_unsloth_model(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = db::delete_unsloth_model(&state.db, &id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if deleted { Ok(StatusCode::OK) } else { Ok(StatusCode::NOT_FOUND) }
}
