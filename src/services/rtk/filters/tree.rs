use crate::services::rtk_filters::*;
pub fn tree(input: &str) -> String {
    let mut filtered: Vec<&str> = Vec::new();
    for line in input.lines() {
        if line.contains("director") && line.contains("file") { continue; }
        if line.trim().is_empty() && filtered.is_empty() { continue; }
        filtered.push(line);
    }
    while filtered.last().map_or(false, |l| l.trim().is_empty()) { filtered.pop(); }
    if filtered.len() > TREE_MAX_LINES {
        let cut = filtered.len() - TREE_MAX_LINES;
        let mut out: String = filtered[..TREE_MAX_LINES].join("\n");
        out.push_str(&format!("\n... +{} more lines", cut));
        return out;
    }
    if filtered.is_empty() { input.to_string() } else { filtered.join("\n") }
}
