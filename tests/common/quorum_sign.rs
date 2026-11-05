//! Shared fixtures for the quorum gate tests (PMAT-5882, R34-3): throwaway ssh keys, a gate
//! staged beside a TEMP allowed-signers file (exactly how CI stages the PR-base copy), and
//! signing/delivery of receipts. No production key exists here; every key is generated per
//! test process in a temp dir.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

pub struct Key {
    pub private: PathBuf,
    pub signers_line: String,
    _dir: tempfile::TempDir,
}

pub fn new_key() -> Key {
    let dir = tempfile::tempdir().unwrap();
    let private = dir.path().join("k");
    let o = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", "test", "-f"])
        .arg(&private)
        .output()
        .unwrap();
    assert!(o.status.success(), "ssh-keygen: {o:?}");
    let publ = std::fs::read_to_string(dir.path().join("k.pub")).unwrap();
    let mut it = publ.split_whitespace();
    let (ty, b64) = (it.next().unwrap(), it.next().unwrap());
    Key {
        private,
        signers_line: format!("quorum-writer namespaces=\"quorum-receipt\" {ty} {b64}\n"),
        _dir: dir,
    }
}

/// The key every ordinary fixture is signed with (and the staged signers file trusts).
pub fn trusted() -> &'static Key {
    static K: OnceLock<Key> = OnceLock::new();
    K.get_or_init(new_key)
}

/// Canonical form the gate verifies: `jq -S -c .`
pub fn canonical(body: &[u8]) -> Vec<u8> {
    pipe("jq", &["-S", "-c", "."], body)
}

fn pipe(cmd: &str, args: &[&str], input: &[u8]) -> Vec<u8> {
    let mut c = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(input).unwrap();
    let o = c.wait_with_output().unwrap();
    o.stdout
}

/// Detached signature over the canonical form of `body`, made with `key`.
pub fn sign_with(key: &Key, body: &[u8]) -> Vec<u8> {
    let canon = canonical(body);
    let sig = pipe(
        "ssh-keygen",
        &[
            "-Y",
            "sign",
            "-n",
            "quorum-receipt",
            "-f",
            key.private.to_str().unwrap(),
        ],
        &canon,
    );
    assert!(!sig.is_empty(), "signing produced nothing (body not JSON?)");
    sig
}

pub fn sign(body: &[u8]) -> Vec<u8> {
    sign_with(trusted(), body)
}

/// Copy the real gate and allowlist into `dir/scripts`, with an allowed-signers file that
/// trusts `signer` (or the placeholder when None). Returns the staged gate path.
pub fn stage_gate(dir: &Path, signer: Option<&Key>) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = dir.join("scripts");
    std::fs::create_dir_all(&s).unwrap();
    for f in ["quorum-store-gate.sh", "quorum-models.json"] {
        std::fs::copy(root.join("scripts").join(f), s.join(f)).unwrap();
    }
    let signers = match signer {
        Some(k) => k.signers_line.clone(),
        None => std::fs::read_to_string(root.join("scripts/quorum-allowed-signers")).unwrap(),
    };
    std::fs::write(s.join("quorum-allowed-signers"), signers).unwrap();
    s.join("quorum-store-gate.sh")
}

/// Gate staged beside the trusted throwaway signer, in a fresh leaked temp dir.
pub fn trusted_gate() -> PathBuf {
    static G: OnceLock<PathBuf> = OnceLock::new();
    G.get_or_init(|| {
        let d = tempfile::tempdir().unwrap().keep();
        stage_gate(&d, Some(trusted()))
    })
    .clone()
}

/// Write `body` + signature into `dir` as `<dir>/<sha>.json[.sig]`.
pub fn write_signed(dir: &Path, sha: &str, body: &[u8]) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(format!("{sha}.json")), body).unwrap();
    std::fs::write(dir.join(format!("{sha}.json.sig")), sign(body)).unwrap();
}

/// Publish `<pr>/<sha>.json[.sig]` as refs/quorum/pr-<pr> on a bare repo, and make it the
/// `origin` of `work`. This is the production delivery path (git fetch of a ref).
pub fn publish_ref(work: &Path, pr: u32, sha: &str, body: &[u8], sig: &[u8]) {
    let git = |dir: &Path, args: &[&str]| {
        let o = Command::new("git")
            .args([
                "-c",
                "user.name=w",
                "-c",
                "user.email=w@w",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "{args:?} {}",
            String::from_utf8_lossy(&o.stderr)
        );
    };
    let bare = work.join("..").join(format!(
        "receipts-{}.git",
        work.file_name().unwrap().to_string_lossy()
    ));
    let stage = tempfile::tempdir().unwrap();
    if !bare.exists() {
        std::fs::create_dir_all(&bare).unwrap();
        git(&bare, &["init", "-q", "--bare"]);
    }
    git(stage.path(), &["init", "-q", "-b", "w"]);
    let d = stage.path().join(pr.to_string());
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(format!("{sha}.json")), body).unwrap();
    std::fs::write(d.join(format!("{sha}.json.sig")), sig).unwrap();
    git(stage.path(), &["add", "-A"]);
    git(stage.path(), &["commit", "-q", "-m", "receipt"]);
    git(
        stage.path(),
        &[
            "push",
            "-q",
            "-f",
            bare.to_str().unwrap(),
            &format!("HEAD:refs/quorum/pr-{pr}"),
        ],
    );
    let _ = Command::new("git")
        .args(["remote", "remove", "origin"])
        .current_dir(work)
        .output();
    git(work, &["remote", "add", "origin", bare.to_str().unwrap()]);
}
