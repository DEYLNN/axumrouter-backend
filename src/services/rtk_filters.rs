// RTK filters — port of 9router open-sse/rtk/filters/*.js
// Compress tool_result content in-place. Each filter returns compressed String.

use std::collections::HashMap;

// ── Constants ──

pub const RAW_CAP: usize = 10 * 1024 * 1024;       // 10 MiB max input
pub const MIN_COMPRESS_SIZE: usize = 500;            // skip tiny blobs
pub const DETECT_WINDOW: usize = 1024;               // autodetect peeks first N chars
pub const GIT_DIFF_HUNK_MAX_LINES: usize = 100;
pub const DEDUP_LINE_MAX: usize = 2000;
pub const GREP_PER_FILE_MAX: usize = 10;
pub const FIND_PER_DIR_MAX: usize = 10;
pub const FIND_TOTAL_DIR_MAX: usize = 20;
pub const STATUS_MAX_FILES: usize = 10;
pub const STATUS_MAX_UNTRACKED: usize = 10;
pub const LS_EXT_SUMMARY_TOP: usize = 5;
pub const TREE_MAX_LINES: usize = 200;
pub const SEARCH_LIST_PER_DIR_MAX: usize = 10;
pub const SEARCH_LIST_TOTAL_DIR_MAX: usize = 20;
pub const SMART_TRUNCATE_HEAD: usize = 120;
pub const SMART_TRUNCATE_TAIL: usize = 60;
pub const SMART_TRUNCATE_MIN_LINES: usize = 250;
pub const READ_NUMBERED_MIN_HIT_RATIO: f64 = 0.7;
pub const MAX_RESULT_LINES: usize = 500;

// ── gitDiff ──

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

// ── gitStatus ──

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

// ── find ──

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

// ── dedupLog ──

pub fn dedup_log(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut prev: Option<&str> = None;
    let mut run_count: usize = 0;
    let mut blank_streak: usize = 0;

    let flush_run = |out: &mut Vec<String>, prev: &Option<&str>, run_count: &mut usize| {
        if let Some(p) = prev {
            if *run_count > 1 {
                out.push(format!("  ... ({} duplicate lines)", *run_count - 1));
            }
        }
        *run_count = 0;
    };

    for line in lines {
        if line.trim().is_empty() {
            if blank_streak < 1 { out.push(line.to_string()); }
            blank_streak += 1;
            flush_run(&mut out, &prev, &mut run_count);
            prev = None;
            continue;
        }
        blank_streak = 0;
        if prev == Some(line) {
            run_count += 1;
            continue;
        }
        flush_run(&mut out, &prev, &mut run_count);
        out.push(line.to_string());
        prev = Some(line);
        run_count = 1;
        if out.len() >= DEDUP_LINE_MAX {
            out.push(format!("... (truncated at {} lines)", DEDUP_LINE_MAX));
            return out.join("\n");
        }
    }
    flush_run(&mut out, &prev, &mut run_count);
    out.join("\n")
}

// ── smartTruncate ──

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

// ── readNumbered ──

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

// ── buildOutput ──

pub fn build_output(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() { return input.to_string(); }

    let mut errors: Vec<&str> = Vec::new();
    let mut warnings: Vec<&str> = Vec::new();
    let mut deprecations: Vec<&str> = Vec::new();
    let mut summary: Option<String> = None;
    let mut compiling_count: u32 = 0;
    let mut downloading_count: u32 = 0;
    let mut in_cargo_error = false;

    for line in &lines {
        let trimmed = line.trim();

        if in_cargo_error {
            if trimmed.is_empty() { in_cargo_error = false; continue; }
            let is_cont = trimmed.starts_with("-->") || trimmed.starts_with('|')
                || trimmed.chars().next().map_or(false, |c| c.is_ascii_digit() && trimmed.contains('|'))
                || trimmed.starts_with('=');
            if is_cont { errors.push(line); continue; }
            in_cargo_error = false;
        }
        if trimmed.is_empty() { continue; }

        if trimmed.starts_with("npm ERR!") || trimmed.starts_with("npm error") || trimmed.starts_with("yarn error") {
            errors.push(line);
        } else if trimmed.starts_with("npm warn deprecated") {
            deprecations.push(line);
        } else if trimmed.starts_with("npm warn") || trimmed.starts_with("yarn warn") {
            warnings.push(line);
        } else if trimmed.starts_with("error[") || trimmed.starts_with("error:") || trimmed.starts_with("error -->") {
            errors.push(line); in_cargo_error = true;
        } else if trimmed.starts_with("warning[") || trimmed.starts_with("warning:") || trimmed.starts_with("warning -->") {
            warnings.push(line); in_cargo_error = true;
        } else if trimmed.to_uppercase().starts_with("ERROR:") {
            errors.push(line);
        } else if trimmed.to_uppercase().starts_with("[ERROR]") || trimmed.to_uppercase().starts_with("BUILD FAILED") {
            errors.push(line);
        } else if trimmed.to_uppercase().starts_with("[WARNING]") {
            warnings.push(line);
        } else if trimmed.starts_with("Compiling") || trimmed.starts_with("   Compiling") {
            compiling_count += 1;
        } else if trimmed.starts_with("Downloading") || trimmed.starts_with("   Downloading") || trimmed.starts_with("Fetching") {
            downloading_count += 1;
        } else if trimmed.starts_with("added ") || trimmed.starts_with("removed ") || trimmed.starts_with("changed ")
            || trimmed.starts_with("audited ") || trimmed.starts_with("installed ")
            || trimmed.starts_with("Finished") || trimmed.starts_with("   Finished")
            || trimmed.to_uppercase().starts_with("BUILD SUCCESS")
            || trimmed.starts_with("Successfully installed") || trimmed.starts_with("Successfully built") {
            summary = Some(match summary {
                Some(s) => format!("{}\n{}", s, line),
                None => line.to_string(),
            });
        }
    }

    let mut out = String::new();
    let keep_dep = deprecations.len().min(3);
    for d in deprecations.iter().take(keep_dep) { out.push_str(d); out.push('\n'); }
    if deprecations.len() > 3 { out.push_str(&format!("... +{} more deprecated packages\n", deprecations.len() - 3)); }
    if compiling_count > 0 { out.push_str(&format!("Compiled {} packages\n", compiling_count)); }
    if downloading_count > 0 { out.push_str(&format!("Downloaded {} packages\n", downloading_count)); }
    for e in errors { out.push_str(e); out.push('\n'); }
    let keep_warn = warnings.len().min(5);
    for w in warnings.iter().take(keep_warn) { out.push_str(w); out.push('\n'); }
    if warnings.len() > 5 { out.push_str(&format!("... +{} more warnings\n", warnings.len() - 5)); }
    if let Some(s) = summary { out.push_str(&s); out.push('\n'); }

    let trimmed_out = out.trim_end().to_string();
    if trimmed_out.is_empty() { input.to_string() } else { trimmed_out }
}

// ── tree ──

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

// ── ls ──

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
        for (i, (ext, count)) in ext_vec.iter().take(LS_EXT_SUMMARY_TOP).enumerate() {
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

// ── searchList ──

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
