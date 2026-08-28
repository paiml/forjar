//! FJ-006: Package resource handler (apt + cargo + uv + brew).
//! FJ-1398: Cross-platform resource abstraction via brew provider.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;

pub mod cargo;

use cargo::apply_cargo_present;

/// Generate shell script to install packages.
pub fn apply_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let state = resource.state.as_deref().unwrap_or("present");

    match (provider, state) {
        ("apt", "present") => apply_apt_present(resource),
        ("apt", "absent") => apply_apt_absent(resource),
        ("apt", "latest") => apply_apt_latest(resource),
        ("cargo", "present") => apply_cargo_present(resource),
        ("cargo", "absent") => cargo::apply_cargo_absent(resource),
        ("uv", "present") => apply_uv_present(resource),
        ("uv", "absent") => apply_uv_absent(resource),
        ("brew", "present") => apply_brew_present(resource),
        ("brew", "absent") => apply_brew_absent(resource),
        (other_provider, other_state) => {
            // AN UNSUPPORTED DECLARATION MUST NOT CONVERGE.
            //
            // This was `echo 'unsupported: ...'`, which exits 0, so forjar
            // reported the resource CONVERGED. An operator who declares
            // something forjar cannot do gets a success and no package action —
            // the declaration is silently ignored and the lock records it as
            // satisfied.
            //
            // `(cargo, absent)` was the live instance: apt, uv and brew all had
            // an absent arm and cargo did not, so a declared removal of a cargo
            // crate echoed and converged. (forjar#278.)
            //
            // Exit 1 with the pair named, so the failure says which combination
            // is missing rather than leaving the operator to diff the match.
            format!(
                "echo 'forjar: unsupported package declaration: \
                 provider={other_provider}, state={other_state}' >&2\nexit 1"
            )
        }
    }
}

fn apply_apt_present(resource: &Resource) -> String {
    let packages = &resource.packages;
    let version = resource.version.as_deref();
    let pkg_list: Vec<String> = packages
        .iter()
        .map(|p| match version {
            Some(v) => sh_squote(&format!("{p}={v}")),
            None => sh_squote(p),
        })
        .collect();
    let check_list: Vec<String> = packages.iter().map(|p| sh_squote(p)).collect();
    let joined = pkg_list.join(" ");
    let check_joined = check_list.join(" ");
    format!(
        "set -euo pipefail\n\
         NEED_INSTALL=0\n\
         for pkg in {check_joined}; do\n\
           dpkg -l \"$pkg\" 2>/dev/null | grep -q '^ii ' || NEED_INSTALL=1\n\
         done\n\
         if [ \"$NEED_INSTALL\" = \"1\" ]; then\n\
           if [ \"$(id -u)\" -ne 0 ]; then\n\
             sudo apt-get update -qq\n\
             DEBIAN_FRONTEND=noninteractive sudo apt-get install -y -qq {joined}\n\
           else\n\
             apt-get update -qq\n\
             DEBIAN_FRONTEND=noninteractive apt-get install -y -qq {joined}\n\
           fi\n\
         fi\n\
         # Postcondition: all packages installed\n\
         for pkg in {check_joined}; do\n\
           dpkg -l \"$pkg\" 2>/dev/null | grep -q '^ii '\n\
         done"
    )
}

// PMAT-161: state=latest semantics — refresh package lists then run
// `apt-get install`, which installs missing packages or upgrades to the
// newest available version (no-op if already current). Unlike `present`,
// this is not guarded by a `dpkg -l` presence check, since the goal is
// to converge on the latest available version regardless of what is
// currently installed.
//
// FJ-PMAT-161-1: `apt-get update` is tolerated as best-effort (`|| true`).
// In production, hosts routinely have one or two unreachable third-party
// PPAs, masked PackageKit units, or stale arm64 entries on x86_64 boxes
// that make `apt-get update` exit non-zero even when the repos we
// actually care about refreshed cleanly. Failing hard there blocks
// upgrades that should succeed. The subsequent `apt-get install` fails
// loud and clear if the requested package can't be resolved against the
// (possibly partially-stale) cache, so correctness of the postcondition
// is preserved. This matches canonical Dockerfile / Ansible practice.
fn apply_apt_latest(resource: &Resource) -> String {
    let packages = &resource.packages;
    let pkg_list: Vec<String> = packages.iter().map(|p| sh_squote(p)).collect();
    let joined = pkg_list.join(" ");
    let check_joined = pkg_list.join(" ");
    format!(
        "set -euo pipefail\n\
         if [ \"$(id -u)\" -ne 0 ]; then\n\
           sudo apt-get update -qq || true\n\
           DEBIAN_FRONTEND=noninteractive sudo apt-get install -y -qq {joined}\n\
         else\n\
           apt-get update -qq || true\n\
           DEBIAN_FRONTEND=noninteractive apt-get install -y -qq {joined}\n\
         fi\n\
         # Postcondition: all packages installed (at latest available)\n\
         for pkg in {check_joined}; do\n\
           dpkg -l \"$pkg\" 2>/dev/null | grep -q '^ii '\n\
         done"
    )
}

