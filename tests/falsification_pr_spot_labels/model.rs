//! Parsed-document helpers: scalars, labels, matrix literals, per-job checks.

use crate::*;

pub(crate) const PS: &str = "${{ (github.event_name == 'pull_request' && vars.PR_SPOT_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\",\"burst-public\"]') || fromJSON('[\"self-hosted\",\"clean-room\",\"X64\"]') }}";
pub(crate) const BU: &str = "${{ (github.event_name == 'pull_request' && vars.BURST_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\"]') || fromJSON('[\"self-hosted\",\"clean-room\",\"X64\"]') }}";
pub(crate) const REUSABLE_RUNS_ON: &str = "[\"self-hosted\",\"clean-room\",\"X64\"]";

pub(crate) fn canonical(job: &str) -> Option<&'static str> {
    match job {
        "lockfile" | "examples-validate" | "dogfood-surface" => Some(PS),
        "no-default-features" | "doctests" | "dogfood-guards" => Some(BU),
        _ => None,
    }
}

pub(crate) fn names_pool(s: &str) -> bool {
    let l = s.to_lowercase();
    l.contains("burst") || l.contains("pool-canary")
}

/// GitHub compares context names, matrix keys, input names and variable names without regard to
/// case, so every lookup here does too. Two keys that differ only in case are refused by `scalars`.
pub(crate) fn key(m: &serde_yaml_ng::Mapping, k: &str) -> Option<Value> {
    m.iter()
        .find(|(mk, _)| mk.as_str().is_some_and(|s| s.eq_ignore_ascii_case(k)))
        .map(|(_, v)| v.clone())
}

/// `${{ <ctx>.<name> }}` (context compared case-insensitively) -> `<name>`; anything else None.
pub(crate) fn ctx_ref<'a>(s: &'a str, ctx: &str) -> Option<&'a str> {
    let r = s.trim().strip_prefix("${{ ")?.strip_suffix(" }}")?;
    let (c, k) = r.split_once('.')?;
    (c.eq_ignore_ascii_case(ctx)
        && !k.is_empty()
        && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    .then_some(k)
}

/// Every scalar (keys and values) under `v`; a merge key or a tag cannot be judged.
pub(crate) fn scalars(v: &Value, out: &mut Vec<String>, why: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Number(n) => out.push(n.to_string()),
        Value::Bool(b) => out.push(b.to_string()),
        Value::Null => {}
        Value::Sequence(s) => s.iter().for_each(|x| scalars(x, out, why)),
        Value::Mapping(m) => {
            let mut seen = std::collections::HashSet::new();
            for k in m.keys().filter_map(Value::as_str) {
                if !seen.insert(k.to_ascii_lowercase()) {
                    why.push(format!("keys differing only in case: `{k}`, cannot judge"));
                }
            }
            for (k, x) in m {
                if matches!(k, Value::String(s) if s == "<<") {
                    why.push("merge key `<<` is not modeled".into());
                }
                scalars(k, out, why);
                scalars(x, out, why);
            }
        }
        Value::Tagged(_) => why.push("tagged value is not modeled".into()),
    }
}

pub(crate) fn all_scalars(v: &Value, why: &mut Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    scalars(v, &mut out, why);
    out
}

/// The flat label list of a resolved runs-on, or None when the shape is not modeled.
pub(crate) fn labels(v: &Value) -> Option<Vec<String>> {
    match v {
        Value::String(s) => Some(vec![s.clone()]),
        Value::Sequence(s) => s
            .iter()
            .map(|x| x.as_str().map(str::to_string))
            .collect::<Option<Vec<_>>>(),
        Value::Mapping(m) => {
            let mut out = Vec::new();
            for (k, x) in m {
                match k.as_str()?.to_ascii_lowercase().as_str() {
                    "group" => out.push(x.as_str()?.to_string()),
                    "labels" => out.extend(labels(x)?),
                    _ => return None,
                }
            }
            Some(out)
        }
        _ => None,
    }
}

pub(crate) fn literal_matrix_ok(job: &serde_yaml_ng::Mapping, name: &str, why: &mut Vec<String>) {
    let ro = key(job, "runs-on").unwrap_or(Value::Null);
    let expr = ro.as_str().unwrap_or("").trim();
    let k = ctx_ref(expr, "matrix");
    let Some(k) = k else {
        why.push(format!(
            "job {name}: runs-on expression is not allowed: {expr}"
        ));
        return;
    };
    let Some(Value::Mapping(strategy)) = key(job, "strategy") else {
        why.push(format!(
            "job {name}: matrix.{k} has no strategy.matrix to judge"
        ));
        return;
    };
    let Some(Value::Mapping(matrix)) = key(&strategy, "matrix") else {
        why.push(format!(
            "job {name}: strategy.matrix is not a mapping, cannot judge"
        ));
        return;
    };
    let mut vals = Vec::new();
    if let Some(v) = key(&matrix, k) {
        vals.push(v);
    }
    match key(&matrix, "include") {
        None => {}
        Some(Value::Sequence(items)) => {
            for it in items {
                match it {
                    Value::Mapping(m) => vals.extend(key(&m, k)),
                    _ => why.push(format!("job {name}: matrix include entry not a mapping")),
                }
            }
        }
        Some(_) => why.push(format!(
            "job {name}: matrix include is not a list, cannot judge"
        )),
    }
    if vals.is_empty() {
        why.push(format!("job {name}: matrix.{k} has no literal values"));
    }
    let mut inner = Vec::new();
    for v in &vals {
        let s = all_scalars(v, why);
        if s.is_empty() {
            inner.push("(empty)".to_string());
        }
        inner.extend(s);
    }
    for s in inner {
        if s.contains("${{") || names_pool(&s) || s == "(empty)" {
            why.push(format!(
                "job {name}: matrix.{k} value is not a plain literal: {s}"
            ));
        }
    }
}

