//! FJ-3601..FJ-3603: Shell installer, Homebrew formula, cargo-binstall generators.

use super::dist_installer_shell::{
    installer_arg_parsing, installer_download_helpers, installer_output_helpers,
    installer_platform_detection, installer_resolve_version,
};
use crate::core::types::DistConfig;

// ── FJ-3601: Shell Installer ──

/// Build the `verify_checksum()` shell function snippet.
fn build_checksum_snippet(dist: &DistConfig) -> String {
    if dist.checksums.is_some() {
        let sums_file = dist.checksums.as_deref().unwrap_or("SHA256SUMS");
        format!(
            r#"
verify_checksum() {{
  SUMS_URL="https://github.com/${{REPO}}/releases/download/${{TAG}}/{sums_file}"
  info "downloading checksums..."
  CHECKSUMS=$(download "$SUMS_URL" 2>/dev/null) || CHECKSUMS=""
  if [ -z "$CHECKSUMS" ]; then
    # Fall back to the per-asset .sha256 the release workflow always uploads
    CHECKSUMS=$(download "https://github.com/${{REPO}}/releases/download/${{TAG}}/${{ASSET}}.sha256" 2>/dev/null) || CHECKSUMS=""
  fi
  [ -n "$CHECKSUMS" ] || die "failed to download checksums for $TAG"
  EXPECTED=$(echo "$CHECKSUMS" | grep "$ASSET" | awk '{{print $1}}')
  if [ -z "$EXPECTED" ]; then
    # A CHECKSUM FILE THAT DOES NOT MENTION THIS ASSET IS NOT A PASS.
    #
    # This used to `warn ... skipping verification` and INSTALL ANYWAY. The
    # dangerous case is not a missing SHA256SUMS — that path already dies above.
    # It is a STALE one: it downloads fine, so the per-asset fallback never
    # fires, and the grep simply finds nothing.
    #
    # forjar v1.18.0 came within one asset of this. Its SHA256SUMS was written
    # by a run that globbed a reused staging directory, so it carried four
    # entries for 1.17.0 alongside 1.18.0's. Had the macOS archives been
    # uploaded by the later run instead of the earlier one, every mac install
    # would have printed a warning and proceeded unverified.
    #
    # Try the per-asset sidecar before giving up, then refuse.
    CHECKSUMS=$(download "https://github.com/${{REPO}}/releases/download/${{TAG}}/${{ASSET}}.sha256" 2>/dev/null) || CHECKSUMS=""
    EXPECTED=$(echo "$CHECKSUMS" | grep "$ASSET" | awk '{{print $1}}')
  fi
  if [ -z "$EXPECTED" ]; then
    # Wording note: do NOT write "for $ASSET" here. bashrs's SC1086 parses that
    # literal sequence inside a string as a for-loop over an expanded variable
    # and fails the dist lint gate, which is a required check.
    die "$TAG publishes no checksum matching asset $ASSET -- refusing to install unverified"
  fi
  ACTUAL=$(compute_checksum "$ARCHIVE")
  if [ "$ACTUAL" != "$EXPECTED" ]; then
    die "checksum mismatch: expected $EXPECTED, got $ACTUAL"
  fi
  info "checksum verified"
}}"#
        )
    } else {
        r#"
verify_checksum() {
  info "no checksums configured -- skipping verification"
}"#
        .to_string()
    }
}

/// Build the `case` arms for OS/arch to asset resolution.
fn build_asset_cases(dist: &DistConfig) -> String {
    // Group targets by (os, arch) so multiple libc variants share one case arm.
    let mut grouped: indexmap::IndexMap<
        (String, String),
        Vec<&crate::core::types::DistBinaryTarget>,
    > = indexmap::IndexMap::new();
    for t in &dist.targets {
        grouped
            .entry((t.os.clone(), t.arch.clone()))
            .or_default()
            .push(t);
    }

    let mut asset_cases = String::new();
    for ((os, arch), targets) in &grouped {
        let body = build_case_body(targets);
        asset_cases.push_str(&format!(
            r#"
    {os}/{arch}){body}
      ;;"#
        ));
    }
    asset_cases
}

