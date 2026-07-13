use crate::services::rtk_filters::*;
pub fn smart_truncate(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    if lines.len() < SMART_TRUNCATE_MIN_LINES { return input.to_string(); }
    let head = &lines[..SMART_TRUNCATE_HEAD.min(lines.len())];
    let tail = &lines[lines.len().saturating_sub(SMART_TRUNCATE_TAIL)..];
    let cut = lines.len() - head.len() - tail.len();
    let mut out: Vec<&str> = head.to_vec();
    out.push(&"");
    let trunc_msg = format!("... +{} lines truncated", cut);
    out.push(Box::leak(trunc_msg.into_boxed_str()));
    out.extend(tail);
    out.join("\n")
}
