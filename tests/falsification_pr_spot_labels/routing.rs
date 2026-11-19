//! Rules (e) and (f): label-set routing and reusable-workflow calls.

use crate::*;

// ---- rule (e): a job must carry a label that only a named pool carries ----

/// Pool labels this repository's workflows already use (the every-job-runs-on test pins them). A
/// self-hosted job leaves the spot seats only by carrying one of these. Compared lower-case.
pub(crate) const POOL_LABELS: [&str; 10] = [
    "clean-room",
    "build",
    "intel",
    "gx10",
    "yoga",
    "gate-light",
    "gpu",
    "cuda",
    "mini",
    "mini-builder",
];

/// The GitHub-hosted runner labels, spelled out. Only a label in this list, ALONE, can reach a
/// hosted runner: `self-hosted` anywhere in `runs-on` means self-hosted only, and a label that is
/// not in the list (`ubuntu-public`, `ubuntu-`) is just a label some self-hosted seat may carry.
pub(crate) const HOSTED_LABELS: [&str; 14] = [
    "ubuntu-latest",
    "ubuntu-24.04",
    "ubuntu-22.04",
    "ubuntu-20.04",
    "ubuntu-24.04-arm",
    "ubuntu-22.04-arm",
    "macos-latest",
    "macos-15",
    "macos-14",
    "macos-13",
    "macos-12",
    "windows-latest",
    "windows-2025",
    "windows-2022",
];

/// Ok when this label list names a pool, or is exactly one GitHub-hosted runner label; anything
/// else (including a bare `self-hosted`, or labels no pool is known to carry) can be taken by a
/// spot seat. Routing is by subset: a runner must carry EVERY listed label, so one pool label
/// takes the job off the spot seats, while no hosted label can rescue a list holding `self-hosted`.
pub(crate) fn leaves_spot(opt: &Value) -> Result<(), String> {
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
    let low: Vec<String> = ls.iter().map(|l| l.trim().to_lowercase()).collect();
    let pool = low.iter().any(|l| POOL_LABELS.contains(&l.as_str()));
    let hosted = low.len() == 1 && HOSTED_LABELS.contains(&low[0].as_str());
    if pool || hosted {
        Ok(())
    } else {
        Err(format!(
            "labels {ls:?} name no pool and are not one GitHub-hosted label, so a spot seat can take the job"
        ))
    }
}

pub(crate) fn matrix_options(j: &serde_yaml_ng::Mapping, k: &str) -> Result<Vec<Value>, String> {
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

/// `runs-on` as a mapping: runner groups are refused outright (no workflow here uses one, and a
/// group's membership cannot be read from this repository); `labels` alone is judged.
pub(crate) fn reaches_spot(j: &serde_yaml_ng::Mapping, ro: &Value) -> Result<(), String> {
    match ro {
        Value::Mapping(m) => {
            if let Some(k) = m
                .keys()
                .find(|k| !k.as_str().is_some_and(|s| s.eq_ignore_ascii_case("labels")))
            {
                return Err(format!(
                    "runs-on key {k:?} is refused: only `labels` is modeled, not runner groups"
                ));
            }
            match key(m, "labels") {
                Some(l) => leaves_spot(&l),
                None => Err("empty runs-on mapping".into()),
            }
        }
        Value::String(s) if s.contains("${{") => {
            let k = ctx_ref(s, "matrix")
                .ok_or_else(|| format!("runs-on expression cannot be resolved: {s}"))?;
            matrix_options(j, k)?.iter().try_for_each(leaves_spot)
        }
        other => leaves_spot(other),
    }
}

// ---- rule (f): a job that `uses:` a reusable workflow is judged, never trusted ----

/// The one `secrets` form a call may carry.
#[derive(PartialEq, Eq)]
pub(crate) enum Secrets {
    /// no `secrets:` key at all
    Absent,
    /// the exact string `inherit`, or no key
    Inherit,
}

/// One allowed EXTERNAL reusable-workflow call: scoped to its exact call site, its exact pinned
/// `uses` string, its one `secrets` form and its inputs. Everything else outside this repository
/// is red.
pub(crate) struct ExternalCall {
    pub file: &'static str,
    pub job: &'static str,
    pub uses: &'static str,
    pub secrets: Secrets,
    /// inputs that must be present with exactly this scalar value
    pub required: &'static [(&'static str, &'static str)],
    /// further inputs that may be present; any input not listed here or above is red
    pub optional: &'static [&'static str],
    pub why: &'static str,
}

