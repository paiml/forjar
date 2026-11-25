//! Fixture for tests/falsification_dogfood_canary_receipts.rs: a throwaway git repo
//! holding one clean canary round, and the helpers that build and mutate it.
#![allow(dead_code)]
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const SCRIPT: &str = "scripts/dogfood/lib/receipt.sh";
pub const OLD_ZERO: &str = "quorum.claims_refuted = 0: a round that killed no claim did not hunt, which is the vacuous receipt this floor exists to refuse";

pub fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn git(root: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .args(args)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .output()
        .expect("git runs");
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

pub fn put(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
    std::fs::write(p, body).expect("write");
}

pub fn sha256(body: &str) -> String {
    let mut c = Command::new("sha256sum");
    c.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    let mut ch = c.spawn().expect("sha256sum");
    std::io::Write::write_all(&mut ch.stdin.take().expect("stdin"), body.as_bytes()).expect("feed");
    let o = ch.wait_with_output().expect("wait");
    String::from_utf8_lossy(&o.stdout)
        .split_whitespace()
        .next()
        .expect("hash")
        .to_string()
}

pub const A_ADDED_1: &str = "The change adds a function named added_one to src/a.rs at line four.";
pub const A_ADDED_2: &str = "The change adds a function named added_two to src/a.rs at line five.";
pub const P_DELETE: &str = "The change deletes the function added_one from src/a.rs entirely.";
pub const P_FIRST: &str = "The change removes the first line l1 from the src/a.rs file.";

/// The pieces of one clean round. Cases mutate a field, then `run` builds a
/// repo (base, head, evidence commit), hashes the files into the receipt and
/// asks the predicate.
#[derive(Clone)]
pub struct Fx {
    pub claims: Value,
    pub plan: Value,
    pub verdicts: Value,
    pub receipt: Value,
}

pub fn base_fx() -> Fx {
    let planted = json!([
        {"id": "2", "text": P_DELETE, "falsified_by": "src/a.rs:4", "position": 2},
        {"id": "4", "text": P_FIRST, "falsified_by": "src/a.rs:5", "position": 4}
    ]);
    let v = |j: &str, a: &str, b: &str| json!({"judge": j, "verdicts": {"1": "SURVIVED", "2": a, "3": "SURVIVED", "4": b}});
    Fx {
        claims: json!([
            {"id": "1", "text": A_ADDED_1}, {"id": "2", "text": P_DELETE},
            {"id": "3", "text": A_ADDED_2}, {"id": "4", "text": P_FIRST}
        ]),
        plan: json!({"planted": planted}),
        verdicts: json!([
            v("J1", "REFUTED", "REFUTED"),
            v("J2", "REFUTED", "REFUTED"),
            v("J3", "SURVIVED", "REFUTED")
        ]),
        receipt: json!({
            "quorum": {
                "lanes": ["a", "b", "c"], "judges": 3, "refuters_per_claim": 3,
                "claims_reviewed": 2, "claims_confirmed": 2, "claims_refuted": 0,
                "canary": {
                    "planted_by": "courier", "real_claim_ids": ["1", "3"],
                    "planted": planted, "caught": ["2", "4"],
                    "claims_file": ".quorum/evidence/claims.json",
                    "plan_file": ".quorum/evidence/plan.json",
                    "verdicts_file": ".quorum/evidence/verdicts.json"
                }
            },
            "evidence": {"files": []}
        }),
    }
}

pub fn canary(fx: &mut Fx) -> &mut Value {
    &mut fx.receipt["quorum"]["canary"]
}

/// How one run differs from the clean one. The placeholders @HEAD, @BASE and
/// @OTHER (a real commit between head and the evidence commit) are replaced in
/// the receipt text, the subject arguments and the evidence revs.
#[derive(Clone, Default)]
pub struct Opt {
    pub subj_head: Option<String>,
    pub subj_base: Option<String>,
    pub no_subject: bool,
    pub revs: Vec<String>,
    pub skip_hash: bool,
    pub stale_first: bool,
    pub delete_only: bool,
    pub claims_raw: Option<String>,
    pub resolve_empty: bool,
    /// An evidence file named here is hashed into the receipt but never committed.
    pub no_commit: Option<&'static str>,
    /// 1: squash-merge shaped, only the pre-merge receipt (bound to the PR head);
    /// 2: the same plus a post-merge receipt bound to the squash commit.
    pub squash: u8,
    /// A further evidence.files entry (path only; a zero hash) in the receipt.
    pub extra_ev: Option<&'static str>,
    /// Commit the evidence files first, so @EVID names a commit that adds them.
    pub evid_head: bool,
    /// Rewrites the serialized text of one evidence file ("plan.json", ...) before it is hashed.
    pub body_edit: Option<BodyEdit>,
    /// Rewrites the serialized receipt text.
    pub rec_edit: Option<fn(String) -> String>,
    /// (tool, mode): run with a PATH holding every other tool and, for this one,
    /// nothing ("absent"), a script that prints nothing and exits 0 ("silent0")
    /// or one that prints nothing and exits 1 ("silent1").
    pub shim: Option<(&'static str, &'static str)>,
    /// With squash = 2: answer what the census (`census_one`) says of the
    /// committed post-merge receipt, "ok" or its sentence.
    pub census: bool,
    /// The census MUTANT: canary receipts are left out of the comparison and
    /// given no assertion of their own.
    pub census_skip_canary: bool,
}

/// The external tools the receipt predicate and this harness can reach.
const TOOLS: [&str; 17] = [
    "git",
    "jq",
    "grep",
    "mktemp",
    "cat",
    "sha256sum",
    "awk",
    "rm",
    "dirname",
    "env",
    "sed",
    "sort",
    "uniq",
    "tr",
    "head",
    "wc",
    "od",
];

/// A directory to use as the whole PATH: a symlink to each of TOOLS except the
/// shimmed one, which is missing or a silent script.
fn shim_path(dir: &Path, shim: (&str, &str)) -> String {
    let bin = dir.join("shimbin");
    std::fs::create_dir_all(&bin).expect("shim dir");
    for t in TOOLS {
        let dst = bin.join(t);
        if t == shim.0 {
            match shim.1 {
                m if m.starts_with("when:") || m.starts_with("rej:") => {
                    // when:<case pattern on "$*">: silent for that call, the real tool otherwise.
                    let real = ["/usr/bin", "/bin"]
                        .iter()
                        .map(|d| Path::new(d).join(t))
                        .find(|p| p.exists())
                        .expect("real tool");
                    // rej:<pattern>: exit 2 (an option this tool does not know) instead.
                    let (pat, code) = match m.strip_prefix("rej:") {
                        Some(p) => (p, 2),
                        None => (&m["when:".len()..], 0),
                    };
                    std::fs::write(
                        &dst,
                        format!(
                            "#!/bin/sh\ncase \"$*\" in {pat}) exit {code};; esac\nexec {} \"$@\"\n",
                            real.display()
                        ),
                    )
                    .expect("shim");
                    let mut p = std::fs::metadata(&dst).expect("meta").permissions();
                    std::os::unix::fs::PermissionsExt::set_mode(&mut p, 0o755);
                    std::fs::set_permissions(&dst, p).expect("chmod");
                }
                "silent0" | "silent1" => {
                    let code = if shim.1 == "silent0" { 0 } else { 1 };
                    std::fs::write(&dst, format!("#!/bin/sh\nexit {code}\n")).expect("shim");
                    let mut p = std::fs::metadata(&dst).expect("meta").permissions();
                    std::os::unix::fs::PermissionsExt::set_mode(&mut p, 0o755);
                    std::fs::set_permissions(&dst, p).expect("chmod");
                }
                _ => {}
            }
            continue;
        }
        let found = ["/usr/bin", "/bin", "/usr/local/bin"]
            .iter()
            .map(|d| Path::new(d).join(t))
            .find(|p| p.exists());
        if let Some(src) = found {
            std::os::unix::fs::symlink(src, &dst).expect("link");
        }
    }
    bin.to_string_lossy().into_owned()
}

pub type BodyEdit = (&'static str, fn(String) -> String);

pub type OptEdit = fn(&mut Opt);

fn set_repo(root: &Path) {
    git(root, &["init", "-q", "-b", "main"]);
    for kv in [
        ("core.hooksPath", "/dev/null"),
        ("commit.gpgsign", "false"),
        ("user.email", "a@b.c"),
        ("user.name", "n"),
    ] {
        git(root, &["config", kv.0, kv.1]);
    }
}

/// The serialized text of one evidence file, after the run's raw overrides.
fn evidence_body(name: &str, v: &Value, raw: &Option<String>, o: &Opt) -> String {
    let mut body = match (raw, name) {
        (Some(b), "claims.json") => b.clone(),
        _ => serde_json::to_string_pretty(v).expect("json"),
    };
    if let Some((n, edit)) = o.body_edit {
        if n == name {
            body = edit(body);
        }
    }
    body
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
    if o.census {
        // The whole committed-receipt test, over this fixture repo.
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
        return match census_all(script, root, o.census_skip_canary) {
            Ok(_) => "ok".into(),
            Err(e) => e,
        };
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

/// What the predicate says for `fx` under `o`. Evidence is committed in a
/// final commit; the subject (head, base) defaults to the commit that adds the
/// changed lines and its parent.
pub fn run(script: &Path, fx: &Fx, o: &Opt) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    set_repo(root);
    put(root, "src/a.rs", "l1\nl2\nl3\n");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "base"]);
    let base = git(root, &["rev-parse", "HEAD"]);
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
    let rec_text = sub(&r.to_string());
    put(
        root,
        "rec.json",
        &o.rec_edit.map_or(rec_text.clone(), |e| e(rec_text)),
    );
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "evidence"]);
    if o.squash > 0 {
        return squash_answer(script, root, o, &sub(&r.to_string()), &base, &head);
    }
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
    let mut ch = Command::new("jq")
        .args(["-r", prog])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("jq");
    std::io::Write::write_all(&mut ch.stdin.take().expect("stdin"), body.as_bytes()).expect("feed");
    String::from_utf8_lossy(&ch.wait_with_output().expect("wait").stdout)
        .trim()
        .to_string()
}

