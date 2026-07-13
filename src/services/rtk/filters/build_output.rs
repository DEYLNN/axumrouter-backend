use crate::services::rtk_filters::*;
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
