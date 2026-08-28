//! A published release object must be a description of ITSELF.
//!
//! forjar#325. v1.18.0 shipped with four `forjar-1.17.0-*.tar.gz` assets attached
//! to it and a SHA256SUMS carrying 10 lines for 6 archives — four of them naming
//! a version that release is not. The macOS archives had no `.sha256` sidecar at
//! all while every Linux archive did.
//!
//! #324 fixed one of the two producers. Nothing ever read the finished object
//! BACK, so the same defect reached v1.19.0, v1.20.0 and v1.20.1 unnoticed. On
//! v1.20.1 — the release `install.sh` resolves by default — the two checksum
//! surfaces disagreed outright:
//!
//!   sidecar  forjar-1.20.1-x86_64-unknown-linux-gnu.tar.gz.sha256
//!            e17903a9e87e8e562ec0055e51d28e76e3c8d707ced959b89a0789a308aee775
//!   SHA256SUMS line for the same asset
//!            ec28f843c37a27c645f169524313f7577fbf4fd73f5105f3f3fec1d4b763bf81
//!   the actual bytes
//!            ec28f843c37a27c645f169524313f7577fbf4fd73f5105f3f3fec1d4b763bf81
//!
//! The published sidecar named a build that release.yml had already clobbered.
//! `install.sh` falls back to the sidecar when SHA256SUMS does not name the asset
//! and then refuses, so a wrong sidecar is one contaminated SHA256SUMS away from
//! a hard, wrong-digest install failure.
//!
//! The fixtures below are the RECORDED pre-repair state, so this suite is offline
//! and deterministic: the env overrides make the auditor read files instead of
//! calling `gh`, and no test here touches the network.

use std::process::Command;

fn script() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/release-object-audit.sh")
}

const TARGETS: &[&str] = &[
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
];

/// The four Linux targets binary-release.yml built before darwin was added.
const LINUX_TARGETS: &[&str] = &[
    "aarch64-unknown-linux-gnu",
    "aarch64-unknown-linux-musl",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
];

/// A deterministic stand-in digest. Only the two digests the disagreement test
/// turns on are the real recorded values.
fn digest_for(name: &str) -> String {
    let mut out = String::new();
    let mut acc: u8 = 7;
    for b in name.bytes() {
        acc = acc.wrapping_mul(31).wrapping_add(b);
        if out.len() < 64 {
            out.push_str(&format!("{acc:02x}"));
        }
    }
    while out.len() < 64 {
        out.push('0');
    }
    out.truncate(64);
    out
}

fn tarballs(version: &str, targets: &[&str]) -> Vec<String> {
    targets
        .iter()
        .map(|t| format!("forjar-{version}-{t}.tar.gz"))
        .collect()
}

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("sidecars")).expect("sidecars dir");
        Fixture { dir }
    }

    fn assets(&self, names: &[String]) -> &Self {
        std::fs::write(self.dir.path().join("assets.txt"), names.join("\n") + "\n")
            .expect("assets.txt");
        self
    }

    fn sums(&self, lines: &[(String, String)]) -> &Self {
        let body: String = lines
            .iter()
            .map(|(d, n)| format!("{d}  {n}\n"))
            .collect::<Vec<_>>()
            .join("");
        std::fs::write(self.dir.path().join("SHA256SUMS"), body).expect("SHA256SUMS");
        self
    }

    fn sidecar(&self, name: &str, digest: &str) -> &Self {
        std::fs::write(
            self.dir
                .path()
                .join("sidecars")
                .join(format!("{name}.sha256")),
            format!("{digest}  {name}\n"),
        )
        .expect("sidecar");
        self
    }

    fn check(&self, tag: &str) -> (bool, String) {
        let p = self.dir.path();
        let out = Command::new("bash")
            .arg(script())
            .arg("check")
            .arg(tag)
            .env("FORJAR_AUDIT_ASSETS_FILE", p.join("assets.txt"))
            .env("FORJAR_AUDIT_SUMS_FILE", p.join("SHA256SUMS"))
            .env("FORJAR_AUDIT_SIDECAR_DIR", p.join("sidecars"))
            .output()
            .expect("the auditor script must be runnable with bash");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.success(), text)
    }
}

