//! Package `check_script` generation.
//!
//! Split out of `package.rs` to keep both files under the 500-line limit. The
//! four providers each assert one line per package so a partially-installed
//! set names the packages that are actually missing (FJ-2720).

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::package::parse_cargo_features;
use crate::resources::verdict;

/// Generate shell script to check if packages are installed.
pub fn check_script(resource: &Resource) -> String {
    let provider = resource.provider.as_deref().unwrap_or("apt");
    let packages = &resource.packages;

    match provider {
        "apt" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| {
                    let q = sh_squote(p);
                    verdict::assert_that(
                        &format!("dpkg -l {q} 2>/dev/null | grep -q '^ii '"),
                        &format!("installed:{p}"),
                        &format!("missing:{p}"),
                    )
                })
                .collect();
            verdict::check_script_from(&checks)
        }
        "cargo" => {
            // GH-257: ask CARGO what it installed, not the PATH.
            //
            // This was `command -v <crate_name>`, which is wrong twice over.
            //
            // 1. A crate's name is not its binary's name. `kani-verifier`
            //    installs `cargo-kani` and `kani`; `command -v kani-verifier`
            //    can never succeed, so the resource reported missing forever
            //    even with the crate installed and working. Observed on intel:
            //    `forjar check -r kani-verifier` FAILED while
            //    `cargo-kani --version` printed 0.67.0.
            //
            // 2. `command -v` tests that a path exists and carries the
            //    executable bit — not that it RUNS. A dangling symlink passes.
            //    Also observed on intel: ~/.cargo/bin/rustup was gone while
            //    ~/.cargo/bin/cargo remained as a symlink to it, and every
            //    existence-check was satisfied by a link that executed nothing.
            //
            // `cargo install --list` is cargo's own record of what it
            // installed, keyed by CRATE name and carrying the version:
            //
            //     kani-verifier v0.67.0:
            //         cargo-kani
            //         kani
            //
            // So it answers the question actually being asked, and it does so
            // without depending on PATH — which differs between a login shell
            // and the runner service that will use the tool.
            let version = resource.version.as_deref();
            let checks: Vec<String> = packages
                .iter()
                .map(|p| {
                    let (crate_name, _) = parse_cargo_features(p);
                    // Anchor on the crate name and the ` v` that precedes the
                    // version, so `pmat` cannot be satisfied by `pmat-extra`.
                    let needle = match version {
                        Some(v) => format!("^{crate_name} v{v}:"),
                        None => format!("^{crate_name} v"),
                    };
                    verdict::assert_that(
                        &format!(
                            "cargo install --list 2>/dev/null | grep -q {}",
                            sh_squote(&needle)
                        ),
                        &format!("installed:{crate_name}"),
                        &format!("missing:{crate_name}"),
                    )
                })
                .collect();
            verdict::check_script_from(&checks)
        }
        "uv" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| {
                    verdict::assert_that(
                        &format!(
                            "uv tool list 2>/dev/null | grep -q {}",
                            sh_squote(&format!("^{p}"))
                        ),
                        &format!("installed:{p}"),
                        &format!("missing:{p}"),
                    )
                })
                .collect();
            verdict::check_script_from(&checks)
        }
        "brew" => {
            let checks: Vec<String> = packages
                .iter()
                .map(|p| {
                    let q = sh_squote(p);
                    verdict::assert_that(
                        &format!("brew list {q} >/dev/null 2>&1"),
                        &format!("installed:{p}"),
                        &format!("missing:{p}"),
                    )
                })
                .collect();
            verdict::check_script_from(&checks)
        }
        other => verdict::check_script_from(&[verdict::always_diverged(&format!(
            "unsupported provider: {other}"
        ))]),
    }
}
