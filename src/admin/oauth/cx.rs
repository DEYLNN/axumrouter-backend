use std::sync::Arc;
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::Json;
use crate::state::AppState;

use crate::providers::oauth::codex::oauth::{self, ManualCodeRequest, OAuthStartResponse};

pub async fn start() -> Json<OAuthStartResponse> {
    Json(oauth::start().await)
}

/// Browser redirect callback — code + state from authorize redirect.
pub async fn exchange(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Redirect, (axum::http::StatusCode, String)> {
    let code = params.get("code").cloned().unwrap_or_default();
    let oauth_state = params.get("state").cloned().unwrap_or_default();
    let token = oauth::exchange_code(&code, &oauth_state)
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e))?;
    oauth::save_token(&state, &token)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Redirect::to("/admin/auth-files"))
}

/// Manual code entry (paste the code from the authorize redirect URL).
pub async fn manual(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ManualCodeRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    match oauth::exchange_code(&body.code, &body.state.unwrap_or_default()).await {
        Ok(token) => {
            oauth::save_token(&state, &token)
                .await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
            Ok(Json(serde_json::json!({ "success": true, "message": "Codex OAuth connected" })))
        }
        Err(e) => Ok(Json(serde_json::json!({ "success": false, "error": e }))),
    }
}