//! FJ-006: Package resource handler (apt + cargo + uv + brew).
//! FJ-1398: Cross-platform resource abstraction via brew provider.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;

/// Generate shell script to install packages.
pub fn apply_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let state = resource.state.as_deref().unwrap_or("present");

    match (provider, state) {
        ("apt", "present") => apply_apt_present(resource),
        ("apt", "absent") => apply_apt_absent(resource),
        ("apt", "latest") => apply_apt_latest(resource),
        ("cargo", "present") => apply_cargo_present(resource),
        ("cargo", "absent") => apply_cargo_absent(resource),
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

/// FJ-51: Cargo binary cache — skip recompilation when cached binary exists.
///
/// Cache layout: `$FORJAR_CACHE_DIR/<pkg>-<version>-<arch>/bin/`
/// Default cache dir: `~/.forjar/cache/cargo`
/// Disable: `FORJAR_NO_CARGO_CACHE=1`
///
/// Supports `crate[feat1,feat2]` syntax in package names to pass `--features`
/// to `cargo install`. Example: `packages: ["whisper-apr[cli]"]`.
/// True if a cargo crate name / feature uses only the cargo-legal charset
/// (`[A-Za-z0-9._-]`). Used to reject names that would otherwise be
/// interpolated into the double-quoted cache key, where `$(...)`/backticks
/// would otherwise be live.
fn is_safe_cargo_token(tok: &str) -> bool {
    !tok.is_empty()
        && tok
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// True if `version` is a safe cargo version requirement charset.
fn is_safe_cargo_version(ver: &str) -> bool {
    !ver.is_empty()
        && ver.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+' | '*' | '~' | '^')
        })
}

/// Validate every cargo package spec (crate name + features) and the optional
/// version against the cargo-legal charset. Returns the offending token on
/// the first failure.
fn first_unsafe_cargo_token<'a>(
    packages: &'a [String],
    version: Option<&'a str>,
) -> Option<&'a str> {
    if let Some(v) = version {
        if !is_safe_cargo_version(v) {
            return Some(v);
        }
    }
    for p in packages {
        let (crate_name, features) = parse_cargo_features(p);
        if !is_safe_cargo_token(crate_name) {
            return Some(crate_name);
        }
        if let Some(bad) = features.into_iter().find(|f| !is_safe_cargo_token(f)) {
            return Some(bad);
        }
    }
    None
}

