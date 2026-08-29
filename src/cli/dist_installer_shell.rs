//! The generated installer's static shell prelude.
//!
//! Split out of `dist_generators.rs`, which had grown past the 500-line file
//! limit. These are the snippets that are emitted verbatim or from a single
//! string: argument parsing, output helpers, platform detection, the download
//! helpers and tag resolution. Nothing here reads `DistConfig.targets` — the
//! parts that do (asset cases, checksums, `main`) stayed behind.
//!
//! EMISSION ORDER IS LOAD-BEARING. `sh` resolves a function only once its
//! definition has been executed, so every snippet that CALLS another must be
//! emitted after it. `generate_installer` owns that ordering; these functions
//! only supply the text.

/// The `--version/--prefix/--force/--yes/--help` parser and the `..` guard on
/// `--prefix`. Emitted verbatim; no config value reaches it.
///
/// Calls `usage` and `die`, so it MUST be emitted after
/// [`installer_output_helpers`] — `sh` resolves a function only once its
/// definition has been executed, and a forward call is valid syntax that
/// `sh -n` cannot catch (it dies at runtime with 127 `usage: not found`).
/// `dist --verify` executes `--help` to hold that ordering.
pub(crate) fn installer_arg_parsing() -> &'static str {
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
pub(crate) fn installer_output_helpers(binary: &str, raw_url: &str) -> String {
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
pub(crate) fn installer_platform_detection() -> &'static str {
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
pub(crate) fn installer_download_helpers() -> &'static str {
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
pub(crate) fn installer_resolve_version() -> &'static str {
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
