//! Census of the canary fixture: builds the census repo and judges it.
#![allow(dead_code)]
use super::*;

/// Commit `rec.json` at `.quorum/x.json` inside the fixture repo.
fn census_commit(root: &Path) {
    put(
        root,
        ".quorum/x.json",
        &std::fs::read_to_string(root.join("rec.json")).expect("rec"),
    );
    git(root, &["add", "-A"]);
    git(
        root,
        &["commit", "--allow-empty", "-qm", "committed receipt"],
    );
}

fn format_census(res: Result<CensusSummary, String>) -> String {
    match res {
        Ok(s) if s.canary_pending > 0 => "pending".into(),
        Ok(_) => "ok".into(),
        Err(e) => e,
    }
}

/// Commit `rec.json` at `.quorum/x.json` and run the census over the fixture repo.
fn census_commit_and_run(
    script: &Path,
    root: &Path,
    skip_canary: bool,
    pending_reachable: bool,
) -> String {
    census_commit(root);
    format_census(census_all_mode(
        script,
        root,
        skip_canary,
        pending_reachable,
    ))
}

fn squash_prep(root: &Path, o: &Opt, text: &str, base: &str, head: &str) {
    // A squash merge: one ordinary commit S on the base, holding the PR's tree
    // (receipt included), so the receipt's reviewed_head (the PR head) is no
    // ancestor of anything on main.
    let ev = git(root, &["rev-parse", "HEAD"]);
    git(root, &["checkout", "-q", "--detach", base]);
    git(root, &["read-tree", "-u", "--reset", &ev]);
    git(root, &["commit", "-qm", "squash"]);
    let sq = git(root, &["rev-parse", "HEAD"]);
    if o.squash > 1 {
        let post = text.replace(head, &sq);
        put(root, "rec.json", &post);
        git(root, &["add", "-A"]);
        git(
            root,
            &["commit", "--allow-empty", "-qm", "post-merge receipt"],
        );
    }
}

/// A squash-merge shaped history from the committed `rec.json`: the answer the
/// pick and the predicate give (or, with `o.census`, the census of rec.json).
fn squash_answer(
    script: &Path,
    root: &Path,
    o: &Opt,
    text: &str,
    base: &str,
    head: &str,
) -> String {
    squash_prep(root, o, text, base, head);
    if o.census {
        return census_commit_and_run(
            script,
            root,
            o.census_skip_canary,
            o.census_pending_reachable,
        );
    }
    let call = r#". "$1"; s="$(git rev-parse HEAD~1)"; [ "$2" = 1 ] && s="$(git rev-parse HEAD)"; r="$(dogfood_receipt_pick "$s" rec.json)"; dogfood_receipt_status "$r" "$s" "${s}^1" "$s" HEAD"#;
    let out = Command::new("bash")
        .args(["-c", call, "x"])
        .arg(script)
        .arg(o.squash.to_string())
        .current_dir(root)
        .output()
        .expect("bash");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn build_repo(fx: &Fx, o: &Opt) -> (tempfile::TempDir, String, String, String, String, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    set_repo(root);
    put(root, "src/a.rs", "l1\nl2\nl3\n");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "base"]);
    let base = git(root, &["rev-parse", "HEAD"]);
    if o.multi_commit && !o.delete_only {
        put(root, "src/a.rs", "l1\nl2\nl3\nfn added_one() {}\n");
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "c1"]);
    }
    let changed = if o.delete_only {
        "l1\nl2\n"
    } else {
        "l1\nl2\nl3\nfn added_one() {}\nfn added_two() {}\n"
    };
    put(root, "src/a.rs", changed);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "head"]);
    let head = git(root, &["rev-parse", "HEAD"]);
    put(root, "src/b.rs", "x\n");
    if o.stale_first {
        put(root, ".quorum/evidence/claims.json", "[]");
    }
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "other"]);
    let other = git(root, &["rev-parse", "HEAD"]);
    let mut r = fx.receipt.clone();
    let mut files = Vec::new();
    let mut evid = String::new();
    let raw = o.claims_raw.clone();
    for (name, v, role) in [
        ("claims.json", &fx.claims, "claims-blind"),
        ("plan.json", &fx.plan, "canary-plan"),
        ("verdicts.json", &fx.verdicts, "judge-verdicts"),
    ] {
        let body = evidence_body(name, v, &raw, o);
        if o.no_commit != Some(name) {
            put(root, &format!(".quorum/evidence/{name}"), &body);
        }
        files.push(json!({"path": format!(".quorum/evidence/{name}"), "roles": [role], "sha256": sha256(&body)}));
    }
    if o.evid_head {
        git(root, &["add", "-A"]);
        git(root, &["commit", "-qm", "evidence files"]);
        evid = git(root, &["rev-parse", "HEAD"]);
    }
    let sub = |s: &str| {
        s.replace("@HEAD", &head)
            .replace("@BASE", &base)
            .replace("@OTHER", &other)
            .replace("@EVID", &evid)
    };
    if let Some(p) = o.extra_ev {
        files.push(json!({"path": p, "roles": ["code"], "sha256": "0".repeat(64)}));
    }
    if !o.skip_hash {
        r["evidence"]["files"] = Value::Array(files);
    }
    if let Some(c) = r["quorum"].get_mut("canary").and_then(Value::as_object_mut) {
        c.entry("reviewed_base").or_insert(json!(base));
        c.entry("reviewed_head").or_insert(json!(head));
    }
    let raw_rec = sub(&r.to_string());
    put(
        root,
        "rec.json",
        &o.rec_edit.map_or(raw_rec.clone(), |e| e(raw_rec.clone())),
    );
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "evidence"]);
    (dir, base, head, other, evid, raw_rec)
}