fn apply_apt_absent(resource: &Resource) -> String {
    let packages = &resource.packages;
    let pkg_list: Vec<String> = packages.iter().map(|p| sh_squote(p)).collect();
    let joined = pkg_list.join(" ");
    format!(
        "set -euo pipefail\n\
         NEED_REMOVE=0\n\
         for pkg in {joined}; do\n\
           dpkg -l \"$pkg\" 2>/dev/null | grep -q '^ii ' && NEED_REMOVE=1\n\
         done\n\
         if [ \"$NEED_REMOVE\" = \"1\" ]; then\n\
           if [ \"$(id -u)\" -ne 0 ]; then\n\
             DEBIAN_FRONTEND=noninteractive sudo apt-get remove -y -qq {joined}\n\
           else\n\
             DEBIAN_FRONTEND=noninteractive apt-get remove -y -qq {joined}\n\
           fi\n\
         fi"
    )
}

/// Parse a cargo package spec into (crate_name, features).
///
/// Supports `crate[feat1,feat2]` syntax for specifying cargo features inline.
/// Example: `"whisper-apr[cli]"` → `("whisper-apr", vec!["cli"])`
pub(crate) fn parse_cargo_features(pkg: &str) -> (&str, Vec<&str>) {
    if let Some(bracket_start) = pkg.find('[') {
        let crate_name = &pkg[..bracket_start];
        let rest = &pkg[bracket_start + 1..];
        let features_str = rest.trim_end_matches(']');
        let features: Vec<&str> = features_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        (crate_name, features)
    } else {
        (pkg, vec![])
    }
}

/// `set -euo pipefail` followed by one generated line per package.
///
/// The uv/brew/cargo install and removal arms are the same shape with a
/// different verb; writing that shape once keeps them from drifting apart.
pub(crate) fn per_package_script(packages: &[String], line: impl Fn(&str) -> String) -> String {
    let lines: Vec<String> = packages.iter().map(|p| line(p)).collect();
    format!("set -euo pipefail\n{}", lines.join("\n"))
}

/// One state-query line per package, newline-joined.
///
/// Deliberately no `set -e`: a query for one package that fails must not stop
/// the rest from reporting, or the state hash covers only the prefix.
pub(crate) fn per_package_query(packages: &[String], line: impl Fn(&str) -> String) -> String {
    packages
        .iter()
        .map(|p| line(p))
        .collect::<Vec<_>>()
        .join("\n")
}

fn apply_uv_present(resource: &Resource) -> String {
    let version = resource.version.as_deref();
    per_package_script(&resource.packages, |p| match version {
        Some(v) => format!(
            "uv tool install --force {}",
            sh_squote(&format!("{p}=={v}"))
        ),
        None => format!("uv tool install --force {}", sh_squote(p)),
    })
}

fn apply_uv_absent(resource: &Resource) -> String {
    per_package_script(&resource.packages, |p| {
        format!("uv tool uninstall {} 2>/dev/null || true", sh_squote(p))
    })
}

/// FJ-1398: Homebrew install (macOS/Linux cross-platform).
fn apply_brew_present(resource: &Resource) -> String {
    let packages = &resource.packages;
    let version = resource.version.as_deref();
    let check_list: Vec<String> = packages.iter().map(|p| sh_squote(p)).collect();
    let installs: Vec<String> = packages
        .iter()
        .map(|p| match version {
            Some(v) => format!("brew install {}", sh_squote(&format!("{p}@{v}"))),
            None => format!("brew install {}", sh_squote(p)),
        })
        .collect();
    let check_joined = check_list.join(" ");
    format!(
        "set -euo pipefail\n\
         NEED_INSTALL=0\n\
         for pkg in {check_joined}; do\n\
           brew list \"$pkg\" >/dev/null 2>&1 || NEED_INSTALL=1\n\
         done\n\
         if [ \"$NEED_INSTALL\" = \"1\" ]; then\n\
           {}\n\
         fi",
        installs.join("\n  ")
    )
}

fn apply_brew_absent(resource: &Resource) -> String {
    per_package_script(&resource.packages, |p| {
        format!("brew uninstall {} 2>/dev/null || true", sh_squote(p))
    })
}

/// Generate shell to query installed versions (for state hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let packages = &resource.packages;

    match provider {
        "apt" => per_package_query(packages, |p| {
            format!(
                "dpkg-query -W -f '${{Package}}=${{Version}}\\n' {} 2>/dev/null || echo {}",
                sh_squote(p),
                sh_squote(&format!("{p}=MISSING"))
            )
        }),
        "cargo" => cargo::state_query(packages),
        "uv" => per_package_query(packages, |p| {
            format!(
                "uv tool list 2>/dev/null | grep -q {} && echo {} || echo {}",
                sh_squote(&format!("^{p}")),
                sh_squote(&format!("{p}=installed")),
                sh_squote(&format!("{p}=MISSING"))
            )
        }),
        "brew" => per_package_query(packages, |p| {
            format!(
                "brew list --versions {} 2>/dev/null || echo {}",
                sh_squote(p),
                sh_squote(&format!("{p}=MISSING"))
            )
        }),
        other => format!("echo 'unsupported provider: {other}'"),
    }
}
