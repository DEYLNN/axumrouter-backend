//! Strip configurable thinking/reasoning tag blocks from model output content.
//!
//! Some upstream providers (e.g. mmx/minimax via Anthropic-format) return their
//! chain-of-thought as **literal text** inside the visible `text` block instead
//! of as a separate `thinking` block. The gateway can't tell those tags apart
//! from legitimate content unless the operator explicitly declares which tag
//! patterns to strip, per model.
//!
//! Default tag patterns cover the common variants seen across providers. The
//! helper is generic — callers supply the pattern list (from `ModelDef`) so
//! adding a new provider with a different convention is a one-line TOML edit.
//!
//! Default patterns (when caller passes empty slice):
//!   - `<ant_thinking>...</ant_thinking>`
//!   - `<thinking>...</thinking>`
//!   - `<reasoning>...</reasoning>`
//!   - `<reflection>...</reflection>`
//!   - `<think>...</think>` (Qwen / DeepSeek plain style)
//!   - `<|thinking|>...<|/thinking|>` (Qwen 3 chatml style)

use regex::Regex;

/// Tag patterns stripped from content when `thinking_tags` is configured for
/// the model. Matched as `<tag>...</tag>` block-replace, multiline.
const DEFAULT_TAGS: &[&str] = &[
    "ant_thinking",
    "thinking",
    "reasoning",
    "reflection",
    "think", // <think> (no underscore — Qwen/DeepSeek style)
];

/// Strip all configured thinking-tag blocks from `content`.
///
/// `patterns` is a list of tag NAMES (without angle brackets). Empty list →
/// returns content unchanged. If `patterns` is empty AND caller wants the
/// defaults, pass `DEFAULT_TAGS` explicitly via the wrapper.
///
/// Matching is greedy and multiline. Outer whitespace is collapsed after strip.
pub fn strip_thinking_tags(content: &str, patterns: &[&str]) -> String {
    if content.is_empty() || patterns.is_empty() {
        return content.to_string();
    }

    let mut out = content.to_string();
    for tag in patterns {
        // Escape user-supplied tag name for use in regex (defensive — even
        // though tag names should be alphanumeric only).
        let escaped = regex::escape(tag);
        // Two patterns per tag: <tag>...</tag>  AND  <|tag|>...<|/tag|>
        // The chatml variant uses pipe-delimited angle brackets.
        for pattern in [
            format!(r"(?s)<{escaped}>.*?</{escaped}>\s*", escaped = escaped),
            format!(r"(?s)<\|{escaped}\|>.*?<\|/{escaped}\|>\s*", escaped = escaped),
        ] {
            if let Ok(re) = Regex::new(&pattern) {
                out = re.replace_all(&out, "").to_string();
            }
        }
    }

    // Collapse 3+ consecutive newlines (common after tag removal) down to 2.
    if let Ok(re) = Regex::new(r"\n{3,}") {
        out = re.replace_all(&out, "\n\n").to_string();
    }

    out.trim().to_string()
}

/// Returns the default tag patterns. Callers should prefer these when the
/// model declares `thinking_tags = true` (boolean shorthand).
pub fn default_patterns() -> &'static [&'static str] {
    DEFAULT_TAGS
}

/// Convenience: strip using `&[String]` from TOML config.
pub fn strip_with_strings(content: &str, patterns: &[String]) -> String {
    if patterns.is_empty() {
        return content.to_string();
    }
    let refs: Vec<&str> = patterns.iter().map(|s| s.as_str()).collect();
    strip_thinking_tags(content, &refs)
}

/// Convenience: strip using `&'static [&'static str]` (compile-time constants).
/// Used by hardcoded-provider constants like Cline where tag list is fixed.
pub fn strip_thinking_tags_const(content: &str, patterns: &'static [&'static str]) -> String {
    strip_thinking_tags(content, patterns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ant_thinking_default_pattern() {
        let input = "<ant_thinking>internal reasoning here</ant_thinking>\n\nvisible answer";
        let out = strip_thinking_tags(input, default_patterns());
        assert_eq!(out, "visible answer");
    }

    #[test]
    fn strips_think_qwen_style() {
        let input = "<think>hidden</think>after";
        let out = strip_thinking_tags(input, &["think"]);
        assert_eq!(out, "after");
    }

    #[test]
    fn empty_patterns_passthrough() {
        let input = "<thinking>x</thinking>";
        assert_eq!(strip_thinking_tags(input, &[]), input);
    }

    #[test]
    fn preserves_legitimate_tags() {
        // <code> blocks must NOT be stripped — patterns don't include "code".
        let input = "before <code>fn main() {}</code> after";
        let out = strip_thinking_tags(input, default_patterns());
        assert_eq!(out, input);
    }
}
