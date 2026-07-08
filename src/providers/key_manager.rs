use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::db::models::ApiKey;
use crate::error::GatewayError;

/// Locked key info with per-key expiry and backoff tracking
pub struct LockedKey {
    pub key_id: String,
    pub locked_at: Instant,
    /// Explicit expiry — overrides global cooldown_secs when set
    pub locked_until: Option<Instant>,
    pub reason: String,
    /// Exponential backoff level (429 only). Reset on success.
    pub backoff_level: u32,
}

/// Key cooldown lock config — mirrors 9router's ERROR_RULES model
pub struct KeyLockConfig {
    /// Auth failures (401, 403): fixed cooldown
    pub auth_cooldown_secs: u64,
    /// Rate limit (429): exponential backoff base in seconds
    pub rate_limit_backoff_base: u64,
    /// Rate limit: max backoff in seconds
    pub rate_limit_backoff_max: u64,
    /// Transient errors (5xx, etc.): fixed cooldown
    pub transient_cooldown_secs: u64,
}

impl Default for KeyLockConfig {
    fn default() -> Self {
        Self {
            // 2m — like 9router COOLDOWN.long
            auth_cooldown_secs: 120,
            // 90s base — long enough to be visible in UI and avoid retry storms.
            // 9router uses exponential backoff; this keeps same pattern with bigger provider-safe base.
            rate_limit_backoff_base: 90,
            // 5m — like 9router BACKOFF_CONFIG.max=5*60*1000
            rate_limit_backoff_max: 300,
            // 30s — like 9router TRANSIENT_COOLDOWN_MS=30000
            transient_cooldown_secs: 30,
        }
    }
}

/// Modular key management: round-robin distribution with cooldown lock failover.
/// Provider-agnostic — any multi-key provider can use this.
pub struct KeyManager {
    keys: Vec<ApiKey>,
    cursor: AtomicUsize,
    locked: Mutex<HashMap<String, LockedKey>>,
    config: KeyLockConfig,
}

impl KeyManager {
    pub fn new(keys: Vec<ApiKey>) -> Self {
        Self {
            keys,
            cursor: AtomicUsize::new(0),
            locked: Mutex::new(HashMap::new()),
            config: KeyLockConfig::default(),
        }
    }

    pub fn with_config(keys: Vec<ApiKey>, config: KeyLockConfig) -> Self {
        Self {
            keys,
            cursor: AtomicUsize::new(0),
            locked: Mutex::new(HashMap::new()),
            config,
        }
    }

    /// Returns next active key (round-robin, skipping locked keys).
    fn next_active(&self) -> Result<&ApiKey, GatewayError> {
        let mut locked = self.locked.lock().unwrap();
        let now = Instant::now();

        // Garbage-collect expired locks
        locked.retain(|_, lk| {
            match lk.locked_until {
                Some(expiry) => now < expiry,
                None => now.duration_since(lk.locked_at).as_secs() < self.config.auth_cooldown_secs,
            }
        });

        // Filter out currently locked keys
        let active: Vec<&ApiKey> = self
            .keys
            .iter()
            .filter(|k| !locked.contains_key(&k.id))
            .collect();

        if active.is_empty() {
            return Err(GatewayError::no_available_keys(
                "No active API keys — all keys in cooldown lock".to_string(),
            ));
        }

        let index = self.cursor.fetch_add(1, Ordering::Relaxed) % active.len();
        Ok(active[index])
    }

    /// Returns next key for usage. External wrapper for provider use.
    pub fn next(&self) -> Result<&ApiKey, GatewayError> {
        self.next_active()
    }

