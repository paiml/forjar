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
    /// Split the code change into two commits (`c1 -> c2`) off `base`.
    pub multi_commit: bool,
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
    /// The census MUTANT: a reachable `reviewed_head` is passed off as pending
    /// instead of being judged by the bound path.
    pub census_pending_reachable: bool,
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

fn write_executable(dst: &Path, body: &str) {
    std::fs::write(dst, body).expect("shim");
    let mut p = std::fs::metadata(dst).expect("meta").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut p, 0o755);
    std::fs::set_permissions(dst, p).expect("chmod");
}

fn write_shim_tool(dst: &Path, tool: &str, mode: &str) {
    if mode == "silent0" || mode == "silent1" {
        let code = if mode == "silent0" { 0 } else { 1 };
        write_executable(dst, &format!("#!/bin/sh\nexit {code}\n"));
        return;
    }
    // when:<case pattern on "$*">: silent for that call, the real tool otherwise.
    // rej:<pattern>: exit 2 (an option this tool does not know) instead.
    let Some((pat, code)) = mode
        .strip_prefix("rej:")
        .map(|p| (p, 2))
        .or_else(|| mode.strip_prefix("when:").map(|p| (p, 0)))
    else {
        return;
    };
    let real = ["/usr/bin", "/bin"]
        .iter()
        .map(|d| Path::new(d).join(tool))
        .find(|p| p.exists())
        .expect("real tool");
    write_executable(
        dst,
        &format!(
            "#!/bin/sh\ncase \"$*\" in {pat}) exit {code};; esac\nexec {} \"$@\"\n",
            real.display()
        ),
    );
}

/// A directory to use as the whole PATH: a symlink to each of TOOLS except the
/// shimmed one, which is missing or a silent script.
fn shim_path(dir: &Path, shim: (&str, &str)) -> String {
    let bin = dir.join("shimbin");
    std::fs::create_dir_all(&bin).expect("shim dir");
    for t in TOOLS {
        let dst = bin.join(t);
        if t == shim.0 {
            write_shim_tool(&dst, t, shim.1);
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

pub use census::*;

pub mod cases;
pub mod cases_review;
pub mod cases_round3;
pub mod cases_round3b;
pub mod cases_round6;
pub mod cases_round7;
mod census;
mod census_jq;
pub use census_jq::*;
pub mod mutants;
