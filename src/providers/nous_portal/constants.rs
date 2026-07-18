pub const PROVIDER_ID: &str = "np";
pub const PROVIDER_NAME: &str = "Nous Portal";
pub const CATEGORY: &str = "oauth";
pub const COLOR: &str = "#2563EB";
pub const ICON_NAME: &str = "np.png";

pub const PORTAL_URL: &str = "https://portal.nousresearch.com";
pub const INFERENCE_URL: &str = "https://inference-api.nousresearch.com/v1";
pub const CLIENT_ID: &str = "hermes-cli";
pub const SCOPE: &str = "inference:invoke";

pub const DEVICE_CODE_URL: &str = "https://portal.nousresearch.com/api/oauth/device/code";
pub const TOKEN_URL: &str = "https://portal.nousresearch.com/api/oauth/token";

pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

use crate::engine::openai_compat::config::ModelDef;

pub fn models() -> Vec<ModelDef> {
    vec![
        ModelDef::new("stepfun/step-3.7-flash:free", "StepFun Step 3.7 Flash Free", 256000, false, true),
        ModelDef::new("tencent/hy3:free", "Tencent Hy3 Free", 260000, false, true),
    ]
}