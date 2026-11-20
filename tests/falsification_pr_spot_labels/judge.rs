//! Per-job, per-file and per-directory judging.

use crate::*;

pub(crate) fn judge_job(
    file: &str,
    dir: &Path,
    name: &str,
    job: &Value,
    is_ci: bool,
    why: &mut Vec<String>,
) {
    let ctx = format!("{file}: job {name}");
    let Value::Mapping(j) = job else {
        return why.push(format!("{ctx} is not a mapping, cannot judge"));
    };
    let canon = canonical(file, name);
    let all = all_scalars(job, why);
    check_opt_in_vars(&ctx, canon, &all, why);
    let uses = key(j, "uses").is_some();
    match (canon, key(j, "runs-on")) {
        (Some(c), _) => {
            check_allowlisted(&ctx, j, c, &all, why);
        }
        (None, Some(ro)) => {
            check_runs_on(&format!("{file}:{name}"), j, &ro, why);
            if let Err(e) = reaches_spot(j, &ro) {
                why.push(format!("{ctx}: can land on a spot seat: {e}"));
            }
        }
        (None, None) if !uses => why.push(format!("{ctx}: no runs-on and no uses, cannot judge")),
        (None, None) => {}
    }
    if !uses {
        return;
    }
    let is_ci_call = is_ci && name == "ci";
    judge_call(&ctx, (file, name), dir, j, 0, why);
    if let Some(w) = key(j, "with") {
        check_with(&ctx, is_ci_call, &w, why);
    }
    if is_ci_call {
        let w = key(j, "with").and_then(|w| w.as_mapping().and_then(|m| key(m, "runs_on")));
        if w.as_ref().and_then(|v| v.as_str()) != Some(REUSABLE_RUNS_ON) {
            why.push(format!(
                "{ctx}: reusable runs_on is not the literal clean-room value"
            ));
        }
    }
}

pub(crate) fn check_canary(
    base: &str,
    doc: &Value,
    top: &serde_yaml_ng::Mapping,
    expected: Option<&Value>,
    why: &mut Vec<String>,
) {
    match expected {
        None => why.push(format!("{base}: no committed expected copy, cannot judge")),
        Some(e) if e != doc => why.push(format!(
            "{base}: does not equal tests/pool-canary.expected.yml"
        )),
        Some(_) => {}
    }
    match key(top, "on") {
        Some(Value::Mapping(on)) if on.len() == 1 && key(&on, "workflow_dispatch").is_some() => {}
        _ => why.push(format!("{base}: is not dispatch-only")),
    }
}

pub(crate) fn permissions_ok(v: &Value) -> bool {
    match v {
        Value::Mapping(m) => m.iter().all(|(pk, pv)| {
            let pv = pv.as_str().unwrap_or("");
            pv == "none" || (pk.as_str() == Some("contents") && pv == "read")
        }),
        _ => false,
    }
}

pub(crate) fn check_workflow_level(
    base: &str,
    top: &serde_yaml_ng::Mapping,
    why: &mut Vec<String>,
) {
    for (k, v) in top {
        let ks = k.as_str().unwrap_or("");
        if ks == "jobs" {
            continue;
        }
        let mut w = Vec::new();
        let mut names = all_scalars(v, &mut w);
        names.push(ks.to_string());
        for s in names
            .iter()
            .filter(|s| s.to_lowercase().contains("secrets"))
        {
            why.push(format!("{base}: workflow level names secrets: {s}"));
        }
        if ks == "permissions" && !permissions_ok(v) {
            why.push(format!(
                "{base}: workflow permissions wider than contents: read"
            ));
        }
    }
}

pub(crate) fn parse_file(path: &Path, base: &str, why: &mut Vec<String>) -> Option<Value> {
    let text = fs::read_to_string(path)
        .map_err(|e| why.push(format!("{base}: unreadable: {e}")))
        .ok()?;
    let doc: Value = serde_yaml_ng::from_str(&text)
        .map_err(|e| why.push(format!("{base}: does not parse (cannot judge): {e}")))
        .ok()?;
    let mut unmodeled = Vec::new();
    all_scalars(&doc, &mut unmodeled);
    unmodeled
        .iter()
        .for_each(|w| why.push(format!("{base}: {w}")));
    Some(doc)
}

pub(crate) fn judge_file(path: &Path, expected_canary: Option<&Value>, why: &mut Vec<String>) {
    let base = path.file_name().unwrap().to_string_lossy().to_string();
    let Some(doc) = parse_file(path, &base, why) else {
        return;
    };
    let Value::Mapping(top) = &doc else {
        return why.push(format!("{base}: document is not a mapping, cannot judge"));
    };
    if base == "pool-canary.yml" {
        return check_canary(&base, &doc, top, expected_canary, why);
    }
    let is_ci = base == "ci.yml";
    let Some(Value::Mapping(jobs)) = key(top, "jobs") else {
        return why.push(format!(
            "{base}: jobs is missing or not a mapping, cannot judge"
        ));
    };
    let mut has_spot = false;
    for (n, j) in &jobs {
        let Some(n) = n.as_str() else {
            why.push(format!("{base}: non-string job id, cannot judge"));
            continue;
        };
        has_spot |= canonical(&base, n).is_some();
        judge_job(
            &base,
            path.parent().unwrap_or(Path::new(".")),
            n,
            j,
            is_ci,
            why,
        );
        if canonical(&base, n).is_some() {
            if let Value::Mapping(jm) = j {
                check_needs(&format!("{base}: job {n}"), jm, &jobs, why);
            }
        }
    }
    if has_spot {
        check_workflow_level(&base, top, why);
    }
}

/// Ok when the directory passes; Err carries every reason it does not.
pub(crate) fn judge(dir: &Path, expected_canary: &str) -> Result<usize, Vec<String>> {
    let mut why = Vec::new();
    let expected: Option<Value> = serde_yaml_ng::from_str(expected_canary).ok();
    let mut files: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.is_file()
                    && matches!(p.extension().and_then(|e| e.to_str()), Some("yml" | "yaml"))
            })
            .collect(),
        Err(e) => return Err(vec![format!("{}: unreadable: {e}", dir.display())]),
    };
    files.sort();
    if files.is_empty() {
        return Err(vec![format!(
            "no workflow files in {}, nothing was measured",
            dir.display()
        )]);
    }
    for f in &files {
        judge_file(f, expected.as_ref(), &mut why);
    }
    if why.is_empty() {
        Ok(files.len())
    } else {
        Err(why)
    }
}
