use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbAuthCredentials {
    pub access_token: String,
    pub email: Option<String>,
    pub account_id: Option<String>,
    pub user_id: Option<String>,
    pub fingerprint_id: Option<String>,
    pub fingerprint_hash: Option<String>,
}