fn apply_cargo_present(resource: &Resource) -> String {
    let packages = &resource.packages;
    let version = resource.version.as_deref();
    let source = resource.source.as_deref();

    // FJ-154: reject crate/feature/version tokens that aren't cargo-legal,
    // since they flow into a double-quoted cache key where command
    // substitution would otherwise be live. Path installs (source set) skip
    // the cache, so they only need the install arg escaped (done below).
    if source.is_none() {
        if let Some(bad) = first_unsafe_cargo_token(packages, version) {
            return format!(
                "echo {} >&2; exit 1",
                sh_squote(&format!("ERROR: unsafe cargo package/version token: {bad}"))
            );
        }
    }

    let installs: Vec<String> = packages
        .iter()
        .map(|p| match (source, version) {
            // Local path installs — no caching, always rebuild
            (Some(s), _) => {
                let (_, features) = parse_cargo_features(p);
                let features_arg = if features.is_empty() {
                    String::new()
                } else {
                    format!(" --features {}", sh_squote(&features.join(",")))
                };
                format!(
                    "cargo install --force --locked --path {}{features_arg}",
                    sh_squote(s)
                )
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
         _CRATES_TOML=\"${{CARGO_HOME:-$HOME/.cargo}}/.crates.toml\"\n\
         # TELL CARGO WHAT WE INSTALLED (forjar#320).\n\
         #\n\
         # `cargo install --root $_STAGING` writes its registry entry to\n\
         # $_STAGING/.crates.toml. We copy only bin/* out and then delete the\n\
         # staging dir, so $CARGO_HOME/.crates.toml never learns about the\n\
         # binaries we just put in $CARGO_HOME/bin. `cargo install --list` then\n\
         # reports the crate MISSING forever, and package_check.rs reads exactly\n\
         # that -- so forjar failed its own check for work it had done.\n\
         #\n\
         # Measured on gx10: rg/fd/bat/hyperfine installed and working, cargo\n\
         # naming none of them; the same registry claiming forjar 1.16.0 on a\n\
         # box running 1.18.0. Wrong in BOTH directions.\n\
         #\n\
         # APPEND-ONLY AND KEYED. `.crates.toml` is `[v1]` followed by one line\n\
         # per install, keyed `\"name ver (source)\" = [\"bin\", ...]`. We drop any\n\
         # existing line for this crate name and append the new one, so a\n\
         # reinstall updates rather than duplicating.\n\
         #\n\
         # Deliberately NOT a TOML parser: this is generated POSIX shell running\n\
         # on hosts that may lack python, and a half-written .crates.toml breaks\n\
         # `cargo install` for every crate on the machine. Write to a temp file\n\
         # and `mv` -- atomic within a filesystem -- so an interrupted run leaves\n\
         # the original intact.\n\
         _fj_register() {{\n\
           _src=\"$1\"\n\
           [ -f \"$_src\" ] || return 0\n\
           _line=$(grep -v '^\\[v1\\]' \"$_src\" | grep -v '^[[:space:]]*$' | head -1)\n\
           [ -n \"$_line\" ] || return 0\n\
           _key=$(printf '%s' \"$_line\" | sed 's/^\"\\([^ ]*\\) .*/\\1/')\n\
           [ -n \"$_key\" ] || return 0\n\
           _tmp=$(mktemp \"${{_CRATES_TOML}}.forjar.XXXXXX\") || return 0\n\
           echo '[v1]' > \"$_tmp\"\n\
           if [ -f \"$_CRATES_TOML\" ]; then\n\
             grep -v '^\\[v1\\]' \"$_CRATES_TOML\" | grep -v \"^\\\"$_key \" >> \"$_tmp\" || true\n\
           fi\n\
           printf '%s\\n' \"$_line\" >> \"$_tmp\"\n\
           mv -f \"$_tmp\" \"$_CRATES_TOML\"\n\
         }}\n\
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
///
/// Uses `install`, not `cp`, to place the binaries. `cp` REFUSES to overwrite a
/// dangling symlink — "cp: not writing through dangling symlink" — and that is
/// precisely the wreckage this resource has to repair: a CI cache-prune step
/// deletes the real files in a shared `~/.cargo/bin` and leaves the symlinks
/// behind, pointing at nothing.
///
/// Measured on paiml/infra's intel 2026-08-19: with `pzsh` reduced to a dangling
/// symlink, `forjar apply --refresh` correctly DETECTED the divergence and then
/// died on `cp`, so it could see the damage and not fix it. `cp -f` does not
/// help — coreutils refuses that too (verified on the host). `install` replaces
/// the destination outright, and unlike `cp --remove-destination` it is not
/// GNU-only, so it also works on the fleet's macOS box.
fn cargo_cached_install(pkg: &str, version: Option<&str>) -> String {
    let (crate_name, features) = parse_cargo_features(pkg);
    let ver_tag = version.unwrap_or("latest");
    let install_arg = match version {
        Some(v) => sh_squote(&format!("{crate_name}@{v}")),
        None => sh_squote(crate_name),
    };
    let features_arg = if features.is_empty() {
        String::new()
    } else {
        format!(" --features {}", sh_squote(&features.join(",")))
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
           install -m 755 \"$_CACHE_DIR/bin/\"* \"$_CARGO_BIN/\"\n\
           _fj_register \"$_CACHE_DIR/.crates.toml\"\n\
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
             cp -f \"$_STAGING/.crates.toml\" \"$_CACHE_DIR/.crates.toml\" 2>/dev/null || true\n\
           fi\n\
           install -m 755 \"$_STAGING/bin/\"* \"$_CARGO_BIN/\"\n\
           _fj_register \"$_STAGING/.crates.toml\"\n\
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
            Some(v) => format!(
                "uv tool install --force {}",
                sh_squote(&format!("{p}=={v}"))
            ),
            None => format!("uv tool install --force {}", sh_squote(p)),
        })
        .collect();
    format!("set -euo pipefail\n{}", installs.join("\n"))
}

/// Remove cargo-installed crates.
///
/// forjar#278: this arm did not exist, so `(cargo, absent)` fell to the
/// catch-all, echoed, and reported converged — a declared removal that never
/// removed anything.
///
/// `|| true` matches the apt/uv/brew absent arms: uninstalling a crate that is
/// not installed is the desired end state, not a failure. The check script is
/// what decides convergence, and it asks whether the crate is gone.
fn apply_cargo_absent(resource: &Resource) -> String {
    let packages = &resource.packages;
    let removals: Vec<String> = packages
        .iter()
        .map(|p| {
            let (crate_name, _) = parse_cargo_features(p);
            format!(
                "cargo uninstall {} 2>/dev/null || true",
                sh_squote(crate_name)
            )
        })
        .collect();
    format!("set -euo pipefail\n{}", removals.join("\n"))
}

fn apply_uv_absent(resource: &Resource) -> String {
    let packages = &resource.packages;
    let removals: Vec<String> = packages
        .iter()
        .map(|p| format!("uv tool uninstall {} 2>/dev/null || true", sh_squote(p)))
        .collect();
    format!("set -euo pipefail\n{}", removals.join("\n"))
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
    let packages = &resource.packages;
    let removals: Vec<String> = packages
        .iter()
        .map(|p| format!("brew uninstall {} 2>/dev/null || true", sh_squote(p)))
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
                .map(|p| {
                    format!(
                        "dpkg-query -W -f '${{Package}}=${{Version}}\\n' {} 2>/dev/null || echo {}",
                        sh_squote(p),
                        sh_squote(&format!("{p}=MISSING"))
                    )
                })
                .collect();
            queries.join("\n")
        }
        "cargo" => {
            // GH-257: ask cargo, not the PATH — see package_check.rs for the
            // full reasoning. This one feeds DRIFT, so its blindness is the
            // more expensive half: with `command -v <crate_name>`, a crate
            // whose binary is named differently (kani-verifier -> cargo-kani)
            // reads as MISSING forever, and a dangling symlink reads as
            // installed. Neither state produces a useful drift signal, which is
            // why an intel host lost rustup, cargo and forjar without a single
            // drift finding.
            // ...AND CHECK THE BINARIES, NOT ONLY THE REGISTRATION.
            //
            // `cargo install --list` reads $CARGO_HOME/.crates.toml — METADATA.
            // It does not stat anything. So when $CARGO_HOME/bin is pruned (which
            // on this fleet is routine: Swatinem/rust-cache's POST step does it,
            // and 16 runners share one $HOME) every binary dies, .crates.toml
            // survives, and this observable keeps reporting `installed`.
            //
            // Measured 2026-08-24 on intel: `cargo-kani` and `kani` both absent
            // from PATH, `~/.kani` intact, and `forjar drift` across the whole
            // machine reported "No drift detected" eight times. The comment this
            // replaces already named the symptom — "why an intel host lost
            // rustup, cargo and forjar without a single drift finding" — and
            // then picked an observable that cannot see it either.
            //
            // Registration alone is not installation. `command -v <crate>` alone
            // is worse (kani-verifier installs `cargo-kani`, not `kani-verifier`,
            // and a dangling symlink reads as present). So do BOTH: take the
            // binary names cargo itself lists under the crate, and require each
            // to exist and be executable.
            //
            // `cargo install --list` prints:
            //     kani-verifier v0.67.0:
            //         cargo-kani
            //         kani
            // Top-level lines are unindented; binaries are indented beneath.
            // Order is stable, so the digest is stable. (paiml/infra#208.)
            let queries: Vec<String> = packages
                .iter()
                .map(|p| {
                    let (crate_name, _) = parse_cargo_features(p);
                    let awk = format!(
                        "awk -v c={} '/^[^[:space:]]/{{inblk=($1==c)}} inblk&&/^[[:space:]]/{{print $1}}'",
                        sh_squote(crate_name)
                    );
                    format!(
                        "if cargo install --list 2>/dev/null | grep -q {reg}; then\n                           bins=$(cargo install --list 2>/dev/null | {awk})\n                           if [ -z \"$bins\" ]; then echo {noBins}\n                           else\n                             st=''\n                             for b in $bins; do\n                               if command -v \"$b\" >/dev/null 2>&1; then st=\"$st$b:ok,\"\n                               else st=\"$st$b:GONE,\"; fi\n                             done\n                             echo {crate}=installed:\"$st\"\n                           fi\n                         else echo {missing}; fi",
                        reg = sh_squote(&format!("^{crate_name} v")),
                        awk = awk,
                        noBins = sh_squote(&format!("{crate_name}=installed:NO-BINARIES-LISTED")),
                        crate = crate_name,
                        missing = sh_squote(&format!("{crate_name}=MISSING")),
                    )
                })
                .collect();
            queries.join("\n")
        }
        "uv" => {
            let queries: Vec<String> = packages
                .iter()
                .map(|p| {
                    format!(
                        "uv tool list 2>/dev/null | grep -q {} && echo {} || echo {}",
                        sh_squote(&format!("^{p}")),
                        sh_squote(&format!("{p}=installed")),
                        sh_squote(&format!("{p}=MISSING"))
                    )
                })
                .collect();
            queries.join("\n")
        }
        "brew" => {
            let queries: Vec<String> = packages
                .iter()
                .map(|p| {
                    format!(
                        "brew list --versions {} 2>/dev/null || echo {}",
                        sh_squote(p),
                        sh_squote(&format!("{p}=MISSING"))
                    )
                })
                .collect();
            queries.join("\n")
        }
        other => format!("echo 'unsupported provider: {other}'"),
    }
}
