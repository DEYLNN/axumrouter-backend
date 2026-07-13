use crate::services::rtk_filters::*;
use std::collections::HashMap;

pub fn search_list(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() { return input.to_string(); }
    let header = lines[0];
    let mut paths: Vec<&str> = Vec::new();
    for raw in &lines[1..] {
        let t = raw.trim();
        if t.starts_with("- ") { paths.push(&t[2..]); }
    }
    if paths.is_empty() { return input.to_string(); }

    let mut by_dir: HashMap<String, Vec<String>> = HashMap::new();
    for p in &paths {
        let (dir, name) = match p.rfind('/') {
            Some(i) => (&p[..i], &p[i+1..]),
            None => (".", *p),
        };
        by_dir.entry(dir.to_string()).or_default().push(name.to_string());
    }

    let mut dirs: Vec<&String> = by_dir.keys().collect();
    dirs.sort();
    let mut out = format!("{}\n{} files in {} dirs:\n\n", header, paths.len(), dirs.len());

    for dir in dirs.iter().take(SEARCH_LIST_TOTAL_DIR_MAX) {
        let names = &by_dir[*dir];
        out.push_str(&format!("{}/ ({}):\n", dir, names.len()));
        for n in names.iter().take(SEARCH_LIST_PER_DIR_MAX) { out.push_str(&format!("  {}\n", n)); }
        if names.len() > SEARCH_LIST_PER_DIR_MAX { out.push_str(&format!("  +{}\n", names.len() - SEARCH_LIST_PER_DIR_MAX)); }
        out.push('\n');
    }
    if dirs.len() > SEARCH_LIST_TOTAL_DIR_MAX {
        out.push_str(&format!("+{} more dirs\n", dirs.len() - SEARCH_LIST_TOTAL_DIR_MAX));
    }
    out.trim_end().to_string()
}
