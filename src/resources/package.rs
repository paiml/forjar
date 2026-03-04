//! FJ-006: Package resource handler (apt + cargo + uv + brew).
//! FJ-1398: Cross-platform resource abstraction via brew provider.

use crate::core::types::Resource;

/// Generate shell script to check if packages are installed.
pub fn check_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let packages = &resource.packages;

    match provider {
        "apt" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| {
                    format!(
                        "dpkg -l '{p}' >/dev/null 2>&1 && echo 'installed:{p}' || echo 'missing:{p}'"
                    )
                })
                .collect();
            checks.join("\n")
        }
        "cargo" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| format!("command -v '{p}' >/dev/null 2>&1 && echo 'installed:{p}' || echo 'missing:{p}'"))
                .collect();
            checks.join("\n")
        }
        "uv" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| format!("uv tool list 2>/dev/null | grep -q '^{p}' && echo 'installed:{p}' || echo 'missing:{p}'"))
                .collect();
            checks.join("\n")
        }
        "brew" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| format!("brew list '{p}' >/dev/null 2>&1 && echo 'installed:{p}' || echo 'missing:{p}'"))
                .collect();
            checks.join("\n")
        }
        other => format!("echo 'unsupported provider: {other}'"),
    }
}

/// Generate shell script to install packages.
pub fn apply_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let state = resource.state.as_deref().unwrap_or("present");

    match (provider, state) {
        ("apt", "present") => apply_apt_present(resource),
        ("apt", "absent") => apply_apt_absent(resource),
        ("cargo", "present") => apply_cargo_present(resource),
        ("uv", "present") => apply_uv_present(resource),
        ("uv", "absent") => apply_uv_absent(resource),
        ("brew", "present") => apply_brew_present(resource),
        ("brew", "absent") => apply_brew_absent(resource),
        (other_provider, other_state) => {
            format!("echo 'unsupported: provider={other_provider}, state={other_state}'")
        }
    }
}

fn apply_apt_present(resource: &Resource) -> String {
    let packages = &resource.packages;
    let version = resource.version.as_deref();
    let pkg_list: Vec<String> = packages
        .iter()
        .map(|p| match version {
            Some(v) => format!("'{p}={v}'"),
            None => format!("'{p}'"),
        })
        .collect();
    let check_list: Vec<String> = packages.iter().map(|p| format!("'{p}'")).collect();
    let joined = pkg_list.join(" ");
    let check_joined = check_list.join(" ");
    format!(
        "set -euo pipefail\n\
         NEED_INSTALL=0\n\
         for pkg in {check_joined}; do\n\
           dpkg -l \"$pkg\" >/dev/null 2>&1 || NEED_INSTALL=1\n\
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
           dpkg -l \"$pkg\" >/dev/null 2>&1\n\
         done"
    )
}

fn apply_apt_absent(resource: &Resource) -> String {
    let packages = &resource.packages;
    let pkg_list: Vec<String> = packages.iter().map(|p| format!("'{p}'")).collect();
    let joined = pkg_list.join(" ");
    format!(
        "set -euo pipefail\n\
         NEED_REMOVE=0\n\
         for pkg in {joined}; do\n\
           dpkg -l \"$pkg\" >/dev/null 2>&1 && NEED_REMOVE=1\n\
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

fn apply_cargo_present(resource: &Resource) -> String {
    let packages = &resource.packages;
    let version = resource.version.as_deref();
    let source = resource.source.as_deref();
    let installs: Vec<String> = packages
        .iter()
        .map(|p| match (source, version) {
            (Some(s), _) => format!("cargo install --force --path '{s}'"),
            (None, Some(v)) => format!("cargo install --force '{p}@{v}'"),
            (None, None) => format!("cargo install --force '{p}'"),
        })
        .collect();
    // Limit build parallelism to avoid OOM on high-core-count machines.
    // Respects CARGO_BUILD_JOBS if already set; defaults to min(nproc/2, 8).
    format!(
        "set -euo pipefail\n\
         command -v cargo >/dev/null 2>&1 || {{\n\
           RUSTUP_INIT=$(mktemp /tmp/rustup-init.XXXXXX)\n\
           curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o \"$RUSTUP_INIT\"\n\
           chmod +x \"$RUSTUP_INIT\"\n\
           \"$RUSTUP_INIT\" -y --no-modify-path\n\
           rm -f \"$RUSTUP_INIT\"\n\
           export PATH=\"$HOME/.cargo/bin:$PATH\"\n\
         }}\n\
         if [ -z \"${{CARGO_BUILD_JOBS:-}}\" ]; then\n\
           _nproc=$(nproc 2>/dev/null || echo 4)\n\
           _half=$(( _nproc / 2 ))\n\
           [ \"$_half\" -lt 1 ] && _half=1\n\
           [ \"$_half\" -gt 8 ] && _half=8\n\
           export CARGO_BUILD_JOBS=$_half\n\
         fi\n\
         {}",
        installs.join("\n")
    )
}

fn apply_uv_present(resource: &Resource) -> String {
    let packages = &resource.packages;
    let version = resource.version.as_deref();
    let installs: Vec<String> = packages
        .iter()
        .map(|p| match version {
            Some(v) => format!("uv tool install --force '{p}=={v}'"),
            None => format!("uv tool install --force '{p}'"),
        })
        .collect();
    format!("set -euo pipefail\n{}", installs.join("\n"))
}

fn apply_uv_absent(resource: &Resource) -> String {
    let packages = &resource.packages;
    let removals: Vec<String> = packages
        .iter()
        .map(|p| format!("uv tool uninstall '{p}' 2>/dev/null || true"))
        .collect();
    format!("set -euo pipefail\n{}", removals.join("\n"))
}

/// FJ-1398: Homebrew install (macOS/Linux cross-platform).
fn apply_brew_present(resource: &Resource) -> String {
    let packages = &resource.packages;
    let version = resource.version.as_deref();
    let check_list: Vec<String> = packages.iter().map(|p| format!("'{p}'")).collect();
    let installs: Vec<String> = packages
        .iter()
        .map(|p| match version {
            Some(v) => format!("brew install '{p}@{v}'"),
            None => format!("brew install '{p}'"),
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
    let packages = &resource.packages;
    let removals: Vec<String> = packages
        .iter()
        .map(|p| format!("brew uninstall '{p}' 2>/dev/null || true"))
        .collect();
    format!("set -euo pipefail\n{}", removals.join("\n"))
}

/// Generate shell to query installed versions (for state hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let packages = &resource.packages;

    match provider {
        "apt" => {
            let queries: Vec<String> = packages
                .iter()
                .map(|p| format!("dpkg-query -W -f '${{Package}}=${{Version}}\\n' '{p}' 2>/dev/null || echo '{p}=MISSING'"))
                .collect();
            queries.join("\n")
        }
        "cargo" => {
            let queries: Vec<String> = packages
                .iter()
                .map(|p| format!("command -v '{p}' >/dev/null 2>&1 && echo '{p}=installed' || echo '{p}=MISSING'"))
                .collect();
            queries.join("\n")
        }
        "uv" => {
            let queries: Vec<String> = packages
                .iter()
                .map(|p| format!("uv tool list 2>/dev/null | grep -q '^{p}' && echo '{p}=installed' || echo '{p}=MISSING'"))
                .collect();
            queries.join("\n")
        }
        "brew" => {
            let queries: Vec<String> = packages
                .iter()
                .map(|p| format!("brew list --versions '{p}' 2>/dev/null || echo '{p}=MISSING'"))
                .collect();
            queries.join("\n")
        }
        other => format!("echo 'unsupported provider: {other}'"),
    }
}
