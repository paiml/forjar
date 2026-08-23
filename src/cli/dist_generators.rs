//! FJ-3601..FJ-3603: Shell installer, Homebrew formula, cargo-binstall generators.

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
    warn "no checksum found for $ASSET -- skipping verification"
    return
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

/// The `--version/--prefix/--force/--yes/--help` parser and the `..` guard on
/// `--prefix`. Emitted verbatim; no config value reaches it.
fn installer_arg_parsing() -> &'static str {
    r#"# ── Argument parsing ──

while [ $# -gt 0 ]; do
  case "$1" in
    --version) TAG="$2"; shift 2 ;;
    --prefix)  PREFIX="$2"; shift 2 ;;
    --force)   FORCE=1; shift ;;
    --yes|-y)  YES=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) die "unknown option: $1" ;;
  esac
done

# Refuse traversal sequences in user-supplied install paths
case "$PREFIX" in
  *..*) die "refusing --prefix containing '..'" ;;
esac"#
}

/// Colour setup, `info`/`warn`/`die`, and `usage()` — the only snippet that
/// names the binary and the raw install.sh URL.
fn installer_output_helpers(binary: &str, raw_url: &str) -> String {
    format!(
        r#"# ── Output helpers ──

RED='' GREEN='' YELLOW='' BOLD='' RESET=''
if [ -t 1 ]; then
  RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
  BOLD='\033[1m'; RESET='\033[0m'
fi

info()  {{ printf '%s%s%s %s\n' "$GREEN" "info:" "$RESET" "$1"; }}
warn()  {{ printf '%s%s%s %s\n' "$YELLOW" "warn:" "$RESET" "$1" >&2; }}
die()   {{ printf '%s%s%s %s\n' "$RED" "error:" "$RESET" "$1" >&2; exit 1; }}

usage() {{
  cat <<USAGE
Install {binary}

USAGE:
    sh install.sh
    sh install.sh --version v1.2.3
    (download first: curl -sSfO {raw_url})

OPTIONS:
    --version <TAG>   Install a specific version (e.g., v1.0.0)
    --prefix <DIR>    Install to a custom directory
    --force           Overwrite existing binary
    --yes, -y         Non-interactive mode
    --help, -h        Show this help
USAGE
}}"#
    )
}

/// `detect_os`, `detect_arch`, `detect_libc`.
fn installer_platform_detection() -> &'static str {
    r#"# ── Platform detection ──

detect_os() {
  case "$(uname -s)" in
    Linux*)  echo "linux" ;;
    Darwin*) echo "darwin" ;;
    *)       die "unsupported OS: $(uname -s)" ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)       echo "x86_64" ;;
    aarch64|arm64)      echo "aarch64" ;;
    *)                  die "unsupported architecture: $(uname -m)" ;;
  esac
}

detect_libc() {
  if [ "$(detect_os)" != "linux" ]; then
    echo "none"
    return
  fi
  if ldd --version 2>&1 | grep -qi musl; then
    echo "musl"
  elif command -v ldd >/dev/null 2>&1; then
    echo "gnu"
  else
    echo "musl"
  fi
}"#
}

/// `download`, `download_file`, `compute_checksum`.
fn installer_download_helpers() -> &'static str {
    r#"# ── Download helpers ──

download() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$1"
  else
    die "curl or wget required"
  fi
}

download_file() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$2" "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$2" "$1"
  else
    die "curl or wget required"
  fi
}

compute_checksum() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    warn "no sha256sum or shasum found -- skipping checksum"
    echo ""
  fi
}"#
}

/// `resolve_version()` — honour a pinned `--version`, else ask the releases API.
fn installer_resolve_version() -> &'static str {
    r#"# ── Version resolution ──

resolve_version() {
  if [ -n "$TAG" ]; then
    return
  fi
  info "resolving latest version..."
  TAG=$(download "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | head -1 | cut -d'"' -f4) \
    || die "failed to resolve latest version"
  if [ -z "$TAG" ]; then
    die "could not determine latest version"
  fi
  info "latest version: $TAG"
}"#
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
      sudo cp "$SRC" "$DEST/$BINARY" || die "install failed"
      sudo chmod +x "$DEST/$BINARY"
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
  cp "$SRC" "$DEST/$BINARY" || die "install failed"
  chmod +x "$DEST/$BINARY"
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

{arg_parsing}

{output_helpers}

{platform_detection}

{download_helpers}
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