    /// Lock a key with full error classification (like 9router's checkFallbackError + applyErrorState).
    /// Determines cooldown from error status + backoff level. Text matching for rate-limit keywords.
    pub fn lock_key(&self, key_id: &str, status: u16, reason: String) {
        let mut locked = self.locked.lock().unwrap();
        let now = Instant::now();

        let (cooldown_secs, new_backoff) = match status {
            401 | 403 => (self.config.auth_cooldown_secs, 0u32),
            429 => {
                // Exponential backoff: level increments with each consecutive 429
                let current_level = locked
                    .get(key_id)
                    .map(|lk| lk.backoff_level)
                    .unwrap_or(0);
                let level = current_level.min(15); // maxLevel = 15 like 9router
                let cooldown = (self.config.rate_limit_backoff_base as u64)
                    .saturating_mul(1u64 << level) // 2^level
                    .min(self.config.rate_limit_backoff_max);
                (cooldown, level + 1)
            }
            _ => (self.config.transient_cooldown_secs, 0u32),
        };

        let locked_until = now
            .checked_add(Duration::from_secs(cooldown_secs))
            .unwrap_or(now + Duration::from_secs(30));

        locked.insert(
            key_id.to_string(),
            LockedKey {
                key_id: key_id.to_string(),
                locked_at: now,
                locked_until: Some(locked_until),
                reason: format!(
                    "HTTP {} — {} (cooldown {}s, backoff_level={})",
                    status,
                    reason.split('{').next().unwrap_or(&reason).trim(),
                    cooldown_secs, new_backoff
                ),
                backoff_level: new_backoff,
            },
        );

        // Also lock duplicates (same key_value)
        let key_value = self
            .keys
            .iter()
            .find(|k| k.id == key_id)
            .map(|k| k.key_value.clone());

        if let Some(val) = key_value {
            for k in &self.keys {
                if k.key_value == val && k.id != key_id {
                    locked.insert(
                        k.id.clone(),
                        LockedKey {
                            key_id: k.id.clone(),
                            locked_at: now,
                            locked_until: Some(locked_until),
                            reason: format!("duplicate of locked key {}", key_id),
                            backoff_level: new_backoff,
                        },
                    );
                }
            }
        }

        // Count active WITHOUT calling active_count() to avoid deadlock
        let active_remain = self.keys.iter().filter(|k| !locked.contains_key(&k.id)).count();

        tracing::warn!(
            "Key '{}' locked for {}s (backoff_level={}), {} active keys remain",
            key_id,
            cooldown_secs,
            new_backoff,
            active_remain
        );
    }

    /// Reset key state after success (like 9router's resetAccountState).
    /// Clears lock + backoff level so key is immediately available.
    pub fn unlock(&self, key_id: &str) {
        let mut locked = self.locked.lock().unwrap();
        locked.remove(key_id);
    }

    /// Reset all locks.
    pub fn reset_locks(&self) {
        let mut locked = self.locked.lock().unwrap();
        locked.clear();
    }

    /// Get current auth cooldown duration in seconds.
    pub fn cooldown_secs(&self) -> u64 {
        self.config.auth_cooldown_secs
    }

    /// Number of active (non-locked) keys.
    pub fn active_count(&self) -> usize {
        let locked = self.locked.lock().unwrap();
        let now = Instant::now();
        self.keys
            .iter()
            .filter(|k| {
                locked
                    .get(&k.id)
                    .map(|lk| {
                        match lk.locked_until {
                            Some(expiry) => now >= expiry,
                            None => now.duration_since(lk.locked_at).as_secs() >= self.config.auth_cooldown_secs,
                        }
                    })
                    .unwrap_or(true)
            })
            .count()
    }

    /// Total keys configured.
    pub fn total_count(&self) -> usize {
        self.keys.len()
    }

    /// Current key IDs managed.
    pub fn key_ids(&self) -> Vec<String> {
        self.keys.iter().map(|k| k.id.clone()).collect()
    }

    /// All currently locked keys with remaining cooldown info.
    pub fn locked_keys(&self) -> Vec<(String, u64, String)> {
        let locked = self.locked.lock().unwrap();
        let now = Instant::now();
        locked
            .iter()
            .filter_map(|(id, lk)| {
                let remaining = match lk.locked_until {
                    Some(expiry) if expiry > now => expiry.duration_since(now).as_secs(),
                    _ => return None,
                };
                Some((id.clone(), remaining, lk.reason.clone()))
            })
            .collect()
    }
}
