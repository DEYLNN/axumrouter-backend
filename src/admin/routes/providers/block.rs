use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct BlockModelRequest {
    pub model: String,
}

#[derive(Serialize)]
pub struct BlockModelResponse {
    pub ok: bool,
    pub message: String,
}

/// POST /admin/api/providers/:id/block
pub async fn api_block_model(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Json(req): Json<BlockModelRequest>,
) -> Json<BlockModelResponse> {
    let model_name = req
        .model
        .strip_prefix(&format!("{}/", provider_id))
        .unwrap_or(&req.model)
        .to_string();
    match crate::db::block_model(&state.db, &provider_id, &model_name).await {
        Ok(_) => Json(BlockModelResponse {
            ok: true,
            message: "Blocked".into(),
        }),
        Err(e) => Json(BlockModelResponse {
            ok: false,
            message: e.to_string(),
        }),
    }
}

/// POST /admin/api/providers/:id/unblock
pub async fn api_unblock_model(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Json(req): Json<BlockModelRequest>,
) -> Json<BlockModelResponse> {
    let model_name = req
        .model
        .strip_prefix(&format!("{}/", provider_id))
        .unwrap_or(&req.model)
        .to_string();
    match crate::db::unblock_model(&state.db, &provider_id, &model_name).await {
        Ok(_) => Json(BlockModelResponse {
            ok: true,
            message: "Unblocked".into(),
        }),
        Err(e) => Json(BlockModelResponse {
            ok: false,
            message: e.to_string(),
        }),
    }
}