pub(crate) const EXTERNAL_CALLS: [ExternalCall; 3] = [
    ExternalCall {
        file: "ci.yml",
        job: "ci",
        uses: "paiml/.github/.github/workflows/sovereign-ci.yml@8adbee1d793df71a31867eb875829b55e04a7c02",
        secrets: Secrets::Inherit,
        required: &[("runs_on", REUSABLE_RUNS_ON)],
        optional: &["repo", "use_nextest", "coverage_on", "heavy_on"],
        why: "the shared gate, pinned to a commit; its jobs run on the literal pool passed as runs_on",
    },
    ExternalCall {
        file: "nightly-bench.yml",
        job: "bench",
        uses: "paiml/.github/.github/workflows/sovereign-ci.yml@fbee3788a0ed92d09974ac21ea9bc97d373af6f0",
        secrets: Secrets::Inherit,
        required: &[],
        optional: &["repo", "run_benchmarks"],
        why: "scheduled benchmark run; no runs_on input is accepted, because at this pin the input's \
              default is the literal clean-room list (read from that commit's workflow file)",
    },
    ExternalCall {
        file: "pr-gate.yml",
        job: "authorize",
        uses: "paiml/.github/.github/workflows/pr-gate.yml@fbee3788a0ed92d09974ac21ea9bc97d373af6f0",
        secrets: Secrets::Absent,
        required: &[],
        optional: &[],
        why: "PR authorization; at this pin its job runs on the light gate pool; no inputs, no secrets",
    },
];

/// A `with` value as the called workflow would read it: JSON text becomes a list or map.
pub(crate) fn input_value(v: &Value) -> Value {
    match v {
        Value::String(s) if matches!(s.trim_start().chars().next(), Some('[' | '{')) => {
            serde_yaml_ng::from_str(s).unwrap_or_else(|_| v.clone())
        }
        other => other.clone(),
    }
}

/// Judge one allowed external call against its entry: site, secrets form and inputs.
pub(crate) fn judge_external(
    ctx: &str,
    site: (&str, &str),
    e: &ExternalCall,
    j: &serde_yaml_ng::Mapping,
    why: &mut Vec<String>,
) {
    if (e.file, e.job) != site {
        why.push(format!(
            "{ctx}: external call site {site:?} is not allowed for {}",
            e.uses
        ));
    }
    match (key(j, "secrets"), &e.secrets) {
        (None, _) => {}
        (Some(Value::String(s)), Secrets::Inherit) if s == "inherit" => {}
        (Some(form), _) => why.push(format!(
            "{ctx}: secrets form {form:?} is not allowed for this call (only {})",
            if e.secrets == Secrets::Inherit {
                "`inherit`"
            } else {
                "no secrets"
            }
        )),
    }
    let with = match key(j, "with") {
        Some(Value::Mapping(w)) => w,
        None => serde_yaml_ng::Mapping::new(),
        Some(_) => return why.push(format!("{ctx}: `with` is not a mapping, cannot judge")),
    };
    for k in with.keys() {
        let Some(ks) = k.as_str() else {
            why.push(format!("{ctx}: input key {k:?} is not a string"));
            continue;
        };
        if !e.required.iter().any(|(r, _)| *r == ks) && !e.optional.contains(&ks) {
            why.push(format!("{ctx}: input `{ks}` is not allowed for this call"));
        }
    }
    for (rk, rv) in e.required {
        let got = key(&with, rk);
        if got.as_ref().and_then(Value::as_str) != Some(*rv) {
            why.push(format!(
                "{ctx}: required input `{rk}` is not exactly {rv}: {got:?}"
            ));
        }
    }
}

