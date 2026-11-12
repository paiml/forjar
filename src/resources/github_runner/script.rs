//! forjar#1000: the script the seat runs to install and register a runner.
//!
//! Nothing here is secret except the registration token, and the token has one
//! way in: a here-document feeding a `read` in the shell that runs `config.sh`
//! (`ACTIONS_RUNNER_INPUT_TOKEN`, the runner's documented input variable). It is
//! never an argument, never a shell variable of the outer script, and the
//! transcript form of the script carries `<redacted>` in its place. The
//! transport's data-payload stripping hides the here-document from lint output.
//!
//! The tarball is verified against the manifest's sha256 BEFORE it is
//! extracted, and `config.sh` is never given `--replace`.

use crate::core::shell_escape::sh_squote;

/// Placeholder the transcript form carries where the token goes.
pub(super) const REDACTED: &str = "<redacted>";

/// Everything the script needs, all of it non-secret.
pub(super) struct SeatPlan {
    pub tarball_url: String,
    pub sha256: String,
    pub work_dir: String,
    pub install_dir: String,
    pub user: String,
    pub registration_url: String,
    pub name: String,
    pub labels: String,
    pub group: Option<String>,
}

/// The tarball URL: `<base>/v<version>/actions-runner-linux-<arch>-<version>.tar.gz`,
/// with the base overridable (`FORJAR_GITHUB_RUNNER_DOWNLOAD_BASE`) for tests.
pub(super) fn tarball_url(version: &str, arm64: bool) -> String {
    let base = std::env::var("FORJAR_GITHUB_RUNNER_DOWNLOAD_BASE")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "https://github.com/actions/runner/releases/download".to_string());
    let arch = if arm64 { "arm64" } else { "x64" };
    format!(
        "{}/v{version}/actions-runner-linux-{arch}-{version}.tar.gz",
        base.trim_end_matches('/')
    )
}

fn config_args(plan: &SeatPlan) -> String {
    let mut args = vec![
        "--unattended".to_string(),
        "--url".into(),
        sh_squote(&plan.registration_url),
        "--name".into(),
        sh_squote(&plan.name),
        "--work".into(),
        sh_squote(&format!("{}/_work", plan.work_dir)),
    ];
    if !plan.labels.is_empty() {
        args.extend(["--labels".to_string(), sh_squote(&plan.labels)]);
    }
    if let Some(g) = &plan.group {
        args.extend(["--runnergroup".to_string(), sh_squote(g)]);
    }
    args.join(" ")
}

/// The full script with `token` on its own line inside the here-document.
pub(super) fn seat_script(plan: &SeatPlan, token: &str) -> String {
    let (work, dir, user) = (
        sh_squote(&plan.work_dir),
        sh_squote(&plan.install_dir),
        sh_squote(&plan.user),
    );
    format!(
        r#"set -euo pipefail
WORK={work}
RUN={dir}
RUNNER_USER={user}
mkdir -p "$WORK" "$RUN"
as_root() {{
  if [ "$(id -u)" = 0 ]; then "$@"; elif sudo -n true 2>/dev/null; then sudo -n "$@"; else "$@"; fi
}}
as_user() {{
  if [ "$(id -un)" = "$RUNNER_USER" ]; then "$@"; else sudo -n -u "$RUNNER_USER" -H "$@"; fi
}}
if [ "$(id -u)" = 0 ]; then chown "${{RUNNER_USER:?}}" "${{WORK:?}}" "${{RUN:?}}"; fi
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
curl -fsSL -o "$TMP/runner.tar.gz" {url}
if ! echo {sha}"  $TMP/runner.tar.gz" | sha256sum -c --status; then
  echo "github_runner: sha256 mismatch for the runner tarball (expected {sha_plain}); refusing to extract or run it" >&2
  exit 1
fi
tar -xzf "$TMP/runner.tar.gz" -C "$RUN"
if [ "$(id -u)" = 0 ]; then chown -R "${{RUNNER_USER:?}}" "${{RUN:?}}"; fi
cat <<'FORJAR_EOF' | as_user bash -c 'IFS= read -r ACTIONS_RUNNER_INPUT_TOKEN; export ACTIONS_RUNNER_INPUT_TOKEN; cd "$1"; shift; exec ./config.sh "$@"' _ "$RUN" {args}
{token}
FORJAR_EOF
cd "$RUN"
as_root ./svc.sh install "$RUNNER_USER"
as_root ./svc.sh start
echo "github_runner: registered {name_plain} and started its service"
"#,
        url = sh_squote(&plan.tarball_url),
        sha = sh_squote(&plan.sha256),
        sha_plain = plan.sha256.replace('\'', ""),
        args = config_args(plan),
        name_plain = plan.name.replace(['\'', '"', '`', '$', '\n'], ""),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> SeatPlan {
        SeatPlan {
            tarball_url: "https://example.org/a.tar.gz".into(),
            sha256: "ab".repeat(32),
            work_dir: "/opt/runner/gate-1".into(),
            install_dir: "/opt/runner/gate-1/runner".into(),
            user: "runner".into(),
            registration_url: "https://github.com/example-org".into(),
            name: "host-gate-1".into(),
            labels: "gate,fast".into(),
            group: Some("default".into()),
        }
    }

    #[test]
    fn the_token_is_only_in_the_here_document() {
        let s = seat_script(&plan(), "TOKENMARKER");
        assert_eq!(s.matches("TOKENMARKER").count(), 1);
        assert!(s.lines().any(|l| l == "TOKENMARKER"));
        let transcript = seat_script(&plan(), REDACTED);
        assert!(!transcript.contains("TOKENMARKER"));
    }

    #[test]
    fn never_replace_and_digest_is_checked_before_extraction() {
        let s = seat_script(&plan(), "t");
        assert!(!s.contains("--replace"));
        assert!(s.find("sha256sum -c").unwrap() < s.find("tar -xzf").unwrap());
        assert!(s.find("tar -xzf").unwrap() < s.find("config.sh").unwrap());
        assert!(s.contains("--unattended") && s.contains("--runnergroup 'default'"));
    }

    #[test]
    fn tarball_url_follows_the_release_layout() {
        let u = tarball_url("2.330.0", false);
        assert!(u.ends_with("/v2.330.0/actions-runner-linux-x64-2.330.0.tar.gz"));
        assert!(tarball_url("2.330.0", true).contains("linux-arm64-2.330.0"));
    }
}
