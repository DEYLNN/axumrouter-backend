use crate::error::GatewayError;
use crate::providers::error_classifier::{classify_provider_error, ClassifiedError};
use crate::providers::key_manager::KeyManager;

/// Unified key lock logic after a provider request error.
/// Classifies error, locks the key with appropriate cooldown, returns classified info.
/// Caller decides whether to retry: `classified.retryable && attempt < total`.
///
/// Usage patterns:
///   // non-streaming (Err without FailedKeyAttempt tracking)
///   Err(e) => {
///       attempt += 1;
///       let c = lock_key_on_error(&self.keys, &key_id, &e);
///       if c.retryable && attempt < total { continue; }
///       return Err(e);
///   }
///
///   // streaming (with FailedKeyAttempt push on retryable)
///   Err(e) => {
///       let c = lock_key_on_error(&self.keys, &key_id, &e);
///       if c.retryable && _attempt + 1 < total.max(1) {
///           failed.push(FailedKeyAttempt { key_id: key_id.clone(), error: e });
///           continue;
///       }
///       return Err(e);
///   }
///
///   // auth build error (always lock + retry immediately)
///   Err(e) => { self.keys.lock_key(&key_id, 400, e.to_string()); continue; }
pub fn lock_key_on_error(
    keys: &KeyManager,
    key_id: &str,
    error: &GatewayError,
) -> ClassifiedError {
    let classified = classify_provider_error(error);
    // Only lock if classifier says to lock (lock_status = Some).
    // Transient/upstream errors (502, 503, overloaded, stream errors) have
    // lock_status = None — key stays available, just skip for this request.
    if let Some(status) = classified.lock_status {
        keys.lock_key(key_id, status, error.to_string());
    }
    classified
}
