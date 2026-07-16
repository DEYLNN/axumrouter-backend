use crate::services::rtk_filters::*;
use std::collections::HashMap;

pub fn ls(input: &str) -> String {
    let mut dirs: Vec<String> = Vec::new();
    let mut files: Vec<(String, String)> = Vec::new(); // (name, size_str)
    let mut by_ext: HashMap<String, u32> = HashMap::new();

    let noise: &[&str] = &["node_modules", ".git", "target", "__pycache__", ".next", "dist", "build", ".cache", ".turbo",
        ".vercel", ".pytest_cache", ".mypy_cache", ".tox", ".venv", "venv", "env",
        "coverage", ".nyc_output", ".DS_Store", "Thumbs.db", ".idea", ".vscode", ".vs"];

    for line in input.lines() {
        if line.starts_with("total ") || line.is_empty() { continue; }
        // Parse: perms links owner group size month day time name
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 { continue; }
        let perms = parts[0];
        let name = parts[8..].join(" ");
        if name == "." || name == ".." { continue; }
        if noise.contains(&name.as_str()) { continue; }
        let file_type = perms.chars().next().unwrap_or('-');
        if file_type == 'd' {
            dirs.push(name);
        } else if file_type == '-' || file_type == 'l' {
            let size = parts[4].parse::<u64>().unwrap_or(0);
            let size_str = if size >= 1_048_576 { format!("{:.1}M", size as f64 / 1_048_576.0) }
                else if size >= 1024 { format!("{:.1}K", size as f64 / 1024.0) }
                else { format!("{}B", size) };
            let ext = name.rfind('.').map(|i| &name[i..]).unwrap_or("no ext").to_string();
            *by_ext.entry(ext).or_default() += 1;
            files.push((name, size_str));
        }
    }

    if dirs.is_empty() && files.is_empty() { return input.to_string(); }

    let mut out = String::new();
    for d in &dirs { out.push_str(&format!("{}/\n", d)); }
    for (name, size) in &files { out.push_str(&format!("{}  {}\n", name, size)); }

    let mut summary = format!("\nSummary: {} files, {} dirs", files.len(), dirs.len());
    if !by_ext.is_empty() {
        let mut ext_vec: Vec<(&String, &u32)> = by_ext.iter().collect();
        ext_vec.sort_by(|a, b| b.1.cmp(a.1));
        summary.push_str(" (");
        let mut first = true;
        for (_i, (ext, count)) in ext_vec.iter().take(LS_EXT_SUMMARY_TOP).enumerate() {
            if !first { summary.push_str(", "); }
            first = false;
            summary.push_str(&format!("{} {}", count, ext));
        }
        if ext_vec.len() > LS_EXT_SUMMARY_TOP {
            summary.push_str(&format!(", +{} more", ext_vec.len() - LS_EXT_SUMMARY_TOP));
        }
        summary.push(')');
    }
    out.push_str(&summary);
    out
}
