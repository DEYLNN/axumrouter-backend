use std::sync::Arc;
use axum::extract::{State, Query};
use axum::response::Redirect;
use axum::{Json};
use crate::state::AppState;

pub async fn start() -> Json<crate::providers::openai_codex::oauth::OAuthStartResponse> {
    Json(crate::providers::openai_codex::oauth::start().await)
}

pub async fn exchange(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Redirect, (axum::http::StatusCode, String)> {
    let code = params.get("code").cloned().unwrap_or_default();
    let oauth_state = params.get("state").cloned().unwrap_or_default();
    let token = crate::providers::openai_codex::oauth::exchange_code(&code, &oauth_state).await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e))?;
    crate::providers::openai_codex::oauth::save_token(&state, &token).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Redirect::to("/admin/auth-files"))
}

pub async fn manual(
    State(state): State<Arc<AppState>>,
    Json(body): Json<crate::providers::openai_codex::oauth::ManualCodeRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    match crate::providers::openai_codex::oauth::exchange_code(&body.code, &body.state.unwrap_or_default()).await {
        Ok(token) => {
            crate::providers::openai_codex::oauth::save_token(&state, &token).await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
            Ok(Json(serde_json::json!({"success":true,"message":"Codex OAuth connected"})))
        }
        Err(e) => Ok(Json(serde_json::json!({"success":false,"error":e}))),
    }
}
