// MiMo Code Free HTTP client — JWT bootstrap + anti-abuse headers
use std::sync::Mutex;
use std::time::{Duration, Instant};
use rand::Rng;

use super::constants;

const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
];

const MIMO_SYSTEM_MARKER: &str =
    "You are MiMoCode, an interactive CLI tool that helps users with software engineering tasks.";

pub struct McfClient {
    http: reqwest::Client,
    jwt: Mutex<(Option<String>, Instant)>, // (jwt, expires_at)
    session_id: String,
}

impl McfClient {
    pub fn new(timeout_secs: u64) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();
        let session_id = format!("ses_{}", uuid::Uuid::new_v4().to_string().replace('-', "").chars().take(24).collect::<String>());
        Self { http, jwt: Mutex::new((None, Instant::now())), session_id }
    }

    fn random_ua() -> &'static str {
        let idx = rand::thread_rng().gen_range(0..USER_AGENTS.len());
        USER_AGENTS[idx]
    }

    fn generate_fingerprint() -> String {
        let seed = format!("mcf-{}-{}", std::process::id(), rand::random::<u64>());
        use sha2::{Sha256, Digest};
        format!("{:x}", Sha256::digest(seed.as_bytes()))
    }

    async fn bootstrap_jwt(&self) -> Result<String, String> {
        // Check cache
        {
            let cache = self.jwt.lock().unwrap();
            if let (Some(ref jwt), expires) = &*cache {
                if Instant::now() < *expires - Duration::from_secs(300) {
                    return Ok(jwt.clone());
                }
            }
        }

        let resp = self.http
            .post(constants::BOOTSTRAP_URL)
            .header("Content-Type", "application/json")
            .header("User-Agent", Self::random_ua())
            .json(&serde_json::json!({ "client": Self::generate_fingerprint() }))
            .send()
            .await
            .map_err(|e| format!("MiMo bootstrap failed: {}", e))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let data: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!("MiMo bootstrap parse error (status {}): {} — body: {}", status.as_u16(), e, &text[..text.len().min(200)]))?;

        let jwt = data["jwt"].as_str().ok_or_else(|| {
            let snippet = &text[..text.len().min(200)];
            format!("No JWT in bootstrap response (status {}): {}", status.as_u16(), snippet)
        })?.to_string();

        // Parse expiry from JWT payload
        let expires = Self::parse_jwt_exp(&jwt);

        let mut cache = self.jwt.lock().unwrap();
        *cache = (Some(jwt.clone()), expires);

        Ok(jwt)
    }

    fn parse_jwt_exp(jwt: &str) -> Instant {
        let parts: Vec<&str> = jwt.split('.').collect();
        if parts.len() != 3 { return Instant::now() + Duration::from_secs(3000); }
        let payload = base64_decode_url(parts[1]);
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&payload) {
            if let Some(exp) = v["exp"].as_u64() {
                return Instant::now() + Duration::from_secs(exp.saturating_sub(
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
                ));
            }
        }
        Instant::now() + Duration::from_secs(3000)
    }

    pub async fn chat(&self, mut body: serde_json::Value) -> Result<reqwest::Response, String> {
        let jwt = self.bootstrap_jwt().await?;

        // Inject anti-abuse system message if missing
        inject_system_marker(&mut body);

        let is_stream = body.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);
        let accept = if is_stream { "text/event-stream" } else { "application/json" };
        let sid = self.session_id.clone();

        let do_request = |jwt: String| {
            let sid = sid.clone();
            let accept = accept;
            let http = &self.http;
            let body = body.clone();
            async move {
                http.post(constants::BASE_URL)
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {}", jwt))
                    .header("X-Mimo-Source", "mimocode-cli-free")
                    .header("x-session-affinity", &sid)
                    .header("User-Agent", Self::random_ua())
                    .header("Accept", accept)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("MiMo chat failed: {}", e))
            }
        };

        let resp = do_request(jwt.clone()).await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let is_441 = status.as_u16() == 441
            || text.contains("\"code\":441")
            || text.contains("\"code\":\"441\"");

        // On auth failure, invalidate cache and retry once
        if status.as_u16() == 401 || status.as_u16() == 403 {
            {
                let mut cache = self.jwt.lock().unwrap();
                *cache = (None, Instant::now());
            }
            let jwt = self.bootstrap_jwt().await?;
            return do_request(jwt).await;
        }

        // On 441 risk control (HTTP 441 or body code:441), wait and retry once
        if is_441 {
            tokio::time::sleep(Duration::from_secs(15)).await;
            let jwt = self.bootstrap_jwt().await?;
            let retry = do_request(jwt).await?;
            let rs = retry.status();
            let rt = retry.text().await.unwrap_or_default();
            if rs.as_u16() == 441 || rt.contains("\"code\":441") || rt.contains("\"code\":\"441\"") {
                return Err(format!("MiMo 441 rate limited — wait a few minutes and retry"));
            }
            let rebuilt = axum::http::Response::builder()
                .status(rs)
                .header("content-type", "application/json")
                .body(rt)
                .map_err(|e| format!("Response build: {}", e))?;
            return Ok(reqwest::Response::from(rebuilt));
        }

        // On auth failure, invalidate cache and retry once
        if status.as_u16() == 401 || status.as_u16() == 403 {
            {
                let mut cache = self.jwt.lock().unwrap();
                *cache = (None, Instant::now());
            }
            let jwt = self.bootstrap_jwt().await?;
            return do_request(jwt).await;
        }

        // Rebuild response from saved text
        let rebuilt = axum::http::Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(text)
            .map_err(|e| format!("Response build: {}", e))?;
        Ok(reqwest::Response::from(rebuilt))
    }
}

fn inject_system_marker(body: &mut serde_json::Value) {
    let messages = match body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        Some(m) => m,
        None => return,
    };
    let has_marker = messages.iter().any(|m| {
        m.get("role").and_then(|r| r.as_str()) == Some("system")
            && m.get("content").and_then(|c| c.as_str()).map_or(false, |c| c.contains(MIMO_SYSTEM_MARKER))
    });
    if has_marker { return; }
    messages.insert(0, serde_json::json!({
        "role": "system",
        "content": MIMO_SYSTEM_MARKER
    }));
}

fn base64_decode_url(input: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.decode(input.as_bytes())
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        .unwrap_or_default()
}
