use crate::error::GatewayError;

use super::auth::FbAuthCredentials;
use super::client::FbClient;
use super::constants;

// ── Model resolution ──

/// Map gateway model id (e.g. "deepseek-v4-flash") to FreeBuff backend model.
pub fn resolve_backend_model(model_val: &str) -> String {
    let short = model_val.split('/').nth(1).unwrap_or(model_val);
    for m in constants::MODELS {
        if m.id == short {
            return m.backend_model.to_string();
        }
    }
    "deepseek/deepseek-v4-flash".to_string()
}

/// Map backend model to FreeBuff agent ID.
pub fn agent_id_for_model(backend_model: &str) -> &str {
    match backend_model {
        "deepseek/deepseek-v4-flash" => "base2-free-deepseek-flash",
        "deepseek/deepseek-v4-pro" => "base2-free-deepseek",
        "minimax/minimax-m3" => "base2-free-minimax-m3",
        "mimo/mimo-v2.5" => "base2-free-mimo",
        "mimo/mimo-v2.5-pro" => "base2-free-mimo-pro",
        _ => "base2-free-deepseek-flash",
    }
}

// ── Session lifecycle ──

/// Run FreeBuff session lifecycle: validate agents, ensure session, start run + child pruner.
/// Returns (run_id, child_run_id, instance_id).
pub async fn run_lifecycle(
    client: &FbClient,
    cred: &FbAuthCredentials,
    agent_id: &str,
    backend_model: &str,
) -> Result<(String, String, Option<String>), GatewayError> {
    client.validate_agents(cred).await;
    let instance_id = client.ensure_free_session(cred, backend_model).await?;
    let run_id = client.start_run(cred, agent_id).await?;
    let child_run_id = client.start_run(cred, constants::CONTEXT_PRUNER_AGENT_ID).await?;
    client.record_run_step(cred, &child_run_id, 1, &[]).await.ok();
    client.finish_run(cred, &child_run_id).await.ok();
    client.record_run_step(cred, &run_id, 1, &[child_run_id.clone()]).await.ok();
    Ok((run_id, child_run_id, Some(instance_id)))
}
