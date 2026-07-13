use crate::services::rtk_filters::*;
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
