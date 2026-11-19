//! Spot (burst) labels, judged on the PARSED workflow documents, never on raw lines.
//!
//! Every `.yml` and `.yaml` under `.github/workflows/` is parsed with the crate's own YAML
//! dependency (serde_yaml_ng), so quoted keys, `\x73` escapes, flow style, explicit `? key`
//! entries and anchors all resolve before anything is judged. Rules:
//!  (a) a job's resolved `runs-on` (string, list, or group/labels map) may name 'burst' or
//!      'pool-canary', or hold a `${{` expression, ONLY for an allowlisted ci.yml job id and ONLY
//!      when it equals that job's one canonical value. A matrix reference is allowed only when
//!      every matrix value is a plain literal without those names. A reusable-workflow call must
//!      not pass such a value in `with`.
//!  (b) an allowlisted job has resolved permissions exactly {contents: read}, no scalar naming
//!      'secrets' or 'id-token', and ci.yml has no workflow-level scalar naming 'secrets' and no
//!      workflow permissions wider than contents: read.
//!  (c) pool-canary.yml parses equal to tests/pool-canary.expected.yml and is dispatch-only.
//!  (d) cannot judge = red: parse failure, duplicate keys, a merge key or tag, a non-mapping
//!      document or jobs section, an unmodeled runs-on shape, an empty or missing directory.

use serde_yaml_ng::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const PS: &str = "${{ (github.event_name == 'pull_request' && vars.PR_SPOT_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\",\"burst-public\"]') || fromJSON('[\"self-hosted\",\"clean-room\",\"X64\"]') }}";
const BU: &str = "${{ (github.event_name == 'pull_request' && vars.BURST_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\"]') || fromJSON('[\"self-hosted\",\"clean-room\",\"X64\"]') }}";
const REUSABLE_RUNS_ON: &str = "[\"self-hosted\",\"clean-room\",\"X64\"]";

fn canonical(job: &str) -> Option<&'static str> {
    match job {
        "lockfile" | "examples-validate" | "dogfood-surface" => Some(PS),
        "no-default-features" | "doctests" | "dogfood-guards" => Some(BU),
        _ => None,
    }
}

fn names_pool(s: &str) -> bool {
    let l = s.to_lowercase();
    l.contains("burst") || l.contains("pool-canary")
}

fn key(m: &serde_yaml_ng::Mapping, k: &str) -> Option<Value> {
    m.get(Value::String(k.to_string())).cloned()
}

