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
                .map(|p| {
                    let (crate_name, _) = parse_cargo_features(p);
                    format!("command -v '{crate_name}' >/dev/null 2>&1 && echo 'installed:{crate_name}' || echo 'missing:{crate_name}'")
                })
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

/// FJ-51: Cargo binary cache — skip recompilation when cached binary exists.
///
/// Cache layout: `$FORJAR_CACHE_DIR/<pkg>-<version>-<arch>/bin/`
/// Default cache dir: `~/.forjar/cache/cargo`
/// Disable: `FORJAR_NO_CARGO_CACHE=1`
///
/// Supports `crate[feat1,feat2]` syntax in package names to pass `--features`
/// to `cargo install`. Example: `packages: ["whisper-apr[cli]"]`.
fn apply_cargo_present(resource: &Resource) -> String {
    let packages = &resource.packages;
    let version = resource.version.as_deref();
    let source = resource.source.as_deref();
    let installs: Vec<String> = packages
        .iter()
        .map(|p| match (source, version) {
            // Local path installs — no caching, always rebuild
            (Some(s), _) => {
                let (_, features) = parse_cargo_features(p);
                let features_arg = if features.is_empty() {
                    String::new()
                } else {
                    format!(" --features '{}'", features.join(","))
                };
                format!("cargo install --force --locked --path '{s}'{features_arg}")
            }
            (None, ver) => cargo_cached_install(p, ver),
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
         _CARGO_BIN=\"${{CARGO_HOME:-$HOME/.cargo}}/bin\"\n\
         {}",
        installs.join("\n")
    )
}

/// Generate a cached cargo install script for a single crate.
///
/// On cache hit: copy pre-built binaries from cache, skip compilation entirely.
/// On cache miss: `cargo install --root <staging>`, then populate cache + install.
///
/// Supports `crate[feat1,feat2]` syntax — features are passed via `--features`
/// and included in the cache key to avoid feature-set collisions.
///
/// Detects empty staging bin dir (no binaries produced) and emits a clear error
/// with a hint about `--features`, instead of failing on `cp` with a cryptic message.
fn cargo_cached_install(pkg: &str, version: Option<&str>) -> String {
    let (crate_name, features) = parse_cargo_features(pkg);
    let ver_tag = version.unwrap_or("latest");
    let install_arg = match version {
        Some(v) => format!("'{crate_name}@{v}'"),
        None => format!("'{crate_name}'"),
    };
    let features_arg = if features.is_empty() {
        String::new()
    } else {
        format!(" --features '{}'", features.join(","))
    };
    let cache_suffix = if features.is_empty() {
        String::new()
    } else {
        format!("+{}", features.join(","))
    };
    format!(
        "_CACHE_KEY=\"{crate_name}-{ver_tag}{cache_suffix}-$(uname -m)\"\n\
         _CACHE_DIR=\"${{FORJAR_CACHE_DIR:-$HOME/.forjar/cache/cargo}}/$_CACHE_KEY\"\n\
         if [ -z \"${{FORJAR_NO_CARGO_CACHE:-}}\" ] && \
            [ -d \"$_CACHE_DIR/bin\" ] && \
            ls \"$_CACHE_DIR/bin/\"* >/dev/null 2>&1; then\n\
           cp \"$_CACHE_DIR/bin/\"* \"$_CARGO_BIN/\"\n\
           echo \"forjar: cache-hit {crate_name} [$_CACHE_KEY]\"\n\
         else\n\
           _STAGING=$(mktemp -d /tmp/forjar-cargo.XXXXXX)\n\
           cargo install --force --locked --root \"$_STAGING\"{features_arg} {install_arg}\n\
           if [ ! -d \"$_STAGING/bin\" ] || ! ls \"$_STAGING/bin/\"* >/dev/null 2>&1; then\n\
             echo \"ERROR: cargo install {crate_name} produced no binaries\" >&2\n\
             echo \"HINT: does the crate need --features? Use packages: [\\\"{crate_name}[feature_name]\\\"]\" >&2\n\
             rm -rf \"$_STAGING\"\n\
             exit 1\n\
           fi\n\
           if [ -z \"${{FORJAR_NO_CARGO_CACHE:-}}\" ]; then\n\
             mkdir -p \"$_CACHE_DIR\"\n\
             cp -a \"$_STAGING/bin\" \"$_CACHE_DIR/\"\n\
           fi\n\
           cp \"$_STAGING/bin/\"* \"$_CARGO_BIN/\"\n\
           rm -rf \"$_STAGING\"\n\
           echo \"forjar: cached {crate_name} [$_CACHE_KEY]\"\n\
         fi"
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
                .map(|p| {
                    let (crate_name, _) = parse_cargo_features(p);
                    format!("command -v '{crate_name}' >/dev/null 2>&1 && echo '{crate_name}=installed' || echo '{crate_name}=MISSING'")
                })
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