/// Build the body of a single OS/arch case arm, handling libc variants.
fn build_case_body(targets: &[&crate::core::types::DistBinaryTarget]) -> String {
    let mut body = String::new();
    let has_libc_variants = targets.iter().any(|t| t.libc.is_some());
    if has_libc_variants {
        for t in targets {
            if let Some(ref libc) = t.libc {
                body.push_str(&format!(
                    r#"
      if [ "$LIBC" = "{libc}" ]; then
        ASSET="{asset}"
      fi"#,
                    libc = libc,
                    asset = t.asset
                ));
            }
        }
        // Fallback: if no libc match, use the first target
        if let Some(first) = targets.first() {
            body.push_str(&format!(
                r#"
      [ -z "$ASSET" ] && ASSET="{}""#,
                first.asset
            ));
        }
    } else if let Some(t) = targets.first() {
        body.push_str(&format!(
            r#"
      ASSET="{}""#,
            t.asset
        ));
    }
    body
}

/// Build the `post_install()` shell function snippet.
fn build_post_install_snippet(dist: &DistConfig) -> String {
    if let Some(ref script) = dist.post_install {
        format!(
            r#"
post_install() {{
  {}
}}"#,
            script.trim()
        )
    } else {
        r#"
post_install() {
  :
}"#
        .to_string()
    }
}

/// Build the version-verification shell snippet (empty if no version_cmd).
fn build_version_verify_snippet(dist: &DistConfig) -> String {
    if let Some(ref cmd) = dist.version_cmd {
        // Anchor the check to the binary just installed — a bare command
        // resolves from PATH and reports a pre-existing installation.
        let anchored = cmd
            .strip_prefix(dist.binary.as_str())
            .map(|rest| format!(r#""$DEST/$BINARY"{rest}"#))
            .unwrap_or_else(|| cmd.clone());
        format!(
            r#"
  info "verifying install..."
  if {anchored} >/dev/null 2>&1; then
    info "$({anchored})"
  else
    warn "version check failed -- installed binary did not run"
  fi"#
        )
    } else {
        String::new()
    }
}

/// `resolve_asset()`, wrapping the generated per-OS/arch `case` arms.
fn installer_resolve_asset(asset_cases: &str) -> String {
    format!(
        r#"# ── Asset resolution ──

resolve_asset() {{
  OS=$(detect_os)
  ARCH=$(detect_arch)
  LIBC=$(detect_libc)
  ASSET=""

  case "$OS/$ARCH" in{asset_cases}
    *) die "no pre-built binary for $OS/$ARCH" ;;
  esac

  if [ -z "$ASSET" ]; then
    die "no matching asset for $OS/$ARCH (libc=$LIBC)"
  fi

  # Expand {{version}} placeholder
  VERSION_NUM="${{TAG#v}}"
  ASSET=$(echo "$ASSET" | sed "s/{{version}}/$VERSION_NUM/g")
}}"#
    )
}

/// `main()` — download, verify, extract, install, PATH hint.
fn installer_main(version_verify: &str) -> String {
    format!(
        r#"# ── Main ──

main() {{
  resolve_version
  resolve_asset

  ASSET_URL="https://github.com/${{REPO}}/releases/download/${{TAG}}/${{ASSET}}"
  TMPDIR=$(mktemp -d)
  ARCHIVE="$TMPDIR/$ASSET"
  trap 'rm -rf "$TMPDIR"' EXIT

  info "downloading $BINARY $TAG..."
  download_file "$ASSET_URL" "$ARCHIVE" || die "download failed: $ASSET_URL"

  verify_checksum

  info "extracting..."
  tar xzf "$ARCHIVE" -C "$TMPDIR" || die "extraction failed"

  # Archives contain a directory named after the asset; fall back to a
  # flat layout for older releases.
  SRC="$TMPDIR/${{ASSET%.tar.gz}}/$BINARY"
  [ -f "$SRC" ] || SRC="$TMPDIR/$BINARY"
  [ -f "$SRC" ] || die "binary not found in archive"

  # Determine install location
  DEST="${{PREFIX:-$INSTALL_DIR}}"
  if [ ! -w "$DEST" ] 2>/dev/null; then
    if [ -w "$FALLBACK_DIR" ] || install -d "$FALLBACK_DIR" 2>/dev/null; then
      DEST="$FALLBACK_DIR"
      warn "$INSTALL_DIR not writable, installing to $DEST"
    else
      # Try with sudo
      info "$INSTALL_DIR not writable, using sudo..."
      sudo install -d "$DEST" 2>/dev/null || die "cannot create $DEST"
      _fj_install_bin "$SRC" "$DEST/$BINARY" sudo || die "install failed"
      info "installed $BINARY to $DEST/$BINARY"
      post_install{version_verify}
      return
    fi
  fi

  # Check existing binary
  if [ -f "$DEST/$BINARY" ] && [ "$FORCE" = "0" ]; then
    warn "$DEST/$BINARY already exists -- use --force to overwrite"
    return 1
  fi

  install -d "$DEST" 2>/dev/null || true
  # ATOMIC: `curl | sh` is how a tool is UPGRADED, so the destination is
  # routinely the binary the user just ran. `cp` opens it in place and takes
  # ETXTBSY ("Text file busy"); rename(2) does not. See core::shell_install.
  _fj_install_bin "$SRC" "$DEST/$BINARY" || die "install failed"
  info "installed $BINARY to $DEST/$BINARY"

  post_install{version_verify}

  # PATH hint
  case ":$PATH:" in
    *":$DEST:"*) ;;
    *) warn "add $DEST to your PATH: export PATH=\"$DEST:\$PATH\"" ;;
  esac
}}"#
    )
}