/// Every scalar (keys and values) under `v`; a merge key or a tag cannot be judged.
fn scalars(v: &Value, out: &mut Vec<String>, why: &mut Vec<String>) {
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

fn all_scalars(v: &Value, why: &mut Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    scalars(v, &mut out, why);
    out
}

/// The flat label list of a resolved runs-on, or None when the shape is not modeled.
fn labels(v: &Value) -> Option<Vec<String>> {
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

fn literal_matrix_ok(job: &serde_yaml_ng::Mapping, name: &str, why: &mut Vec<String>) {
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

fn check_opt_in_vars(ctx: &str, canon: Option<&str>, all: &[String], why: &mut Vec<String>) {
    for s in all {
        let allowed_here = canon.is_some_and(|c| s == c);
        if (s.contains("PR_SPOT_OPTIN") || s.contains("BURST_OPTIN")) && !allowed_here {
            why.push(format!(
                "{ctx}: opt-in variable outside its canonical runs-on: {s}"
            ));
        }
    }
}

fn check_allowlisted(
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

fn check_runs_on(ctx: &str, j: &serde_yaml_ng::Mapping, ro: &Value, why: &mut Vec<String>) {
    match labels(ro) {
        None => why.push(format!("{ctx}: runs-on shape is not modeled, cannot judge")),
        Some(ls) if ls.iter().any(|l| names_pool(l)) => {
            why.push(format!("{ctx}: runs-on names burst or pool-canary: {ls:?}"));
        }
        Some(ls) if ls.iter().any(|l| l.contains("${{")) => literal_matrix_ok(j, ctx, why),
        Some(_) => {}
    }
}

fn check_with(ctx: &str, is_ci_call: bool, w: &Value, why: &mut Vec<String>) {
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

// ---- rule (e): GitHub routes by label SUBSET, so judge the set, not the names ----

/// MEASURED SNAPSHOT, 2026-11-19, read-only REST (`orgs/paiml/actions/runners`, 640 burst seats of
/// which 2 online): every burst seat carries exactly `Linux, burst-x64, ffmpeg, self-hosted, tier2`
/// and sits in runner group `tier2`. `x64` and `burst-public` are added on purpose (the arch label
/// may appear on a seat, and `burst-public` is the PR-only spot label). Compared lower-case.
/// A snapshot, not a live read: re-measure before trusting it for a new seat family.
const SPOT_SEAT_LABELS: [&str; 7] = [
    "self-hosted",
    "linux",
    "x64",
    "tier2",
    "ffmpeg",
    "burst-x64",
    "burst-public",
];

/// MEASURED SNAPSHOT, 2026-11-19, same REST read: org runner groups holding ZERO burst seats
/// (`quarantine` and `burst-public` are left out on purpose). `tier2` holds all of them.
const NON_SPOT_GROUPS: [&str; 15] = [
    "Default",
    "gpu-nodes",
    "gpu-x86",
    "aprender-reserved",
    "gate-path-slot",
    "rmedia-loan",
    "mini-mac",
    "public-build",
    "lambda-night",
    "aprender-only",
    "infra-first",
    "gate-light-shared",
    "release-lane",
    "demo-record",
    "dotgithub-gate",
];

/// True when a pull_request-like event (or a call from one) can start this workflow.
/// An `on` that cannot be read counts as reachable.
fn pr_reachable(top: &serde_yaml_ng::Mapping) -> bool {
    let hit = |s: &str| {
        s.starts_with("pull_request")
            || matches!(s, "merge_group" | "workflow_call" | "workflow_run")
    };
    let on = key(top, "on").or_else(|| top.get(Value::Bool(true)).cloned());
    match on {
        Some(Value::String(s)) => hit(&s),
        Some(Value::Sequence(v)) => v.iter().any(|x| x.as_str().is_none_or(hit)),
        Some(Value::Mapping(m)) => m.keys().any(|k| k.as_str().is_none_or(hit)),
        _ => true,
    }
}

/// Ok when this label list holds at least one label no spot seat carries.
fn leaves_spot(opt: &Value) -> Result<(), String> {
    let ls: Vec<String> = match opt {
        Value::String(s) => vec![s.clone()],
        Value::Sequence(v) => v
            .iter()
            .map(|x| x.as_str().map(str::to_string))
            .collect::<Option<Vec<_>>>()
            .ok_or("a label is not a plain string")?,
        _ => return Err("runs-on shape is not modeled".into()),
    };
    if ls.is_empty() || ls.iter().any(|l| l.contains("${{")) {
        return Err(format!("labels cannot be resolved statically: {ls:?}"));
    }
    if ls
        .iter()
        .any(|l| !SPOT_SEAT_LABELS.contains(&l.to_lowercase().as_str()))
    {
        Ok(())
    } else {
        Err(format!(
            "labels {ls:?} are a subset of the spot seat labels"
        ))
    }
}

fn matrix_options(j: &serde_yaml_ng::Mapping, k: &str) -> Result<Vec<Value>, String> {
    let Some(Value::Mapping(strategy)) = key(j, "strategy") else {
        return Err("matrix reference without a literal strategy".into());
    };
    let Some(Value::Mapping(matrix)) = key(&strategy, "matrix") else {
        return Err("strategy.matrix is not a literal mapping".into());
    };
    let mut out = Vec::new();
    match key(&matrix, k) {
        None => {}
        Some(Value::Sequence(v)) => out.extend(v),
        Some(_) => return Err(format!("matrix.{k} is not a literal list")),
    }
    match key(&matrix, "include") {
        None => {}
        Some(Value::Sequence(items)) => {
            for it in items {
                match it {
                    Value::Mapping(m) => out.extend(key(&m, k)),
                    _ => return Err("matrix include entry is not a mapping".into()),
                }
            }
        }
        Some(_) => return Err("matrix include is not a literal list".into()),
    }
    if out.is_empty() {
        return Err(format!("matrix.{k} has no literal options"));
    }
    Ok(out)
}

fn reaches_spot(j: &serde_yaml_ng::Mapping, ro: &Value) -> Result<(), String> {
    match ro {
        Value::Mapping(m) => {
            if let Some(k) = m
                .keys()
                .find(|k| !matches!(k.as_str(), Some("group" | "labels")))
            {
                return Err(format!("runs-on key {k:?} is not modeled"));
            }
            match (key(m, "group"), key(m, "labels")) {
                (Some(Value::String(g)), _) if NON_SPOT_GROUPS.contains(&g.as_str()) => Ok(()),
                (Some(g), _) => Err(format!("runner group {g:?} is not on the non-spot list")),
                (None, Some(l)) => leaves_spot(&l),
                (None, None) => Err("empty runs-on mapping".into()),
            }
        }
        Value::String(s) if s.contains("${{") => {
            let k = s
                .trim()
                .strip_prefix("${{ matrix.")
                .and_then(|r| r.strip_suffix(" }}"))
                .filter(|k| {
                    !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                })
                .ok_or_else(|| format!("runs-on expression cannot be resolved: {s}"))?;
            matrix_options(j, k)?.iter().try_for_each(leaves_spot)
        }
        other => leaves_spot(other),
    }
}

fn judge_job(file: &str, name: &str, job: &Value, is_ci: bool, reach: bool, why: &mut Vec<String>) {
    let ctx = format!("{file}: job {name}");
    let Value::Mapping(j) = job else {
        return why.push(format!("{ctx} is not a mapping, cannot judge"));
    };
    let canon = if is_ci { canonical(name) } else { None };
    let all = all_scalars(job, why);
    check_opt_in_vars(&ctx, canon, &all, why);
    let uses = key(j, "uses").is_some();
    match (canon, key(j, "runs-on")) {
        (Some(c), _) => check_allowlisted(&ctx, j, c, &all, why),
        (None, Some(ro)) => {
            check_runs_on(&format!("{file}:{name}"), j, &ro, why);
            if reach {
                if let Err(e) = reaches_spot(j, &ro) {
                    why.push(format!("{ctx}: can land on a spot seat: {e}"));
                }
            }
        }
        (None, None) if !uses => why.push(format!("{ctx}: no runs-on and no uses, cannot judge")),
        (None, None) => {}
    }
    if !uses {
        return;
    }
    let is_ci_call = is_ci && name == "ci";
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

fn check_canary(
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

fn permissions_ok(v: &Value) -> bool {
    match v {
        Value::Mapping(m) => m.iter().all(|(pk, pv)| {
            let pv = pv.as_str().unwrap_or("");
            pv == "none" || (pk.as_str() == Some("contents") && pv == "read")
        }),
        _ => false,
    }
}

fn check_workflow_level(base: &str, top: &serde_yaml_ng::Mapping, why: &mut Vec<String>) {
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

fn parse_file(path: &Path, base: &str, why: &mut Vec<String>) -> Option<Value> {
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

fn judge_file(path: &Path, expected_canary: Option<&Value>, why: &mut Vec<String>) {
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
        has_spot |= is_ci && canonical(n).is_some();
        judge_job(&base, n, j, is_ci, pr_reachable(top), why);
    }
    if is_ci && has_spot {
        check_workflow_level(&base, top, why);
    }
}

/// Ok when the directory passes; Err carries every reason it does not.
fn judge(dir: &Path, expected_canary: &str) -> Result<usize, Vec<String>> {
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

fn expected_canary() -> String {
    fs::read_to_string(repo().join("tests/pool-canary.expected.yml")).unwrap()
}

fn guard(dir: &Path) -> bool {
    match judge(dir, &expected_canary()) {
        Ok(_) => true,
        Err(w) => {
            eprintln!("guard refused: {}", w.join(" | "));
            false
        }
    }
}

/// One directory per call, never shared between tests that run on parallel threads (the r5 flake:
/// two tests used one name per process and one deleted the other's tree).
fn unique_dir(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let i = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("forjar-spot-{name}-{}-{i}", std::process::id()))
}

fn scratch(name: &str, ci: &str) -> PathBuf {
    let d = unique_dir(name);
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    fs::write(d.join("ci.yml"), ci).unwrap();
    d
}

fn red(name: &str, ci: &str) {
    assert!(!guard(&scratch(name, ci)), "mutant {name} was accepted");
}

/// Red for the NAMED reason, so a mutant cannot pass by tripping some other rule.
fn red_why(name: &str, ci: &str, needle: &str) {
    match judge(&scratch(name, ci), &expected_canary()) {
        Ok(_) => panic!("mutant {name} was accepted"),
        Err(w) => assert!(
            w.iter().any(|x| x.contains(needle)),
            "mutant {name} refused for the wrong reason (want `{needle}`): {w:?}"
        ),
    }
}

fn ci_text() -> String {
    fs::read_to_string(repo().join(".github/workflows/ci.yml")).unwrap()
}

/// Replace the first `from` that sits inside job `job` of the real ci.yml.
fn job_mut(job: &str, from: &str, to: &str) -> String {
    let ci = ci_text();
    let at = ci.find(&format!("\n  {job}:\n")).expect("job present");
    let (a, b) = ci.split_at(at);
    let m = b.replacen(from, to, 1);
    assert_ne!(b, m, "mutation must apply in {job}: {from}");
    format!("{a}{m}")
}

fn real_mut(from: &str, to: &str) -> String {
    let ci = ci_text();
    let m = ci.replacen(from, to, 1);
    assert_ne!(ci, m, "mutation must apply: {from}");
    m
}

#[test]
fn real_tree_passes() {
    let n = judge(&repo().join(".github/workflows"), &expected_canary())
        .unwrap_or_else(|w| panic!("{w:#?}"));
    assert!(n >= 20, "guard measured only {n} workflow files");
}

#[test]
fn empty_and_missing_dir_fail() {
    let d = unique_dir("empty");
    fs::create_dir_all(&d).unwrap();
    assert!(!guard(&d));
    assert!(!guard(&d.join("absent")));
}

#[test]
fn reusable_runs_on_with_spot_label_fails() {
    let m = real_mut(
        "runs_on: '[\"self-hosted\",\"clean-room\",\"X64\"]'",
        "runs_on: '[\"self-hosted\",\"burst-x64\",\"burst-public\"]'",
    );
    red("runs-on", &m);
}

#[test]
fn signing_job_on_spot_label_fails() {
    let ci = "on: pull_request\njobs:\n  sign:\n    permissions:\n      id-token: write\n    runs-on: [self-hosted, burst-x64]\n    steps: []\n";
    red("sign", ci);
    assert!(guard(&scratch(
        "sign-ok",
        &ci.replace("burst-x64", "clean-room")
    )));
}

// ---- pool-canary.yml: parsed equal to the expected copy, dispatch-only ----

fn canary() -> String {
    fs::read_to_string(repo().join(".github/workflows/pool-canary.yml")).unwrap()
}

fn canary_dir(name: &str, text: &str) -> PathBuf {
    let d = scratch(name, "on: push\njobs: {}\n");
    fs::write(d.join("pool-canary.yml"), text).unwrap();
    d
}

fn red_canary(name: &str, text: &str) {
    assert!(
        !guard(&canary_dir(name, text)),
        "canary mutant {name} accepted"
    );
}

#[test]
fn canary_real_file_passes() {
    assert!(guard(&canary_dir("canary-ok", &canary())));
}

#[test]
fn canary_mutants_fail() {
    let c = canary();
    let cases: [(&str, String); 8] = [
        (
            "secret",
            c.replace("IID: ${{ inputs.iid }}", "IID: ${{ secrets.TOKEN }}"),
        ),
        (
            "idtoken",
            c.replace("contents: read", "contents: read\n  id-token: write"),
        ),
        (
            "trigger",
            c.replace(
                "on:\n  workflow_dispatch:",
                "on:\n  pull_request:\n  workflow_dispatch:",
            ),
        ),
        (
            "runs-on",
            c.replace(
                "[self-hosted, pool-canary]",
                "[self-hosted, clean-room, X64]",
            ),
        ),
        ("write", c.replace("contents: read", "contents: write")),
        (
            "step",
            c.replace("jq --version", "jq --version\n          echo hi"),
        ),
        (
            "extra-job",
            format!("{c}  extra:\n    runs-on: [self-hosted, pool-canary]\n    steps: []\n"),
        ),
        ("unparseable", format!("{c}\n  : : [\n")),
    ];
    for (n, m) in cases {
        assert_ne!(c, m, "{n}");
        red_canary(n, &m);
    }
    // a comment is not a change to the parsed document
    assert!(guard(&canary_dir("canary-comment", &format!("{}# x\n", c))));
}

#[test]
fn canary_label_in_another_file_or_name_fails() {
    let d = scratch("canary-elsewhere", "on: push\njobs: {}\n");
    fs::write(d.join("copy.yml"), canary()).unwrap();
    assert!(!guard(&d));
}

// ---- spot job template ----

const SPOT_JOB: &str = "on: pull_request\njobs:\n  lockfile:\n    runs-on: ${{ (github.event_name == 'pull_request' && vars.PR_SPOT_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\",\"burst-public\"]') || fromJSON('[\"self-hosted\",\"clean-room\",\"X64\"]') }}\n    permissions:\n      contents: read\n    steps: []\n";

#[test]
fn spot_job_template_passes() {
    assert!(guard(&scratch("spot-ok", SPOT_JOB)));
    // comments after a job header are not a change to the parsed document
    assert!(guard(&scratch(
        "spot-comment",
        &SPOT_JOB.replace("  lockfile:\n", "  lockfile: # c\n")
    )));
}

#[test]
fn spot_job_with_secret_fails() {
    red(
        "spot-secret",
        &SPOT_JOB.replace(
            "steps: []",
            "steps:\n      - run: x\n        env:\n          T: ${{ secrets.X }}",
        ),
    );
}

#[test]
fn spot_job_permission_variants_fail() {
    red(
        "write-all",
        &SPOT_JOB.replace(
            "permissions:\n      contents: read",
            "permissions: write-all",
        ),
    );
    for extra in ["pull-requests: write", "id-token: write", "contents: write"] {
        let base = if extra.starts_with("contents") {
            SPOT_JOB.replace("contents: read", extra)
        } else {
            SPOT_JOB.replace("contents: read", &format!("contents: read\n      {extra}"))
        };
        red("scope", &base);
    }
    red(
        "noperm",
        &SPOT_JOB.replace("    permissions:\n      contents: read\n", ""),
    );
    red(
        "flow-perm-write",
        &SPOT_JOB.replace(
            "permissions:\n      contents: read",
            "permissions: {contents: write}",
        ),
    );
}

#[test]
fn spot_job_on_non_pull_request_event_fails() {
    for ev in [
        "github.event_name != 'pull_request'",
        "github.event_name == 'push'",
        "true",
    ] {
        red(
            "event",
            &SPOT_JOB.replace("github.event_name == 'pull_request'", ev),
        );
    }
}

#[test]
fn real_ci_spot_jobs_need_explicit_read_permissions() {
    red(
        "real-noperm",
        &real_mut("    permissions:\n      contents: read\n", ""),
    );
}

// ---- r3 mutants, kept red ----

#[test]
fn r3_workflow_level_mutants_fail() {
    red_why(
        "r3-top-env",
        &real_mut("\nenv:\n", "\nenv:\n  TOK: ${{ secrets.FOO }}\n"),
        "workflow level names secrets",
    );
    red(
        "r3-top-perm",
        &real_mut("\njobs:\n", "\npermissions:\n  contents: write\njobs:\n"),
    );
    red(
        "r3-top-perm-all",
        &real_mut("\njobs:\n", "\npermissions: write-all\njobs:\n"),
    );
}

#[test]
fn r3_block_list_runs_on_with_secrets_env_fails() {
    let m = SPOT_JOB
        .lines()
        .map(|l| {
            if l.starts_with("    runs-on:") {
                "    env:\n      T: ${{ secrets.FOO }}\n    runs-on:\n      - self-hosted\n      - burst-x64".to_string()
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    red("r3-block-list", &m);
}

#[test]
fn r3_matrix_label_fails() {
    red(
        "r3-matrix",
        "on: pull_request\njobs:\n  lockfile:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: [burst-x64]\n    permissions:\n      contents: read\n    steps: []\n",
    );
}

#[test]
fn r3_or_true_fails() {
    red(
        "r3-or-true",
        &real_mut(
            "vars.PR_SPOT_OPTIN == '1')",
            "vars.PR_SPOT_OPTIN == '1' || true)",
        ),
    );
    red(
        "r3-or-true2",
        &real_mut(
            "github.event_name == 'pull_request' && vars.PR_SPOT_OPTIN",
            "github.event_name == 'pull_request' || vars.PR_SPOT_OPTIN",
        ),
    );
}

#[test]
fn r3_reusable_runs_on_variants_fail() {
    for v in [
        "runs_on: ${{ vars.X }}",
        "runs_on: >-\n        ${{ vars.X }}",
        "runs_on: ${{ inputs.x }}",
        "runs_on: '[\"self-hosted\",\"burst-x64\"]'",
    ] {
        red(
            "r3-reusable",
            &real_mut("runs_on: '[\"self-hosted\",\"clean-room\",\"X64\"]'", v),
        );
    }
}

#[test]
fn r3_burst_in_non_allowlisted_job_fails() {
    red("r3-other-job", &SPOT_JOB.replace("  lockfile:", "  other:"));
    let m = real_mut(
        "vars.BURST_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\"]')",
        "vars.PR_SPOT_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\",\"burst-public\"]')",
    );
    red("r3-wrong-canon", &m);
}

#[test]
fn r3_opt_in_variable_anywhere_else_fails() {
    red(
        "r3-var-step",
        &real_mut(
            "    steps:\n      - uses: actions/checkout",
            "    steps:\n      - if: ${{ vars.PR_SPOT_OPTIN == '1' }}\n        uses: actions/checkout",
        ),
    );
    red(
        "r3-ro-var",
        &real_mut(
            "runs-on: [self-hosted, clean-room, X64]",
            "runs-on: ${{ vars.POOL }}",
        ),
    );
}

#[test]
fn r3_new_workflow_with_burst_fails_in_either_extension() {
    for ext in ["yml", "yaml"] {
        let d = scratch("r3-new", "on: push\njobs: {}\n");
        fs::write(
            d.join(format!("x.{ext}")),
            "on: push\njobs:\n  a:\n    runs-on: [self-hosted, burst-x64]\n    steps: []\n",
        )
        .unwrap();
        assert!(!guard(&d), "{ext}");
    }
}

#[test]
fn r3_duplicate_job_or_runs_on_fails() {
    red(
        "r3-dup-runs-on",
        &SPOT_JOB.replace(
            "    permissions:",
            "    runs-on: [self-hosted, clean-room]\n    permissions:",
        ),
    );
    red(
        "r3-dup-job",
        &format!("{SPOT_JOB}  lockfile:\n    runs-on: [self-hosted, clean-room]\n    steps: []\n"),
    );
}

#[test]
fn r3_flow_jobs_burst_fails() {
    red(
        "r3-flow-jobs",
        "on: pull_request\njobs: {lockfile: {runs-on: burst-x64}}\n",
    );
}

// ---- r4: YAML spellings that resolve to the same value, planted RED first ----

fn plant(job_body: &str) -> String {
    format!("on: pull_request\njobs:\n{job_body}")
}

#[test]
fn r4_quoted_key_and_escape_fail() {
    red(
        "r4-quoted-key",
        &plant("  evil:\n    \"runs-on\": [\"self-hosted\", \"burst-x64\"]\n    steps: []\n"),
    );
    red(
        "r4-escape",
        &plant("  evil:\n    runs-on: [\"self-hosted\", \"bur\\x73t-x64\"]\n    steps: []\n"),
    );
    red(
        "r4-quoted-key-escape-secret",
        &plant("  evil:\n    \"runs-on\": [\"self-hosted\", \"bur\\x73t-x64\"]\n    env:\n      T: \"${{ secrets.TOK }}\"\n    steps: []\n"),
    );
}

#[test]
fn r4_flow_style_and_explicit_key_fail() {
    red(
        "r4-flow-job",
        &plant("  evil: {\"runs-on\": [\"self-hosted\", \"bur\\x73t-x64\"]}\n"),
    );
    red(
        "r4-explicit-key",
        &plant("  evil:\n    ? runs-on\n    : [self-hosted, bur\\x73t-x64]\n    steps: []\n")
            .replace("bur\\x73t-x64]\n", "burst-x64]\n"),
    );
    red(
        "r4-explicit-key-escaped",
        &plant(
            "  evil:\n    ? runs-on\n    : [\"self-hosted\", \"bur\\x73t-x64\"]\n    steps: []\n",
        ),
    );
    red(
        "r4-flow-all",
        "{on: pull_request, jobs: {evil: {runs-on: [self-hosted, burst-x64]}}}\n",
    );
}

#[test]
fn r4_expression_runs_on_fails() {
    // fed from a needs output
    red(
        "r4-needs",
        &plant("  pick:\n    runs-on: [self-hosted, clean-room]\n    outputs:\n      l: ${{ steps.s.outputs.l }}\n    steps: []\n  evil:\n    needs: pick\n    runs-on: ${{ fromJSON(needs.pick.outputs.l) }}\n    steps: []\n"),
    );
    // matrix fed from needs output or vars
    red(
        "r4-matrix-needs",
        &plant("  evil:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: ${{ fromJSON(needs.pick.outputs.l) }}\n    steps: []\n"),
    );
    red(
        "r4-matrix-vars",
        &plant("  evil:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: ${{ fromJSON(vars.LBL) }}\n    steps: []\n"),
    );
    red(
        "r4-vars",
        &plant("  evil:\n    runs-on: ${{ vars.LBL }}\n    steps: []\n"),
    );
    red(
        "r4-matrix-include-burst",
        &plant("  evil:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: [a]\n        include:\n          - os: bur\\x73t-x64\n    steps: []\n").replace("bur\\x73t", "burst"),
    );
    red(
        "r4-matrix-no-strategy",
        &plant("  evil:\n    runs-on: ${{ matrix.os }}\n    steps: []\n"),
    );
    // a literal matrix is fine
    assert!(guard(&scratch(
        "r4-matrix-literal-ok",
        &plant("  fine:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: [ubuntu-latest, macos-latest]\n    steps: []\n"),
    )));
}

#[test]
fn r4_map_form_runs_on_fails() {
    red(
        "r4-group-labels",
        &plant("  evil:\n    runs-on:\n      group: g\n      labels: [self-hosted, \"burst-x64\"]\n    steps: []\n"),
    );
    red(
        "r4-labels-escaped",
        &plant("  evil:\n    runs-on:\n      labels: \"bur\\x73t-x64\"\n    steps: []\n"),
    );
    red(
        "r4-unmodeled-shape",
        &plant("  evil:\n    runs-on: 5\n    steps: []\n"),
    );
    red(
        "r4-unmodeled-map-key",
        &plant("  evil:\n    runs-on:\n      other: x\n    steps: []\n"),
    );
}

#[test]
fn r4_reusable_call_passing_pool_value_fails() {
    red(
        "r4-reusable-quoted",
        &plant("  call:\n    uses: ./.github/workflows/x.yml\n    with:\n      \"runs_on\": '[\"self-hosted\",\"bur\\x73t-x64\"]'\n"),
    );
    red(
        "r4-reusable-vars",
        &plant("  call:\n    uses: ./.github/workflows/x.yml\n    with:\n      runs_on: ${{ vars.X }}\n"),
    );
    red(
        "r4-reusable-other-key",
        &plant("  call:\n    uses: ./.github/workflows/x.yml\n    with:\n      pool: burst-x64\n"),
    );
}

#[test]
fn r4_escaped_secrets_fail() {
    red_why(
        "r4-job-secret",
        &SPOT_JOB.replace(
            "steps: []",
            "env:\n      T: \"${{ s\\x65crets.GH_TOKEN }}\"\n    steps: []",
        ),
        "names secrets",
    );
    red_why(
        "r4-top-secret",
        &real_mut("\nenv:\n", "\nenv:\n  T: \"${{ s\\x65crets.GH_TOKEN }}\"\n"),
        "workflow level names secrets",
    );
    red_why(
        "r4-lockfile-secret",
        &job_mut(
            "lockfile",
            "    env:\n",
            "    env:\n      T: \"${{ s\\x65crets.GH_TOKEN }}\"\n",
        ),
        "names secrets",
    );
}

#[test]
fn r4_anchor_and_merge_and_tags_fail() {
    red(
        "r4-anchor",
        &plant("  a:\n    runs-on: &r [self-hosted, burst-x64]\n    steps: []\n  b:\n    runs-on: *r\n    steps: []\n"),
    );
    red(
        "r4-merge-key",
        &plant("  a:\n    <<: {runs-on: [self-hosted, burst-x64]}\n    steps: []\n"),
    );
    red(
        "r4-core-tag",
        &plant("  a:\n    runs-on: !!seq [self-hosted, burst-x64]\n    steps: []\n"),
    );
    red(
        "r4-custom-tag",
        &plant("  a:\n    runs-on: !pool [self-hosted, x]\n    steps: []\n"),
    );
    red(
        "r4-anchor-secret-into-lockfile",
        &SPOT_JOB
            .replace(
                "on: pull_request\n",
                "on: pull_request\nx: &t ${{ secrets.X }}\n",
            )
            .replace("steps: []", "env:\n      T: *t\n    steps: []"),
    );
}

#[test]
fn r4_cannot_judge_is_red() {
    red("r4-unparseable", "on: pull_request\njobs: [\n");
    red(
        "r4-dup-key",
        &plant("  a:\n    runs-on: [self-hosted]\n    runs-on: [self-hosted]\n    steps: []\n"),
    );
    red("r4-dup-job", &plant("  a:\n    runs-on: [self-hosted]\n    steps: []\n  a:\n    runs-on: [self-hosted]\n    steps: []\n"));
    red("r4-not-mapping", "- a\n- b\n");
    red("r4-jobs-not-mapping", "on: push\njobs: x\n");
    red("r4-no-jobs", "on: push\n");
    red("r4-job-not-mapping", "on: push\njobs:\n  a: x\n");
    red("r4-no-runs-on", &plant("  a:\n    steps: []\n"));
}

// ---- the gate step must run the parsed check and its exit code must not be masked ----

fn ci_value() -> Value {
    serde_yaml_ng::from_str(&ci_text()).unwrap()
}

#[test]
fn ci_wires_the_parsed_guard_unmasked_into_the_gate() {
    let ci = ci_value();
    let jobs = ci.get("jobs").unwrap();
    let g = jobs
        .get("spot-labels-guard")
        .expect("spot-labels-guard job");
    assert!(
        g.get("continue-on-error").is_none(),
        "guard job continue-on-error"
    );
    let steps = g.get("steps").and_then(Value::as_sequence).unwrap();
    let runs: Vec<&str> = steps
        .iter()
        .filter_map(|s| s.get("run").and_then(Value::as_str))
        .collect();
    let step = runs
        .iter()
        .find(|r| r.contains("falsification_pr_spot_labels"))
        .expect("a step runs the parsed guard test");
    assert!(step.contains("cargo test --locked --test falsification_pr_spot_labels"));
    for s in steps {
        assert!(
            s.get("continue-on-error").is_none(),
            "step continue-on-error"
        );
    }
    for r in &runs {
        for bad in [
            "|| true",
            "| tail",
            "| head",
            "| grep",
            "|| exit 0",
            "set +e",
            "; true",
        ] {
            assert!(
                !r.contains(bad),
                "run step masks its exit code with `{bad}`: {r}"
            );
        }
    }
    let gate = jobs.get("gate").unwrap();
    let needs: Vec<&str> = gate
        .get("needs")
        .and_then(Value::as_sequence)
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        needs.contains(&"spot-labels-guard"),
        "gate does not need the guard job"
    );
    let gate_text = serde_yaml_ng::to_string(gate).unwrap();
    assert!(
        gate_text.contains("needs.spot-labels-guard.result"),
        "gate never reads the guard result"
    );
}

#[test]
fn ci_step_masking_mutants_are_refused() {
    // the wiring test above must be able to fail: a masked guard step is not accepted
    let masked = ci_text().replace(
        "cargo test --locked --test falsification_pr_spot_labels",
        "cargo test --locked --test falsification_pr_spot_labels || true",
    );
    assert_ne!(masked, ci_text());
    let v: Value = serde_yaml_ng::from_str(&masked).unwrap();
    let step = v["jobs"]["spot-labels-guard"]["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .filter_map(|s| s.get("run").and_then(Value::as_str))
        .any(|r| r.contains("|| true"));
    assert!(step, "mutant did not apply to the guard step");
}

// ---- r5: GitHub routes by label SUBSET (rule (e)) ----

/// A benign ci.yml plus one extra workflow file `x.yml`.
fn with_x(name: &str, x: &str) -> PathBuf {
    let d = scratch(name, "on: push\njobs: {}\n");
    fs::write(d.join("x.yml"), x).unwrap();
    d
}

/// A PR-triggered job that holds a secret, on the given runs-on text.
fn secret_job(on: &str, runs_on: &str) -> String {
    format!(
        "on: {on}\njobs:\n  j:\n    runs-on: {runs_on}\n    env:\n      T: ${{{{ secrets.GH_TOKEN }}}}\n    steps: []\n"
    )
}

fn r5_red(name: &str, x: &str, needle: &str) {
    match judge(&with_x(name, x), &expected_canary()) {
        Ok(_) => panic!("r5 plant {name} was accepted:\n{x}"),
        Err(w) => assert!(
            w.iter().any(|m| m.contains(needle)),
            "r5 plant {name} refused for the wrong reason (want `{needle}`): {w:?}"
        ),
    }
}

#[test]
fn r5_the_five_lane_plants_fail() {
    for (n, ro) in [
        ("self-hosted", "[self-hosted]"),
        ("linux", "[self-hosted, Linux]"),
        ("tier2", "[self-hosted, tier2]"),
        ("ffmpeg", "[self-hosted, ffmpeg]"),
        ("group-tier2", "{group: tier2}"),
    ] {
        r5_red(
            n,
            &secret_job("pull_request", ro),
            "can land on a spot seat",
        );
    }
}

#[test]
fn r5_subset_variants_fail() {
    for (n, ro) in [
        ("scalar", "self-hosted"),
        ("case", "[SELF-HOSTED, linux, TIER2]"),
        ("x64-tier2", "[self-hosted, Linux, X64, tier2, ffmpeg]"),
        ("order", "[ffmpeg, tier2, self-hosted, linux]"),
        ("empty", "[]"),
        ("group-and-labels", "{group: tier2, labels: [clean-room]}"),
        ("group-unlisted", "{group: some-new-group}"),
        ("group-expr", "{group: \"${{ vars.G }}\"}"),
        ("labels-only-spot", "{labels: [self-hosted, tier2]}"),
        ("odd-key", "{group: public-build, extra: x}"),
        ("expr-inputs", "${{ inputs.r }}"),
        ("expr-fromjson", "${{ fromJSON(vars.R) }}"),
        ("label-expr", "[self-hosted, \"${{ vars.L }}\"]"),
    ] {
        r5_red(n, &secret_job("pull_request", ro), "spot");
    }
}

#[test]
fn r5_every_pr_like_trigger_counts() {
    for (n, on) in [
        ("list", "[push, pull_request]"),
        ("target", "pull_request_target"),
        ("review", "{pull_request_review: {}}"),
        ("merge-group", "merge_group"),
        ("call", "workflow_call"),
        (
            "run",
            "{workflow_run: {workflows: [a], types: [completed]}}",
        ),
    ] {
        r5_red(
            n,
            &secret_job(on, "[self-hosted, tier2]"),
            "can land on a spot seat",
        );
    }
    // an unreadable `on` counts as reachable
    r5_red(
        "no-on",
        &secret_job("{}", "[self-hosted]").replace("on: {}\n", ""),
        "can land",
    );
}

#[test]
fn r5_matrix_options_are_judged_one_by_one() {
    let m = |opts: &str| {
        format!(
            "on: pull_request\njobs:\n  j:\n    runs-on: ${{{{ matrix.os }}}}\n    strategy:\n      matrix:\n{opts}\n    steps: []\n"
        )
    };
    r5_red(
        "m-list",
        &m("        os: [[self-hosted, clean-room], [self-hosted, tier2]]"),
        "can land on a spot seat",
    );
    r5_red(
        "m-include",
        &m("        include:\n          - os: [self-hosted, Linux]"),
        "can land on a spot seat",
    );
    r5_red("m-none", &m("        other: [a]"), "no literal options");
    assert!(judge(
        &with_x("m-ok", &m("        include:\n          - os: [self-hosted, clean-room]\n          - os: [self-hosted, clean-room, ARM64]")),
        &expected_canary()
    )
    .is_ok());
}

#[test]
fn r5_exclusive_labels_and_non_spot_groups_pass() {
    for (n, ro) in [
        ("clean-room", "[self-hosted, clean-room, X64]"),
        ("gate-light", "[self-hosted, gate-light]"),
        ("group", "{group: public-build}"),
        (
            "group-labels",
            "{group: gate-light-shared, labels: [self-hosted, tier2]}",
        ),
        ("labels-map", "{labels: [self-hosted, clean-room]}"),
    ] {
        let d = with_x(n, &secret_job("pull_request", ro));
        assert!(judge(&d, &expected_canary()).is_ok(), "{n} refused");
    }
    // a workflow no PR-like event can start is out of this rule's scope
    let d = with_x("push-only", &secret_job("push", "[self-hosted, tier2]"));
    assert!(judge(&d, &expected_canary()).is_ok());
}

#[test]
fn r5_measured_snapshot_is_self_consistent() {
    assert!(SPOT_SEAT_LABELS.iter().all(|l| *l == l.to_lowercase()));
    assert!(!NON_SPOT_GROUPS.contains(&"tier2"));
    assert!(!NON_SPOT_GROUPS.contains(&"burst-public"));
    assert!(!SPOT_SEAT_LABELS.contains(&"clean-room"));
    assert!(!SPOT_SEAT_LABELS.contains(&"gate-light"));
}
