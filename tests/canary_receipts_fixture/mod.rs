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
        {"id": "C02", "text": P_DELETE, "falsified_by": "src/a.rs:4", "position": 2},
        {"id": "C04", "text": P_FIRST, "falsified_by": "src/a.rs:5", "position": 4}
    ]);
    let v = |j: &str, a: &str, b: &str| json!({"judge": j, "verdicts": {"C01": "SURVIVED", "C02": a, "C03": "SURVIVED", "C04": b}});
    Fx {
        claims: json!([
            {"id": "C01", "text": A_ADDED_1}, {"id": "C02", "text": P_DELETE},
            {"id": "C03", "text": A_ADDED_2}, {"id": "C04", "text": P_FIRST}
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
                    "planted_by": "courier", "real_claim_ids": ["C01", "C03"],
                    "planted": planted, "caught": ["C02", "C04"],
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

/// What the predicate says for `fx`; `revs` default to HEAD. `skip_hash`
/// keeps a hand-edited sha256 in the receipt instead of recomputing it.
pub fn run(script: &Path, fx: &Fx, revs: &[&str], skip_hash: bool) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    git(root, &["init", "-q", "-b", "main"]);
    for kv in [
        ("core.hooksPath", "/dev/null"),
        ("commit.gpgsign", "false"),
        ("user.email", "a@b.c"),
        ("user.name", "n"),
    ] {
        git(root, &["config", kv.0, kv.1]);
    }
    put(root, "src/a.rs", "l1\nl2\nl3\n");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "base"]);
    let base = git(root, &["rev-parse", "HEAD"]);
    put(
        root,
        "src/a.rs",
        "l1\nl2\nl3\nfn added_one() {}\nfn added_two() {}\n",
    );
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "head"]);
    let head = git(root, &["rev-parse", "HEAD"]);
    let mut r = fx.receipt.clone();
    let texts = [
        ("claims.json", &fx.claims, "claims-blind"),
        ("plan.json", &fx.plan, "canary-plan"),
        ("verdicts.json", &fx.verdicts, "judge-verdicts"),
    ];
    let mut files = Vec::new();
    for (name, v, role) in texts {
        let body = serde_json::to_string_pretty(v).expect("json");
        put(root, &format!(".quorum/evidence/{name}"), &body);
        files.push(json!({"path": format!(".quorum/evidence/{name}"), "roles": [role], "sha256": sha256(&body)}));
    }
    if !skip_hash {
        r["evidence"]["files"] = Value::Array(files);
    }
    if let Some(c) = r["quorum"].get_mut("canary").and_then(Value::as_object_mut) {
        c.entry("reviewed_base").or_insert(json!(base));
        c.entry("reviewed_head").or_insert(json!(head));
    }
    put(root, "rec.json", &r.to_string());
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "evidence"]);
    let o = Command::new("bash")
        .args([
            "-c",
            r#". "$1"; r="$(cat rec.json)"; shift; dogfood_receipt_status "$r" "$@""#,
            "x",
        ])
        .arg(script)
        .args(revs)
        .current_dir(root)
        .output()
        .expect("bash");
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

pub type Edit = fn(&mut Fx);

pub fn set_claims(fx: &mut Fx, i: usize, k: &str, v: Value) {
    fx.claims[i][k] = v;
}

/// (name, edit, expected answer: "ok" exactly, or a substring of the refusal).
pub mod cases;
