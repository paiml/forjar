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

pub(crate) fn key(m: &serde_yaml_ng::Mapping, k: &str) -> Option<Value> {
    m.get(Value::String(k.to_string())).cloned()
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
                match k.as_str()? {
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
    let k = expr
        .strip_prefix("${{ matrix.")
        .and_then(|r| r.strip_suffix(" }}"))
        .filter(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
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
        if (s.contains("PR_SPOT_OPTIN") || s.contains("BURST_OPTIN")) && !allowed_here {
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
