//! Test harness for `tests/falsification_publish_from_tag.rs`: the sandbox
//! repo, the shim `cargo`, and the shim-log readers. Extracted from the suite
//! itself only to stay under this repo's 500-line file health gate — it is
//! not a separate test target (cargo compiles only top-level `tests/*.rs`).
//!
//! THE SHIM IS THE OBSERVER. `scripts/publish-from-tag.sh` must never reach
//! crates.io from a test, and the properties under test are negatives — no
//! registry token at the publish step, no scratch file in the worktree cargo
//! packages, no early publish of a dependent. A shim `cargo` placed first on
//! PATH is the only vantage point from which those can be attested: it logs
//! argv, cwd, `CARGO_REGISTRY_TOKEN`, `git rev-parse --git-common-dir`, the
//! live worktree count, the `git status --porcelain` line count and a full
//! `ls -a` of its cwd, once per invocation, for the suite to read back.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/publish-from-tag.sh")
}

/// A single workspace crate `demo 0.0.1`, no dependencies.
pub(crate) const META_DEMO: &str = r#"{"packages":[{"name":"demo","version":"0.0.1","publish":null,"manifest_path":"/x/Cargo.toml","dependencies":[]}]}"#;

/// `beta` normally depends on `alpha`, so `alpha` publishes first and the
/// index poll for `alpha` gates `beta`.
pub(crate) const META_CHAIN: &str = r#"{"packages":[
{"name":"alpha","version":"0.1.0","publish":null,"manifest_path":"/x/alpha/Cargo.toml","dependencies":[]},
{"name":"beta","version":"0.1.0","publish":null,"manifest_path":"/x/beta/Cargo.toml","dependencies":[{"name":"alpha","kind":null,"path":"/x/alpha"}]}]}"#;

/// PMAT-186: `a_app` normally depends on `b_core`; `b_core` DEV-depends back
/// on `a_app`. Counting the dev edge makes this a cycle; ignoring it (as
/// cargo's own publish order does) sorts `b_core` then `a_app`.
pub(crate) const META_DEV_BACK_EDGE: &str = r#"{"packages":[
{"name":"a_app","version":"0.1.0","publish":null,"manifest_path":"/x/a/Cargo.toml","dependencies":[{"name":"b_core","kind":null,"path":"/x/b"}]},
{"name":"b_core","version":"0.1.0","publish":null,"manifest_path":"/x/b/Cargo.toml","dependencies":[{"name":"a_app","kind":"dev","path":"/x/a"}]}]}"#;

/// Answers `metadata`, `search`, `info` and `publish` from canned data, and
/// logs seven fields per invocation. `CARGO_SHIM_INFO_MODE` drives `info`:
/// `never` (nothing is on the index), `after:N` (the first N calls FOR THAT
/// SPEC fail, the rest succeed), `absent` (`cargo info` does not exist on
/// this cargo, so `--help` fails too and the script must fall back to
/// `cargo search`).
const CARGO_SHIM: &str = r##"#!/usr/bin/env bash
set -eu
LOG="${CARGO_SHIM_LOG:?}"
STATE="$(dirname "$LOG")"
gcd="$(git rev-parse --git-common-dir 2>/dev/null || printf '')"
if [ -n "$gcd" ] && [ -d "$gcd" ]; then
  gcd="$(cd "$gcd" && pwd -P)"
else
  gcd='<none>'
fi
{
  printf 'ARGV: %s\n' "$*"
  printf 'CWD: %s\n' "$PWD"
  printf 'TOKEN: %s\n' "${CARGO_REGISTRY_TOKEN:-<unset>}"
  printf 'GITCOMMONDIR: %s\n' "$gcd"
  printf 'WORKTREES: %s\n' "$(git worktree list --porcelain 2>/dev/null | grep -c '^worktree' || true)"
  printf 'STATUS: %s\n' "$(git status --porcelain 2>/dev/null | wc -l)"
  printf 'LSCWD: %s\n' "$(ls -a | tr '\n' ' ')"
} >> "$LOG"
case "$1" in
  metadata)
    cat "${CARGO_SHIM_METADATA:?}"
    ;;
  search)
    printf '%s\n' "${CARGO_SHIM_SEARCH:?}"
    ;;
  info)
    mode="${CARGO_SHIM_INFO_MODE:-never}"
    if [ "$mode" = absent ]; then
      printf 'error: no such command: `info`\n' >&2
      exit 101
    fi
    spec="${2:-}"
    if [ "$spec" = "--help" ]; then
      exit 0
    fi
    key="$STATE/n-$(printf '%s' "$spec" | tr -c 'A-Za-z0-9' '_')"
    n=$(( $(cat "$key" 2>/dev/null || printf 0) + 1 ))
    printf '%s' "$n" > "$key"
    case "$mode" in
      never)
        exit 1
        ;;
      after:*)
        if [ "$n" -le "${mode#after:}" ]; then
          exit 1
        fi
        ;;
      *)
        ;;
    esac
    ;;
  publish)
    :
    ;;
