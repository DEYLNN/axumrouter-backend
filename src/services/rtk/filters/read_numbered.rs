use crate::services::rtk_filters::*;
pub fn read_numbered(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    if lines.len() < SMART_TRUNCATE_MIN_LINES { return input.to_string(); }
    let head = &lines[..SMART_TRUNCATE_HEAD.min(lines.len())];
    let tail = &lines[lines.len().saturating_sub(SMART_TRUNCATE_TAIL)..];
    let cut = lines.len() - head.len() - tail.len();
    let mut out: Vec<String> = head.iter().map(|s| s.to_string()).collect();
    out.push(format!("... +{} lines truncated (file continues)", cut));
    out.extend(tail.iter().map(|s| s.to_string()));
    out.join("\n")
}
