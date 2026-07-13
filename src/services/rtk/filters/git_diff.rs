use crate::services::rtk_filters::*;
pub fn git_diff(input: &str) -> String {
    let mut result: Vec<String> = Vec::new();
    let mut current_file = String::new();
    let mut added: u32 = 0;
    let mut removed: u32 = 0;
    let mut in_hunk = false;
    let mut hunk_shown: usize = 0;
    let mut hunk_skipped: usize = 0;
    let mut was_truncated = false;

    for line in input.lines() {
        if result.len() >= MAX_RESULT_LINES {
            result.push("\n... (more changes truncated)".into());
            was_truncated = true;
            break;
        }
        if line.starts_with("diff --git") {
            if hunk_skipped > 0 {
                result.push(format!("  ... ({} lines truncated)", hunk_skipped));
                was_truncated = true;
                hunk_skipped = 0;
            }
            if !current_file.is_empty() && (added > 0 || removed > 0) {
                result.push(format!("  +{} -{}", added, removed));
            }
            current_file = line.strip_prefix("diff --git a/")
                .or_else(|| line.strip_prefix("diff --git "))
                .unwrap_or("unknown")
                .to_string();
            if let Some(idx) = current_file.find(" b/") {
                current_file = current_file[..idx].to_string();
            }
            result.push(format!("\n{}", current_file));
            added = 0;
            removed = 0;
            in_hunk = false;
            hunk_shown = 0;
        } else if line.starts_with("@@") {
            if hunk_skipped > 0 {
                result.push(format!("  ... ({} lines truncated)", hunk_skipped));
                was_truncated = true;
                hunk_skipped = 0;
            }
            in_hunk = true;
            hunk_shown = 0;
            result.push(format!("  {}", line));
        } else if in_hunk {
            if line.starts_with('+') && !line.starts_with("+++") {
                added += 1;
                if hunk_shown < GIT_DIFF_HUNK_MAX_LINES {
                    result.push(format!("  {}", line));
                    hunk_shown += 1;
                } else {
                    hunk_skipped += 1;
                }
            } else if line.starts_with('-') && !line.starts_with("---") {
                removed += 1;
                if hunk_shown < GIT_DIFF_HUNK_MAX_LINES {
                    result.push(format!("  {}", line));
                    hunk_shown += 1;
                } else {
                    hunk_skipped += 1;
                }
            } else if hunk_shown < GIT_DIFF_HUNK_MAX_LINES && !line.starts_with('\\') {
                if hunk_shown > 0 {
                    result.push(format!("  {}", line));
                    hunk_shown += 1;
                }
            }
        }
    }

    if hunk_skipped > 0 {
        result.push(format!("  ... ({} lines truncated)", hunk_skipped));
        was_truncated = true;
    }
    if !current_file.is_empty() && (added > 0 || removed > 0) {
        result.push(format!("  +{} -{}", added, removed));
    }
    if was_truncated {
        result.push("[full diff: rtk git diff --no-compact]".into());
    }

    if result.is_empty() { input.to_string() } else { result.join("\n") }
}
