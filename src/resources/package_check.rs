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
            // So it answers the crate-name-vs-binary-name question, and it does
            // so without depending on PATH — which differs between a login
            // shell and the runner service that will use the tool.
            //
            // 3. BUT A RECORD IS NOT A BINARY. Consulting the record alone
            //    traded "a path exists" for "an entry exists"; neither asks the
            //    tool to RUN, which is the only thing the resource cares about.
            //
            //    Measured on intel 2026-08-19 08:01: rust-cache's post-step
            //    deletes every real file in the shared ~/.cargo/bin and leaves
            //    the symlinks dangling. `.crates2.json` is a different file and
            //    survives, so `cargo install --list` kept reporting a full
            //    inventory of binaries that were gone. `forjar apply
            //    -t stack-tools` then reported rustup-installer and
            //    stack-tool-{copia,forjar,pmat,pzsh} as `no changes` on a host
            //    with no rustup, no cargo and no rustc — 5 of 5 falsely
            //    converged. A plain apply would have restored nothing, exited
            //    0, and left the six newly-declared tools to `cargo install`
            //    with no cargo present.
            //
            // So: ask the RECORD which binaries the crate owns, then ask each
            // BINARY to run. `--version` is the cheapest question that
            // separates "present" from "usable" — the same standard the fleet's
            // own pre-job hook applies (machines/clean-room/runner/pre-job.sh),
            // and the same reason `runner-toolchain-install` runs
            // `cargo --version` instead of trusting a directory name.
            //
            // Resolution goes through $CARGO_HOME/bin, cargo's deterministic
            // install location, so the PATH-independence above is preserved: a
            // dangling symlink fails `--version`, and a binary missing from the
            // service PATH but present where cargo put it still passes.
            //
            // AWK, not grep: the record is a two-level structure and the
            // binaries are the indented continuation lines under their crate.
            const BINS_UNDER_HEADER: &str =
                r#"$0 ~ p {f=1; next} /^[^[:space:]]/ {f=0} f {print $1}"#;
            let version = resource.version.as_deref();
            let checks: Vec<String> = packages
                .iter()
                .map(|p| {
                    let (crate_name, _) = parse_cargo_features(p);
                    // Anchor on the crate name and the ` v` that precedes the
                    // version, so `pmat` cannot be satisfied by `pmat-extra`.
                    //
                    // The VERSION is deliberately NOT part of this needle. The
                    // record's header carries the source dir for a path install:
                    //     probador v1.0.3 (/home/noah/src/probar/crates/...):
                    // so `^probador v1.0.3:` can never match one. Found by
                    // dogfooding on intel, where probador was installed, running,
                    // and at exactly its pinned version — and reported missing.
                    //
                    // The record is used for the one thing it alone knows: WHICH
                    // binaries this crate owns. The version comes from the
                    // binary below.
                    let needle = format!("^{crate_name} v");
                    // A crate recorded with NO binaries cannot satisfy the
                    // check either — `[ -n ]` makes an empty extraction fail
                    // rather than vacuously pass an empty for-loop.
                    // A `cargo-X` binary is a cargo SUBCOMMAND: cargo invokes it
                    // as `cargo-X X ...`, so argv[1] must be the subcommand name
                    // and a bare `--version` is an error for stricter parsers.
                    // Measured on intel after the 2026-08-19 restore:
                    // `cargo-mutants --version` exits 1 ("unexpected argument"),
                    // `cargo mutants --version` prints 27.1.0. Same for
                    // cargo-llvm-cov; cargo-deny and cargo-nextest accept both.
                    //
                    // Without the fallback this check reports two working tools
                    // missing forever, and forjar rebuilds a ~10-minute compile
                    // on every apply — a different lie, not a fix. The fallback
                    // is narrow (only `cargo-*`, only after the plain form has
                    // already failed), so it cannot rescue an absent binary.
                    // The pin is checked against what the BINARY reports, not
                    // what the record claims. Dogfooded on intel: cargo's record
                    // said `copia v0.1.3` while `copia --version` printed 0.2.0
                    // — the binary had been replaced out of band and the record
                    // was stale. Taking existence from the binary and version
                    // from the record applies the lesson by halves and reports a
                    // correctly-installed crate as missing.
                    //
                    // `_fj_seen` requires at least ONE of the crate's binaries
                    // to report the pin, not all of them: a multi-binary crate
                    // may ship a helper that versions itself differently, and
                    // failing on that would be a new false negative.
                    let condition = format!(
                        "_fj_bins=\"$(cargo install --list 2>/dev/null \
                         | awk -v p={} {})\"; \
                         [ -n \"$_fj_bins\" ] && ( \
                         _fj_root=\"${{CARGO_HOME:-$HOME/.cargo}}/bin\"; \
                         _fj_want={}; _fj_seen=0; \
                         for _fj_b in $_fj_bins; do \
                         _fj_p=\"$_fj_root/$_fj_b\"; \
                         _fj_v=\"$(\"$_fj_p\" --version 2>/dev/null)\" || _fj_v=\"\"; \
                         if [ -z \"$_fj_v\" ]; then case \"$_fj_b\" in cargo-*) \
                         _fj_v=\"$(\"$_fj_p\" \"${{_fj_b#cargo-}}\" --version 2>/dev/null)\" \
                         || _fj_v=\"\";; esac; fi; \
                         [ -n \"$_fj_v\" ] || exit 1; \
                         if [ -z \"$_fj_want\" ]; then _fj_seen=1; \
                         else case \"$_fj_v\" in *\"$_fj_want\"*) _fj_seen=1;; esac; fi; \
                         done; \
                         [ \"$_fj_seen\" = 1 ] )",
                        sh_squote(&needle),
                        sh_squote(BINS_UNDER_HEADER),
                        sh_squote(version.unwrap_or("")),
                    );
                    verdict::assert_that(
                        &condition,
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
