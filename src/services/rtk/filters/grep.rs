use crate::services::rtk_filters::*;
use std::collections::HashMap;

// ── grep ──

pub fn grep(input: &str) -> String {
    let mut by_file: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut total = 0;

    for line in input.lines() {
        let first = match line.find(':') { Some(i) => i, None => continue };
        let second = match line[first+1..].find(':') { Some(i) => first + 1 + i, None => continue };
        let file = &line[..first];
        let line_num_str = &line[first+1..second];
        let content = &line[second+1..];
        if !line_num_str.chars().all(|c| c.is_ascii_digit()) { continue; }
        total += 1;
        by_file.entry(file.to_string()).or_default().push((line_num_str.to_string(), content.to_string()));
    }

    if total == 0 { return input.to_string(); }

    let mut files: Vec<&String> = by_file.keys().collect();
    files.sort();
    let mut out = format!("{} matches in {}F:\n\n", total, files.len());

    for file in files {
        let matches = &by_file[file];
        out.push_str(&format!("[file] {} ({}):\n", file, matches.len()));
        for (ln, content) in matches.iter().take(GREP_PER_FILE_MAX) {
            out.push_str(&format!("  {:>4}: {}\n", ln, content.trim()));
        }
        if matches.len() > GREP_PER_FILE_MAX {
            out.push_str(&format!("  +{}\n", matches.len() - GREP_PER_FILE_MAX));
        }
        out.push('\n');
    }
    out
}