/// Build the fixture repo, commit `rec.json` as `.quorum/x.json`, and return the
/// structured census summary.
pub fn run_census(script: &Path, fx: &Fx, o: &Opt) -> Result<CensusSummary, String> {
    let (dir, base, head, _other, _evid, raw_rec) = build_repo(fx, o);
    let root = dir.path();
    if o.squash > 0 {
        squash_prep(root, o, &raw_rec, &base, &head);
    }
    census_commit(root);
    census_all_mode(
        script,
        root,
        o.census_skip_canary,
        o.census_pending_reachable,
    )
}

/// What the predicate says for `fx` under `o`. Evidence is committed in a
/// final commit; the subject (head, base) defaults to the commit that adds the
/// changed lines and its parent.
pub fn run(script: &Path, fx: &Fx, o: &Opt) -> String {
    let (dir, base, head, other, evid, raw_rec) = build_repo(fx, o);
    let root = dir.path();
    if o.squash > 0 {
        return squash_answer(script, root, o, &raw_rec, &base, &head);
    }
    if o.census {
        return census_commit_and_run(
            script,
            root,
            o.census_skip_canary,
            o.census_pending_reachable,
        );
    }
    let sub = |s: &str| {
        s.replace("@HEAD", &head)
            .replace("@BASE", &base)
            .replace("@OTHER", &other)
            .replace("@EVID", &evid)
    };
    let mut args: Vec<String> = Vec::new();
    if !o.no_subject {
        args.push(sub(o.subj_head.as_deref().unwrap_or("@HEAD")));
        args.push(sub(o.subj_base.as_deref().unwrap_or("@BASE")));
        for r in &o.revs {
            args.push(sub(r));
        }
    }
    let call = if o.resolve_empty {
        r#". "$1"; r="$(<rec.json)"; shift; dogfood_receipt_resolve "" "$r" "$@""#
    } else {
        r#". "$1"; r="$(<rec.json)"; shift; dogfood_receipt_status "$r" "$@""#
    };
    let mut cmd = Command::new(if o.shim.is_some() {
        "/bin/bash"
    } else {
        "bash"
    });
    cmd.args(["-c", call, "x"])
        .arg(script)
        .args(&args)
        .current_dir(root);
    if let Some(sh) = o.shim {
        cmd.env("PATH", shim_path(dir.path(), sh));
    }
    let out = cmd.output().expect("bash");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

pub fn set_claims(fx: &mut Fx, i: usize, k: &str, v: Value) {
    fx.claims[i][k] = v;
}

pub type Edit = fn(&mut Fx);

/// (name, edit, run options, expected: "ok" exactly, or a substring of the refusal).
pub type Case = (&'static str, Edit, OptEdit, &'static str);

/// The jq program of main at 9f158fad, verbatim: what every receipt that
/// carries no canary key must still be told.
pub const OLD_JQ: &str = r#"
  def num($v): if ($v | type) == "number" then $v else -1 end;
  def len($v): if ($v | type) == "array" then ($v | length) else -1 end;
  if ([paths | .[-1]? | tostring] | any(test("waiv|override")))
    then "carries a waiver/override key: a waived quorum is an unrefuted claim, not a passed one"
  elif len(.quorum.lanes) < 3
    then "quorum.lanes = \(len(.quorum.lanes)), below the floor of 3 that scripts/quorum-gate.sh enforces at push time"
  elif num(.quorum.judges) < 3
    then "quorum.judges = \(num(.quorum.judges)), below the floor of 3: a majority needs three"
  elif num(.quorum.refuters_per_claim) < 3
    then "quorum.refuters_per_claim = \(num(.quorum.refuters_per_claim)), below the floor of 3: a claim one refuter failed to kill only met one refuter"
  elif num(.quorum.claims_refuted) < 1
    then "quorum.claims_refuted = \(num(.quorum.claims_refuted)): a round that killed no claim did not hunt, which is the vacuous receipt this floor exists to refuse"
  elif len(.evidence.files) < 1
    then "evidence.files is empty: a verdict that cites no file is an opinion"
  else "ok"
  end
"#;

pub fn jq(body: &str, prog: &str) -> String {
    jq_untrimmed(body, prog).trim().to_string()
}

/// `jq -r` output with its trailing newline kept: a value that ends in a newline must not
/// be trimmed into a valid one.
pub fn jq_untrimmed(body: &str, prog: &str) -> String {
    let mut ch = Command::new("jq")
        .args(["-j", prog])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("jq");
    std::io::Write::write_all(&mut ch.stdin.take().expect("stdin"), body.as_bytes()).expect("feed");
    String::from_utf8_lossy(&ch.wait_with_output().expect("wait").stdout).to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CensusKind {
    Plain,
    CanaryJudged,
    CanaryPending,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CensusSummary {
    pub total: usize,
    pub ok: usize,
    pub zero: usize,
    pub plain: usize,
    pub canary_judged: usize,
    pub canary_pending: usize,
    pub pending_files: Vec<String>,
}

/// What one committed receipt (`rel`, under `root`) is told, split in three.
/// A receipt with no `quorum.canary` block (`Plain`) must be told exactly what
/// main's program tells it. A receipt WITH one must carry a string
/// `reviewed_head` and a truthful `claims_refuted` verdict from `OLD_JQ`
/// (`OLD_ZERO` when 0, `"ok"` when >= 1). If `reviewed_head` does not resolve
/// in history or is not an ancestor of `HEAD`, it is `CanaryPending` (awaits
/// its post-merge receipt). If `reviewed_head` IS an ancestor of `HEAD`, it is
/// never pending: `reviewed_base` must resolve and be an ancestor of
/// `reviewed_head`, and the bound path must say `"ok"` (`CanaryJudged`).
/// Ok is (CensusKind, main's answer).
pub fn census_one(script: &Path, root: &Path, rel: &str) -> Result<(CensusKind, String), String> {
    census_one_mode(script, root, rel, false)
}

fn census_git_ok(root: &Path, rel: &str, gargs: &[&str]) -> Result<bool, String> {
    Command::new("git")
        .args(gargs)
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .map_err(|e| format!("{rel}: git: {e}"))
}

fn census_run_status(
    script: &Path,
    root: &Path,
    rel: &str,
    args: &[&str],
) -> Result<String, String> {
    let out = Command::new("bash")
        .args([
            "-c",
            r#". "$1"; f="$2"; shift 2; dogfood_receipt_status "$(cat "$f")" "$@""#,
            "x",
        ])
        .arg(script)
        .arg(root.join(rel))
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("{rel}: bash: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn census_plain(
    script: &Path,
    root: &Path,
    rel: &str,
    old: String,
) -> Result<(CensusKind, String), String> {
    let new = census_run_status(script, root, rel, &[])?;
    if new != old {
        return Err(format!("{rel}: the verdict changed: {new:?} for {old:?}"));
    }
    Ok((CensusKind::Plain, old))
}

fn census_canary_head(body: &str, rel: &str) -> Result<String, String> {
    let head = jq_untrimmed(body, r#".quorum.canary.reviewed_head | strings"#);
    let is_sha = head.len() == 40 && head.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if head.is_empty() || !is_sha {
        return Err(format!(
            "{rel}: canary receipt is missing or its reviewed_head is not a 40-hex commit id"
        ));
    }
    Ok(head)
}

fn census_check_main_verdict(body: &str, rel: &str, old: &str) -> Result<(), String> {
    let want_old = jq(
        body,
        r#"if (.quorum.claims_refuted | type) == "number" and .quorum.claims_refuted == (.quorum.claims_refuted | floor) then (if .quorum.claims_refuted == 0 then "zero" elif .quorum.claims_refuted >= 1 then "ok" else "bad" end) else "bad" end"#,
    );
    if want_old == "zero" && old != OLD_ZERO {
        return Err(format!(
            "{rel}: a canary receipt is told {old:?} by main, not the zero-refuted refusal"
        ));
    }
    if want_old == "ok" && old != "ok" {
        return Err(format!(
            "{rel}: a canary receipt with claims_refuted >= 1 is told {old:?} by main, not 'ok'"
        ));
    }
    if want_old != "zero" && want_old != "ok" {
        return Err(format!(
            "{rel}: a canary receipt has invalid claims_refuted (main gave {old:?})"
        ));
    }
    Ok(())
}

fn census_head_on_main(root: &Path, rel: &str, head: &str) -> Result<bool, String> {
    let head_rev = format!("{head}^{{commit}}");
    if !census_git_ok(root, rel, &["rev-parse", "--verify", "-q", &head_rev])? {
        return Ok(false);
    }
    census_git_ok(root, rel, &["merge-base", "--is-ancestor", head, "HEAD"])
}

fn census_canary_base(root: &Path, rel: &str, body: &str, head: &str) -> Result<String, String> {
    let base = jq(body, r#".quorum.canary.reviewed_base | strings"#);
    let base_rev = format!("{base}^{{commit}}");
    if base.is_empty() || !census_git_ok(root, rel, &["rev-parse", "--verify", "-q", &base_rev])? {
        return Err(format!(
            "{rel}: canary receipt names base {base:?}, which does not resolve in history"
        ));
    }
    if !census_git_ok(root, rel, &["merge-base", "--is-ancestor", &base, head])? {
        return Err(format!(
            "{rel}: canary receipt names base {base:?}, which is not an ancestor of {head:?}"
        ));
    }
    Ok(base)
}

fn census_bound_path(
    script: &Path,
    root: &Path,
    rel: &str,
    base: &str,
    head: &str,
    old: String,
) -> Result<(CensusKind, String), String> {
    let args = [head, base, head, "HEAD"];
    let new = census_run_status(script, root, rel, &args)?;
    if new != "ok" {
        return Err(format!("{rel}: {new}"));
    }
    Ok((CensusKind::CanaryJudged, old))
}

fn census_one_mode(
    script: &Path,
    root: &Path,
    rel: &str,
    pending_reachable: bool,
) -> Result<(CensusKind, String), String> {
    let body = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
    let old = jq(&body, OLD_JQ);
    let has = jq(
        &body,
        r#"if (.quorum | type == "object") and (.quorum | has("canary")) then "yes" else "no" end"#,
    );
    if has != "yes" {
        return census_plain(script, root, rel, old);
    }
    let head = census_canary_head(&body, rel)?;
    census_check_main_verdict(&body, rel, &old)?;
    let head_on_main = census_head_on_main(root, rel, &head)?;
    if !head_on_main || pending_reachable {
        return Ok((CensusKind::CanaryPending, old));
    }
    let base = census_canary_base(root, rel, &body, &head)?;
    census_bound_path(script, root, rel, &base, &head, old)
}

/// The committed-receipt census over every `.quorum/*.json` of `root` (never
/// `evidence/`). With `skip_canary` (the mutant) canary receipts are neither
/// compared nor judged.
pub fn census_all(script: &Path, root: &Path, skip_canary: bool) -> Result<CensusSummary, String> {
    census_all_mode(script, root, skip_canary, false)
}

pub fn census_all_mode(
    script: &Path,
    root: &Path,
    skip_canary: bool,
    pending_reachable: bool,
) -> Result<CensusSummary, String> {
    let dir = std::fs::read_dir(root.join(".quorum")).map_err(|e| format!(".quorum: {e}"))?;
    let mut names: Vec<String> = dir
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    let mut summary = CensusSummary::default();
    for n in names {
        let rel = format!(".quorum/{n}");
        let body = std::fs::read_to_string(root.join(&rel)).map_err(|e| format!("{rel}: {e}"))?;
        if skip_canary && body.contains("\"canary\"") {
            summary.total += 1;
            summary.canary_judged += 1;
            continue;
        }
        let (kind, old) = census_one_mode(script, root, &rel, pending_reachable)?;
        summary.total += 1;
        match kind {
            CensusKind::Plain => summary.plain += 1,
            CensusKind::CanaryJudged => summary.canary_judged += 1,
            CensusKind::CanaryPending => {
                summary.canary_pending += 1;
                summary.pending_files.push(rel);
            }
        }
        summary.ok += usize::from(old == "ok");
        summary.zero += usize::from(old == OLD_ZERO);
    }
    Ok(summary)
}