esac
exit 0
"##;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

pub(crate) struct Sandbox {
    repo: tempfile::TempDir,
    bin: tempfile::TempDir,
    tmp: tempfile::TempDir,
}

impl Sandbox {
    pub(crate) fn new() -> Self {
        let repo = tempfile::tempdir().expect("repo tempdir");
        let bin = tempfile::tempdir().expect("bin tempdir");
        let tmp = tempfile::tempdir().expect("script tempdir");
        let r = repo.path();

        git(r, &["init", "-q", "-b", "main", "--template=", "."]);
        git(r, &["config", "user.email", "test@example.com"]);
        git(r, &["config", "user.name", "test"]);

        fs::write(
            r.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.0.1\"\nedition = \"2021\"\n\n\
             [workspace]\nmembers = [\".\"]\n",
        )
        .expect("write Cargo.toml");
        fs::create_dir_all(r.join("src")).expect("create src dir");
        fs::write(r.join("src/main.rs"), "fn main() {}\n").expect("write main.rs");

        git(r, &["add", "-A"]);
        git(r, &["commit", "-q", "-m", "init"]);
        git(r, &["tag", "v0.0.1"]);
        // Points at itself: `git fetch origin main` inside the script then
        // has a real `origin/main` to check ancestry against.
        git(r, &["remote", "add", "origin", &r.display().to_string()]);

        let cargo_path = bin.path().join("cargo");
        fs::write(&cargo_path, CARGO_SHIM).expect("write cargo shim");
        let mut perms = fs::metadata(&cargo_path).expect("stat shim").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&cargo_path, perms).expect("chmod shim");

        Sandbox { repo, bin, tmp }
    }

    pub(crate) fn path(&self) -> &Path {
        self.repo.path()
    }

    /// Tag the current HEAD under a name whose version does not match
    /// Cargo.toml (which still says 0.0.1).
    pub(crate) fn tag_mismatched(&self, tag: &str) {
        git(self.path(), &["tag", tag]);
    }

    pub(crate) fn run(&self, tag: &str) -> RunSpec<'_> {
        RunSpec {
            sb: self,
            tag: tag.to_string(),
            dry_run: false,
            metadata: META_DEMO,
            search: "demo = \"0.0.0\"    # a demo crate",
            info_mode: "never",
            delays: "0 0 0",
        }
    }
}

/// One invocation of the script under a named log, with the shim's canned
/// answers chosen per test.
pub(crate) struct RunSpec<'a> {
    sb: &'a Sandbox,
    tag: String,
    dry_run: bool,
    metadata: &'a str,
    search: &'a str,
    info_mode: &'a str,
    delays: &'a str,
}

impl<'a> RunSpec<'a> {
    pub(crate) fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    pub(crate) fn metadata(mut self, json: &'a str) -> Self {
        self.metadata = json;
        self
    }

    pub(crate) fn search(mut self, line: &'a str) -> Self {
        self.search = line;
        self
    }

    pub(crate) fn info_mode(mut self, mode: &'a str) -> Self {
        self.info_mode = mode;
        self
    }

    /// Runs the script and returns its output plus the shim log text.
    pub(crate) fn go(self, name: &str) -> (Output, String) {
        let log = self.sb.bin.path().join(format!("log-{name}.txt"));
        let meta = self.sb.bin.path().join(format!("meta-{name}.json"));
        fs::write(&log, "").expect("truncate shim log");
        fs::write(&meta, self.metadata).expect("write canned metadata");
        let path_var = format!(
            "{}:{}",
            self.sb.bin.path().display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut cmd = Command::new("bash");
        cmd.arg(script())
            .arg(&self.tag)
            .current_dir(self.sb.path())
            .env("PATH", path_var)
            .env("TMPDIR", self.sb.tmp.path())
            .env("CARGO_SHIM_LOG", &log)
            .env("CARGO_SHIM_METADATA", &meta)
            .env("CARGO_SHIM_SEARCH", self.search)
            .env("CARGO_SHIM_INFO_MODE", self.info_mode)
            .env("PUBLISH_POLL_DELAYS", self.delays)
            .env("CARGO_REGISTRY_TOKEN", "super-secret-test-token");
        if self.dry_run {
            cmd.env("DRY_RUN", "1");
        } else {
            cmd.env_remove("DRY_RUN");
        }
        let out = cmd.output().expect("run publish-from-tag.sh");
        let text = fs::read_to_string(&log).unwrap_or_default();
        (out, text)
    }
}

pub(crate) fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Every value the shim logged under `field` (e.g. `"CWD"`).
pub(crate) fn field(text: &str, name: &str) -> Vec<String> {
    let prefix = format!("{name}: ");
    text.lines()
        .filter_map(|l| l.strip_prefix(prefix.as_str()))
        .map(str::to_string)
        .collect()
}

pub(crate) fn worktree_count(repo: &Path) -> usize {
    let out = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(repo)
        .output()
        .expect("git worktree list");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count()
}
