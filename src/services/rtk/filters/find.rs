use crate::services::rtk_filters::*;
use std::collections::HashMap;

pub fn find(input: &str) -> String {
    let lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() { return input.to_string(); }

    let mut by_dir: HashMap<String, Vec<String>> = HashMap::new();
    for path in &lines {
        let (dir, basename) = match path.rfind('/') {
            Some(i) => (&path[..i], &path[i+1..]),
            None => (".", *path),
        };
        by_dir.entry(dir.to_string()).or_default().push(basename.to_string());
    }

    let mut dirs: Vec<&String> = by_dir.keys().collect();
    dirs.sort();
    let mut out = format!("{} files in {} dirs:\n\n", lines.len(), dirs.len());

    for dir in dirs.iter().take(FIND_TOTAL_DIR_MAX) {
        let files = &by_dir[*dir];
        let show = if dir.is_empty() { "." } else { dir.as_str() };
        out.push_str(&format!("{}/  ({})\n", show, files.len()));
        for f in files.iter().take(FIND_PER_DIR_MAX) { out.push_str(&format!("  {}\n", f)); }
        if files.len() > FIND_PER_DIR_MAX { out.push_str(&format!("  +{}\n", files.len() - FIND_PER_DIR_MAX)); }
    }
    if dirs.len() > FIND_TOTAL_DIR_MAX {
        out.push_str(&format!("\n+{} more dirs\n", dirs.len() - FIND_TOTAL_DIR_MAX));
    }
    out
}