/// What one committed receipt (`rel`, under `root`) is told, split in two. A
/// receipt with no `quorum.canary` block must be told exactly what main's
/// program tells it. A receipt WITH one is judged by the real bound path, with
/// the commit it vouches for (its `reviewed_head`) and that commit's first
/// parent resolved from the repo's history, as gate E resolves them; it must be
/// ok there, and main's program must refuse it for zero refuted. A commit that
/// does not resolve is an error, never a skip. Ok is (is_canary, main's answer).
pub fn census_one(script: &Path, root: &Path, rel: &str) -> Result<(bool, String), String> {
    let body = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
    let old = jq(&body, OLD_JQ);
    let has = jq(
        &body,
        r#"if (.quorum | type == "object") and (.quorum | has("canary")) then "yes" else "no" end"#,
    );
    let mut args: Vec<String> = Vec::new();
    if has == "yes" {
        let head = jq(&body, r#".quorum.canary.reviewed_head | strings"#);
        let known = Command::new("git")
            .args(["rev-parse", "--verify", "-q", &format!("{head}^{{commit}}")])
            .current_dir(root)
            .output()
            .map_err(|e| format!("{rel}: git: {e}"))?;
        if head.is_empty() || !known.status.success() {
            return Err(format!(
                "{rel}: canary receipt names commit {head:?}, which does not resolve in history"
            ));
        }
        if old != OLD_ZERO {
            return Err(format!(
                "{rel}: a canary receipt is told {old:?} by main, not the zero-refuted refusal"
            ));
        }
        args = vec![head.clone(), format!("{head}^1"), head, "HEAD".into()];
    }
    let out = Command::new("bash")
        .args([
            "-c",
            r#". "$1"; f="$2"; shift 2; dogfood_receipt_status "$(cat "$f")" "$@""#,
            "x",
        ])
        .arg(script)
        .arg(root.join(rel))
        .args(&args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("{rel}: bash: {e}"))?;
    let new = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if has == "yes" {
        return if new == "ok" {
            Ok((true, old))
        } else {
            Err(format!("{rel}: {new}"))
        };
    }
    if new != old {
        return Err(format!("{rel}: the verdict changed: {new:?} for {old:?}"));
    }
    Ok((false, old))
}

/// The committed-receipt census over every `.quorum/*.json` of `root` (never
/// `evidence/`): (total, ok, refused for zero refuted, canary). With
/// `skip_canary` (the mutant) canary receipts are neither compared nor judged.
pub fn census_all(
    script: &Path,
    root: &Path,
    skip_canary: bool,
) -> Result<(usize, usize, usize, usize), String> {
    let dir = std::fs::read_dir(root.join(".quorum")).map_err(|e| format!(".quorum: {e}"))?;
    let mut names: Vec<String> = dir
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    let (mut total, mut ok, mut zero, mut canary) = (0, 0, 0, 0);
    for n in names {
        let rel = format!(".quorum/{n}");
        let body = std::fs::read_to_string(root.join(&rel)).map_err(|e| format!("{rel}: {e}"))?;
        if skip_canary && body.contains("\"canary\"") {
            total += 1;
            canary += 1;
            continue;
        }
        let (is_canary, old) = census_one(script, root, &rel)?;
        total += 1;
        canary += usize::from(is_canary);
        ok += usize::from(old == "ok");
        zero += usize::from(old == OLD_ZERO);
    }
    Ok((total, ok, zero, canary))
}

pub mod cases;
pub mod cases_review;
pub mod cases_round3;
pub mod cases_round6;
pub mod cases_round7;
pub mod mutants;