/// The recorded v1.20.1 object: six own archives, 22 strays inherited from four
/// earlier releases, a SHA256SUMS describing all 28, sidecars for the four Linux
/// archives only, and one of those sidecars naming the wrong bytes.
fn contaminated_v1_20_1() -> Fixture {
    let own = tarballs("1.20.1", TARGETS);
    let mut strays: Vec<String> = Vec::new();
    for v in ["1.17.0", "1.18.0", "1.19.0", "1.20.0"] {
        let targets = if v == "1.17.0" {
            LINUX_TARGETS
        } else {
            TARGETS
        };
        strays.extend(tarballs(v, targets));
    }
    assert_eq!(strays.len(), 22, "the recorded stray count");

    let mut assets: Vec<String> = strays.clone();
    assets.extend(own.clone());
    for t in LINUX_TARGETS {
        assets.push(format!("forjar-1.20.1-{t}.tar.gz.sha256"));
    }
    assets.push("SHA256SUMS".to_string());

    let sums: Vec<(String, String)> = own
        .iter()
        .chain(strays.iter())
        .map(|n| (digest_for(n), n.clone()))
        .collect();
    assert_eq!(sums.len(), 28, "the recorded SHA256SUMS line count");

    let f = Fixture::new();
    f.assets(&assets).sums(&sums);
    for t in LINUX_TARGETS {
        let name = format!("forjar-1.20.1-{t}.tar.gz");
        f.sidecar(&name, &digest_for(&name));
    }
    // The one real disagreement, verbatim from the published object.
    f.sidecar(
        "forjar-1.20.1-x86_64-unknown-linux-gnu.tar.gz",
        "e17903a9e87e8e562ec0055e51d28e76e3c8d707ced959b89a0789a308aee775",
    );
    f
}

/// The repaired v1.18.0 object: six archives, six agreeing sidecars, a six-line
/// SHA256SUMS, nothing else.
fn clean_v1_18_0() -> Fixture {
    let own = tarballs("1.18.0", TARGETS);
    let mut assets = own.clone();
    assets.extend(own.iter().map(|n| format!("{n}.sha256")));
    assets.push("SHA256SUMS".to_string());
    let sums: Vec<(String, String)> = own.iter().map(|n| (digest_for(n), n.clone())).collect();

    let f = Fixture::new();
    f.assets(&assets).sums(&sums);
    for n in &own {
        f.sidecar(n, &digest_for(n));
    }
    f
}

#[test]
fn contaminated_release_object_is_rejected() {
    let (ok, out) = contaminated_v1_20_1().check("v1.20.1");
    assert!(
        !ok,
        "the auditor accepted a release carrying 22 assets from four other \
         versions (#325). Output:\n{out}"
    );
    for expected in [
        "stray: forjar-1.17.0-x86_64-unknown-linux-gnu.tar.gz",
        "SHA256SUMS describes 28 archives, release has 6",
        "sums-extra: forjar-1.20.0-x86_64-apple-darwin.tar.gz",
        "sidecar-missing: forjar-1.20.1-x86_64-apple-darwin.tar.gz.sha256",
        "sidecar-disagrees: forjar-1.20.1-x86_64-unknown-linux-gnu.tar.gz",
    ] {
        assert!(
            out.contains(expected),
            "the auditor never reported {expected:?}. Output:\n{out}"
        );
    }
}

#[test]
fn every_violation_is_reported_not_just_the_first() {
    // A report that stops at the first stray makes a release carrying four
    // versions look like it has one problem, which is how #325 was closed after
    // repairing exactly one of four affected tags.
    let (_, out) = contaminated_v1_20_1().check("v1.20.1");
    let strays = out.lines().filter(|l| l.starts_with("stray: ")).count();
    assert_eq!(
        strays, 22,
        "expected all 22 strays named, got {strays}. Output:\n{out}"
    );
}

#[test]
fn clean_release_object_passes() {
    let (ok, out) = clean_v1_18_0().check("v1.18.0");
    assert!(ok, "the auditor rejected a clean release. Output:\n{out}");
    let violations: Vec<&str> = out
        .lines()
        .filter(|l| {
            l.starts_with("stray:")
                || l.starts_with("sums-")
                || l.starts_with("sidecar-")
                || l.starts_with("no-archives:")
        })
        .collect();
    assert!(
        violations.is_empty(),
        "clean release reported violations {violations:?}. Output:\n{out}"
    );
}

#[test]
fn an_empty_release_object_does_not_pass_vacuously() {
    // The denominator assertion. Without it a release that lost every asset
    // sails through "no strays" and "SHA256SUMS names nothing extra".
    let f = Fixture::new();
    f.assets(&["SHA256SUMS".to_string()]).sums(&[]);
    let (ok, out) = f.check("v9.9.9");
    assert!(!ok, "an assetless release passed the audit. Output:\n{out}");
    assert!(
        out.contains("no-archives:"),
        "the auditor did not say the release carries no archives. Output:\n{out}"
    );
}
