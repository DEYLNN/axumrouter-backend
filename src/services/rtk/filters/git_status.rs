use crate::services::rtk_filters::*;
pub fn git_status(input: &str) -> String {
    let lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() { return "Clean working tree".into(); }

    let mut branch = String::new();
    let mut staged_files: Vec<String> = Vec::new();
    let mut modified_files: Vec<String> = Vec::new();
    let mut untracked_files: Vec<String> = Vec::new();
    let mut staged: u32 = 0;
    let mut modified: u32 = 0;
    let mut untracked: u32 = 0;
    let mut conflicts: u32 = 0;

    let porcelain_re = |s: &str| {
        s.len() >= 3 && s.chars().nth(0).map_or(false, |c| " MADRCU?!".contains(c))
            && s.chars().nth(1).map_or(false, |c| " MADRCU?!".contains(c))
            && s.chars().nth(2) == Some(' ')
    };

    for raw in &lines {
        if let Some(b) = raw.strip_prefix("On branch ") { branch = b.trim().to_string(); continue; }
        if raw.starts_with("##") { branch = raw.trim_start_matches("## ").to_string(); continue; }

        if porcelain_re(raw) {
            let x = raw.chars().nth(0).unwrap();
            let y = raw.chars().nth(1).unwrap();
            let file = &raw[3..];

            if raw.starts_with("??") {
                untracked += 1;
                untracked_files.push(file.to_string());
                continue;
            }
            if "MADRC".contains(x) {
                staged += 1;
                staged_files.push(file.to_string());
            } else if x == 'U' { conflicts += 1; }
            if y == 'M' || y == 'D' {
                modified += 1;
                modified_files.push(file.to_string());
            }
            continue;
        }

        // Long form: "modified: path", "new file: path", etc.
        if let Some(rest) = raw.trim().strip_prefix("modified:").or(raw.trim().strip_prefix("Modified:")) {
            modified += 1;
            modified_files.push(rest.trim().to_string());
        } else if let Some(rest) = raw.trim().strip_prefix("new file:").or(raw.trim().strip_prefix("New file:")) {
            staged += 1;
            staged_files.push(rest.trim().to_string());
        } else if let Some(rest) = raw.trim().strip_prefix("deleted:").or(raw.trim().strip_prefix("Deleted:")) {
            modified += 1;
            modified_files.push(rest.trim().to_string());
        } else if let Some(_) = raw.trim().strip_prefix("renamed:") {
            staged += 1;
        } else if raw.contains("both modified") { conflicts += 1; }
    }

    let mut out = String::new();
    if !branch.is_empty() { out.push_str(&format!("* {}\n", branch)); }

    if staged > 0 {
        out.push_str(&format!("+ Staged: {} files\n", staged));
        for f in staged_files.iter().take(STATUS_MAX_FILES) { out.push_str(&format!("   {}\n", f)); }
        if staged_files.len() > STATUS_MAX_FILES { out.push_str(&format!("   ... +{} more\n", staged_files.len() - STATUS_MAX_FILES)); }
    }
    if modified > 0 {
        out.push_str(&format!("~ Modified: {} files\n", modified));
        for f in modified_files.iter().take(STATUS_MAX_FILES) { out.push_str(&format!("   {}\n", f)); }
        if modified_files.len() > STATUS_MAX_FILES { out.push_str(&format!("   ... +{} more\n", modified_files.len() - STATUS_MAX_FILES)); }
    }
    if untracked > 0 {
        out.push_str(&format!("? Untracked: {} files\n", untracked));
        for f in untracked_files.iter().take(STATUS_MAX_UNTRACKED) { out.push_str(&format!("   {}\n", f)); }
        if untracked_files.len() > STATUS_MAX_UNTRACKED { out.push_str(&format!("   ... +{} more\n", untracked_files.len() - STATUS_MAX_UNTRACKED)); }
    }
    if conflicts > 0 { out.push_str(&format!("conflicts: {} files\n", conflicts)); }
    if staged == 0 && modified == 0 && untracked == 0 && conflicts == 0 { out.push_str("clean — nothing to commit\n"); }

    out.trim_end().to_string()
}