pub(crate) fn judge_call(
    ctx: &str,
    site: (&str, &str),
    dir: &Path,
    j: &serde_yaml_ng::Mapping,
    depth: usize,
    why: &mut Vec<String>,
) {
    let Some(uses) = key(j, "uses") else { return };
    let Some(u) = uses.as_str() else {
        return why.push(format!("{ctx}: `uses` is not a string, cannot judge"));
    };
    if let Some(local) = u.strip_prefix("./") {
        return judge_local_call(ctx, dir, local, j, depth, why);
    }
    match EXTERNAL_CALLS.iter().find(|e| e.uses == u) {
        None => why.push(format!(
            "{ctx}: external or unresolvable reusable call is not on the external-call allowlist: {u}"
        )),
        Some(e) => judge_external(ctx, site, e, j, why),
    }
}

pub(crate) fn judge_local_call(
    ctx: &str,
    dir: &Path,
    rel: &str,
    j: &serde_yaml_ng::Mapping,
    depth: usize,
    why: &mut Vec<String>,
) {
    let ctx = format!("{ctx} via local call ./{rel}");
    if key(j, "secrets").is_some() {
        why.push(format!(
            "{ctx}: secrets passed to a local call not on an allowlist (inherit or a map)"
        ));
    }
    let name = rel.strip_prefix(".github/workflows/").unwrap_or("");
    let plain = !name.is_empty()
        && !name.contains('/')
        && !name.contains("..")
        && (name.ends_with(".yml") || name.ends_with(".yaml"));
    if !plain || depth >= 3 {
        return why.push(format!(
            "{ctx}: not a plain workflow file under .github/workflows (or nested too deep), cannot judge"
        ));
    }
    let mut inner = Vec::new();
    let Some(Value::Mapping(top)) = parse_file(&dir.join(name), name, &mut inner)
        .and_then(|d| matches!(d, Value::Mapping(_)).then_some(d))
    else {
        inner.push("callee is missing or not a mapping".to_string());
        return why.push(format!("{ctx}: cannot judge: {inner:?}"));
    };
    why.extend(inner.into_iter().map(|w| format!("{ctx}: {w}")));
    let Some(Value::Mapping(jobs)) = key(&top, "jobs") else {
        return why.push(format!("{ctx}: callee jobs missing, cannot judge"));
    };
    let with = match key(j, "with") {
        Some(Value::Mapping(w)) => w,
        _ => serde_yaml_ng::Mapping::new(),
    };
    for (n, cj) in &jobs {
        let cctx = format!("{ctx}: job {}", n.as_str().unwrap_or("?"));
        let Value::Mapping(cm) = cj else {
            why.push(format!("{cctx} is not a mapping, cannot judge"));
            continue;
        };
        match key(cm, "runs-on") {
            None if key(cm, "uses").is_some() => judge_call(
                &cctx,
                (name, n.as_str().unwrap_or("?")),
                dir,
                cm,
                depth + 1,
                why,
            ),
            None => why.push(format!("{cctx}: no runs-on and no uses, cannot judge")),
            Some(ro) => {
                let resolved = ro
                    .as_str()
                    .and_then(|s| ctx_ref(s, "inputs"))
                    .map(|k| {
                        key(&with, k)
                            .map(|v| input_value(&v))
                            .unwrap_or_else(|| Value::String(format!("${{{{ inputs.{k} }}}}")))
                    })
                    .unwrap_or(ro);
                if let Err(e) = reaches_spot(cm, &resolved) {
                    why.push(format!("{cctx}: can land on a spot seat: {e}"));
                }
            }
        }
    }
}
