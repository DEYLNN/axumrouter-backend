/// Credentials parsed from OAuth token for Kilo Code.
#[derive(Debug, Clone)]
pub struct KcOAuthCredential {
    pub access_token: String,
    pub email: Option<String>,
    pub org_id: Option<String>,
}

impl KcOAuthCredential {
    pub fn parse(kv: &str) -> Result<Self, String> {
        let val: serde_json::Value =
            serde_json::from_str(kv).map_err(|e| format!("KiloCode: invalid key_value JSON: {e}"))?;
        let access_token = val["access_token"]
            .as_str()
            .or_else(|| val["accessToken"].as_str())
            .ok_or_else(|| "KiloCode: missing access_token".to_string())?
            .to_string();
        let email = val["email"].as_str().map(String::from);
        let org_id = val["org_id"]
            .as_str()
            .or_else(|| val["orgId"].as_str())
            .or_else(|| {
                val["providerSpecificData"]
                    .as_object()
                    .and_then(|d| d.get("orgId"))
                    .and_then(|v| v.as_str())
            })
            .map(String::from);
        Ok(Self { access_token, email, org_id })
    }
}