/// Generate a POSIX-compliant shell installer script.
///
/// Emission order is load-bearing: every snippet that a later snippet
/// *calls* must come first. Concretely, `output_helpers` (which defines
/// `info`/`warn`/`die`/`usage`) precedes `arg_parsing` (which calls
/// `usage` and `die` at top level, before `main` ever runs).
pub fn generate_installer(dist: &DistConfig) -> String {
    let binary = &dist.binary;
    let repo = &dist.repo;
    let install_dir = &dist.install_dir;
    // A literal "~" in a quoted shell assignment never tilde-expands —
    // emit $HOME so the fallback dir actually resolves.
    let fallback_dir = dist
        .install_dir_fallback
        .strip_prefix("~/")
        .map(|rest| format!("$HOME/{rest}"))
        .unwrap_or_else(|| dist.install_dir_fallback.clone());
    let raw_url = format!("https://raw.githubusercontent.com/{repo}/main/install.sh");
    let description = if dist.description.is_empty() {
        binary
    } else {
        &dist.description
    };

    let checksum_verify = build_checksum_snippet(dist);
    let asset_cases = build_asset_cases(dist);
    let post_install = build_post_install_snippet(dist);
    let version_verify = build_version_verify_snippet(dist);
    let arg_parsing = installer_arg_parsing();
    let output_helpers = installer_output_helpers(binary, &raw_url);
    let platform_detection = installer_platform_detection();
    let download_helpers = installer_download_helpers();
    let install_helper = crate::core::shell_install::atomic_install_fn();
    let resolve_version = installer_resolve_version();
    let resolve_asset = installer_resolve_asset(&asset_cases);
    let main_body = installer_main(&version_verify);

    format!(
        r#"#!/bin/sh
# install.sh — generated by forjar dist (do not edit)
# {description}
# Usage: curl -sSf {raw_url} | sh
# Pinned: curl -sSf {raw_url} | sh -s -- --version v1.0.0
set -eu

BINARY="{binary}"
REPO="{repo}"
INSTALL_DIR="{install_dir}"
FALLBACK_DIR="{fallback_dir}"
TAG=""
FORCE=0
YES=0
PREFIX=""

{output_helpers}

{arg_parsing}

{platform_detection}

{download_helpers}

{install_helper}
{checksum_verify}

{resolve_version}

{resolve_asset}
{post_install}

{main_body}

main
"#
    )
}

// ── FJ-3602: Homebrew Formula — lives in dist_homebrew (PMAT-080) ──
pub use super::dist_homebrew::generate_homebrew;

// ── FJ-3603: cargo-binstall ──

pub fn generate_binstall(dist: &DistConfig) -> String {
    let repo_url = format!("https://github.com/{}", dist.repo);

    let mut overrides = String::new();
    for t in &dist.targets {
        let rust_target = super::dist_generators_b::to_rust_triple(t);
        let asset_tpl = t.asset.replace("{version}", "{ version }");
        overrides.push_str(&format!(
            r#"
[package.metadata.binstall.overrides.{rust_target}]
pkg-url = "{repo_url}/releases/download/v{{{{ version }}}}/{asset_tpl}"
"#
        ));
    }

    format!(
        r#"# Generated by forjar dist — paste into Cargo.toml
[package.metadata.binstall]
pkg-url = "{repo_url}/releases/download/v{{{{ version }}}}/{{{{ name }}}}-{{{{ version }}}}-{{{{ target }}}}{{{{ archive-suffix }}}}"
bin-dir = "{{{{ bin }}}}{{{{ binary-ext }}}}"
pkg-fmt = "tgz"
{overrides}"#
    )
}
