use crate::error::GatewayError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderErrorKind {
    Auth,
    RateLimit,
    Transient,
    Permanent,
}

#[derive(Debug, Clone)]
pub struct ClassifiedError {
    pub kind: ProviderErrorKind,
    pub status: Option<u16>,
    pub retryable: bool,
    pub lock_status: Option<u16>,
}

#[derive(Debug, Clone, Copy)]
struct TextRule {
    needles: &'static [&'static str],
    kind: ProviderErrorKind,
    retryable: bool,
    lock_status: Option<u16>,
}

const TEXT_RULES: &[TextRule] = &[
    TextRule {
        needles: &[
            "invalid api key",
            "incorrect api key",
            "unauthorized",
            "forbidden",
            "authentication failed",
            "permission denied",
        ],
        kind: ProviderErrorKind::Auth,
        retryable: true,
        lock_status: Some(401),
    },
    TextRule {
        needles: &[
            "rate limit",
            "rate_limited",
            "too many requests",
            "quota exceeded",
            "insufficient quota",
            "billing quota",
        ],
        kind: ProviderErrorKind::RateLimit,
        retryable: true,
        lock_status: Some(429),
    },
    TextRule {
        needles: &[
            "overloaded",
            "capacity",
            "temporarily unavailable",
            "timeout",
            "timed out",
            "connection reset",
            "connection closed",
            "connection refused",
            "dns error",
            "network error",
            "service unavailable",
            "bad gateway",
            "gateway timeout",
        ],
        kind: ProviderErrorKind::Transient,
        retryable: true,
        lock_status: Some(503),
    },
];

pub fn classify_provider_error(error: &GatewayError) -> ClassifiedError {
    match error {
        GatewayError::ProviderHttpError { status, body, .. } => classify_http_error(*status, body),
        GatewayError::ProviderError(msg) => classify_text_error(None, msg),
        _ => ClassifiedError::permanent(None),
    }
}

pub fn classify_http_error(status: u16, body: &str) -> ClassifiedError {
    let text_based = classify_text_error(Some(status), body);
    if !matches!(text_based.kind, ProviderErrorKind::Permanent) {
        return text_based;
    }

    match status {
        // 4xx auth/rate errors → always retryable (failover)
        400 | 401 | 402 | 403 | 407 | 429 => {
            ClassifiedError::retryable(ProviderErrorKind::Auth, Some(status), Some(status))
        }
        // 404/410/422 → permanent (invalid request, jangan failover)
        404 | 410 | 422 => ClassifiedError::permanent(Some(status)),
        // 5xx + transient → retryable
        408 | 425 | 500 | 502 | 503 | 504 | 507 | 509 => {
            ClassifiedError::retryable(ProviderErrorKind::Transient, Some(status), Some(status))
        }
        // default 4xx lainnya → treat as auth (failover)
        s if (400..500).contains(&s) => {
            ClassifiedError::retryable(ProviderErrorKind::Auth, Some(status), Some(status))
        }
        _ => ClassifiedError::permanent(Some(status)),
    }
}

pub fn classify_text_error(status: Option<u16>, body: &str) -> ClassifiedError {
    let lower = body.to_ascii_lowercase();

    for rule in TEXT_RULES {
        if rule.needles.iter().any(|needle| lower.contains(needle)) {
            let lock_status = match (rule.kind, rule.lock_status, status) {
                (ProviderErrorKind::Transient, Some(_), Some(s)) => Some(s),
                (_, lock, _) => lock,
            };
            return ClassifiedError {
                kind: rule.kind,
                status,
                retryable: rule.retryable,
                lock_status,
            };
        }
    }

    ClassifiedError::permanent(status)
}

impl ClassifiedError {
    fn permanent(status: Option<u16>) -> Self {
        Self {
            kind: ProviderErrorKind::Permanent,
            status,
            retryable: false,
            lock_status: None,
        }
    }

    fn retryable(kind: ProviderErrorKind, status: Option<u16>, lock_status: Option<u16>) -> Self {
        Self {
            kind,
            status,
            retryable: true,
            lock_status,
        }
    }
}
