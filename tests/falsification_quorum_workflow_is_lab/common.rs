use std::path::Path;

pub(crate) fn workflow() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/quorum.yml"),
    )
    .expect("quorum.yml")
}

/// The `on:` value: any inline text after `on:` plus the indented block under it.
pub(crate) fn triggers_of(w: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for l in w.lines() {
        if let Some(rest) = l.strip_prefix("on:") {
            inside = true;
            out.push_str(&strip_comment(rest));
            out.push('\n');
            continue;
        }
        if inside && !l.is_empty() && !l.starts_with(' ') && !l.starts_with('#') {
            break;
        }
        if inside && !l.trim_start().starts_with('#') {
            out.push_str(&strip_comment(l));
            out.push('\n');
        }
    }
    out
}

pub(crate) fn strip_comment(l: &str) -> String {
    l.split(" #").next().unwrap_or("").to_string()
}

/// Allow-list: ONLY these may appear under `on:`; anything else is a PR-path (or other) trigger.
pub(crate) const ALLOWED_TRIGGERS: [&str; 2] = ["schedule", "workflow_dispatch"];

/// Trigger names under `on:` (inline `[a, b]` / `a` / `{a: ..}` forms and 2-space block keys).
pub(crate) fn trigger_names(t: &str) -> Vec<String> {
    let mut names = Vec::new();
    for (i, l) in t.lines().enumerate() {
        if i == 0 {
            let inline = l.trim().trim_matches(|c| "[]{}".contains(c));
            for part in inline.split(',') {
                let n = part.split(':').next().unwrap_or("").trim();
                if !n.is_empty() {
                    names.push(n.to_string());
                }
            }
        } else if l.starts_with("  ") && !l.starts_with("   ") && !l.trim_start().starts_with('-') {
            let n = l.trim().trim_end_matches(':').trim();
            if !n.is_empty() {
                names.push(n.to_string());
            }
        }
    }
    names
}

/// Every trigger that is not on the allow-list.
pub(crate) fn forbidden(t: &str) -> Vec<String> {
    trigger_names(t)
        .into_iter()
        .filter(|n| !ALLOWED_TRIGGERS.contains(&n.as_str()))
        .collect()
}

/// Text of every `run: |` block (lines indented deeper than the `run:` key).
pub(crate) fn run_blocks(w: &str) -> String {
    let mut out = String::new();
    let mut indent: Option<usize> = None;
    for l in w.lines() {
        let ind = l.len() - l.trim_start().len();
        if let Some(i) = indent {
            if l.trim().is_empty() || ind > i {
                out.push_str(l);
                out.push('\n');
                continue;
            }
            indent = None;
        }
        if l.trim_start().starts_with("run:") {
            indent = Some(ind);
        }
    }
    out
}

/// User input reaches the shell only via `env:`; no `${{ }}` expression inside any `run:` block.
pub(crate) fn no_expression_in_run(w: &str) -> bool {
    !run_blocks(w).contains("${{")
}

pub(crate) fn code_lines(w: &str) -> Vec<String> {
    run_blocks(w)
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Exactly one gate invocation, and it runs the file extracted from `$base` into `$d`.
pub(crate) fn runs_only_base_extracted_gate(w: &str) -> bool {
    let extract =
        r#"git show "$base:scripts/quorum-store-gate.sh" > "$d/scripts/quorum-store-gate.sh""#;
    let lines = code_lines(w);
    let others: Vec<&String> = lines
        .iter()
        .filter(|l| l.contains("quorum-store-gate.sh") && !l.contains(extract))
        .collect();
    let d_ok = lines
        .iter()
        .any(|l| l.trim() == r#"d="$RUNNER_TEMP/quorum-base-gate-$n""#);
    others.len() == 1
        && others[0].contains(r#"bash "$d/scripts/quorum-store-gate.sh""#)
        && !others[0].contains("FETCH_HEAD")
        && !others[0].contains("| bash")
        && !others[0].contains("<(")
        && lines.iter().filter(|l| l.contains(extract)).count() == 1
        && d_ok
}

/// Every `continue` and every failure branch sets `rc=1` first.
pub(crate) fn fails_closed(w: &str) -> bool {
    let lines = code_lines(w);
    let continues_ok = lines.iter().filter(|l| l.contains("continue")).all(|l| {
        l.split("continue")
            .next()
            .is_some_and(|b| b.contains("rc=1"))
    });
    let marked = [
        "FAIL: bad PR number",
        "FAIL: gh pr view",
        "FAIL: fetch PR",
        "base lacks the gate",
    ]
    .iter()
    .all(|m| {
        let hit: Vec<&String> = lines.iter().filter(|l| l.contains(m)).collect();
        !hit.is_empty() && hit.iter().all(|l| l.contains("rc=1; continue"))
    });
    continues_ok && marked
}

pub(crate) fn triggers() -> String {
    triggers_of(&workflow())
}