pub(crate) fn check_opt_in_vars(
    ctx: &str,
    canon: Option<&str>,
    all: &[String],
    why: &mut Vec<String>,
) {
    for s in all {
        let allowed_here = canon.is_some_and(|c| s == c);
        let l = s.to_lowercase();
        if (l.contains("pr_spot_optin") || l.contains("burst_optin")) && !allowed_here {
            why.push(format!(
                "{ctx}: opt-in variable outside its canonical runs-on: {s}"
            ));
        }
    }
}

pub(crate) fn check_allowlisted(
    ctx: &str,
    j: &serde_yaml_ng::Mapping,
    c: &str,
    all: &[String],
    why: &mut Vec<String>,
) {
    match key(j, "runs-on") {
        Some(Value::String(s)) if s == c => {}
        other => why.push(format!(
            "{ctx}: runs-on is not its one canonical value: {other:?}"
        )),
    }
    let want: Value = serde_yaml_ng::from_str("contents: read").unwrap();
    if key(j, "permissions").as_ref() != Some(&want) {
        why.push(format!("{ctx}: permissions are not exactly contents: read"));
    }
    for s in all {
        let l = s.to_lowercase();
        if l.contains("secrets") || l.contains("id-token") {
            why.push(format!("{ctx}: names secrets or id-token: {s}"));
        }
        if l.contains("download-artifact") || l.contains("run download") {
            why.push(format!("{ctx}: downloads an artifact: {s}"));
        }
    }
}

pub(crate) fn check_runs_on(
    ctx: &str,
    j: &serde_yaml_ng::Mapping,
    ro: &Value,
    why: &mut Vec<String>,
) {
    match labels(ro) {
        None => why.push(format!("{ctx}: runs-on shape is not modeled, cannot judge")),
        Some(ls) if ls.iter().any(|l| names_pool(l)) => {
            why.push(format!("{ctx}: runs-on names burst or pool-canary: {ls:?}"));
        }
        Some(ls) if ls.iter().any(|l| l.contains("${{")) => literal_matrix_ok(j, ctx, why),
        Some(_) => {}
    }
}

pub(crate) fn check_with(ctx: &str, is_ci_call: bool, w: &Value, why: &mut Vec<String>) {
    let Value::Mapping(wm) = w else {
        return why.push(format!("{ctx}: `with` is not a mapping, cannot judge"));
    };
    for (k, x) in wm {
        let ks = k.as_str().unwrap_or("").to_lowercase();
        let vs = all_scalars(x, why);
        let exact = is_ci_call && ks == "runs_on" && vs.len() == 1 && vs[0] == REUSABLE_RUNS_ON;
        for s in vs {
            if !exact && (names_pool(&s) || (ks.contains("runs") && !s.is_empty())) {
                why.push(format!(
                    "{ctx}: reusable call passes a runs-on or pool value: {k:?}={s}"
                ));
            }
        }
    }
}

fn grants_write(v: &Value) -> bool {
    let w = |s: &str| s.to_lowercase().contains("write");
    match v {
        Value::String(s) => w(s),
        Value::Mapping(m) => m.values().any(|x| x.as_str().is_some_and(w)),
        _ => false,
    }
}

/// An allowlisted spot job may take nothing from a job that can hold a secret or a write token:
/// no `needs` on one (transitively), no unknown `needs`, and no artifact download. A job's
/// outputs and artifacts are the two ways a secret crosses from a clean-room job to a spot one.
pub(crate) fn check_needs(
    ctx: &str,
    j: &serde_yaml_ng::Mapping,
    jobs: &serde_yaml_ng::Mapping,
    why: &mut Vec<String>,
) {
    let mut todo: Vec<String> = match key(j, "needs") {
        None => vec![],
        Some(Value::String(s)) => vec![s],
        Some(Value::Sequence(v)) => {
            let names: Vec<_> = v
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect();
            if names.len() != v.len() {
                why.push(format!(
                    "{ctx}: needs: has a non-string entry, cannot judge"
                ));
            }
            names
        }
        Some(_) => {
            why.push(format!("{ctx}: needs: shape is not modeled, cannot judge"));
            vec![]
        }
    };
    let mut seen = std::collections::HashSet::new();
    while let Some(n) = todo.pop() {
        if !seen.insert(n.to_lowercase()) {
            continue;
        }
        let Some(Value::Mapping(nj)) = key(jobs, &n) else {
            why.push(format!("{ctx}: needs: unknown or unjudgeable job `{n}`"));
            continue;
        };
        let mut w = Vec::new();
        let named: Vec<String> = all_scalars(&Value::Mapping(nj.clone()), &mut w)
            .into_iter()
            .filter(|s| {
                let l = s.to_lowercase();
                l.contains("secrets") || l.contains("id-token")
            })
            .collect();
        if !named.is_empty() || key(&nj, "permissions").is_some_and(|p| grants_write(&p)) {
            why.push(format!(
                "{ctx}: needs: job `{n}` holds a secret, id-token or write permission"
            ));
        }
        match key(&nj, "needs") {
            Some(Value::String(s)) => todo.push(s),
            Some(Value::Sequence(v)) => {
                todo.extend(v.iter().filter_map(|x| x.as_str().map(str::to_string)))
            }
            _ => {}
        }
    }
}
