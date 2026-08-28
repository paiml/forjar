#!/usr/bin/env bash
# Assert that a published GitHub release object is a description of ITSELF.
#
# forjar#325. v1.18.0 shipped with four `forjar-1.17.0-*.tar.gz` assets attached
# to it and a SHA256SUMS carrying 10 lines for 6 real archives — four of them
# describing a version that release is not. #324 fixed one of the two producers
# (release.yml staged into a fixed path on a non-ephemeral runner). Nothing ever
# read the finished object BACK, so the same defect reached v1.19.0, v1.20.0 and
# v1.20.1 unnoticed: nightly.yml asserts only that every `v*` tag HAS a release,
# never that the release describes itself.
#
# TWO SURFACES, TWO PRODUCERS. binary-release.yml writes `<archive>.tar.gz` and
# `<archive>.tar.gz.sha256` as a pair; release.yml later `--clobber`s the same
# tarball name with a different build. Nothing orders them, so a sidecar can name
# bytes that no longer exist while SHA256SUMS names the ones that do. Both
# surfaces are checked against each other here, because agreeing with one of them
# is not agreeing with the release.
#
# READ THE ASSET LIST WITH `gh api`, NEVER `gh release view`. During the #325
# investigation both `gh release view --json assets` and the CDN download URLs
# served PRE-repair content for a release that had already been fixed; only
# `gh api repos/OWNER/REPO/releases/tags/TAG` was fresh. Anyone verifying this by
# hand and using `gh release view` will "confirm" a state that no longer exists.
#
# Usage:
#   release-object-audit.sh check  <tag>            read-only, exit 1 on violations
#   release-object-audit.sh repair <tag> [--apply]  prints the gh commands;
#                                                   --apply actually runs them
#
# Offline test hooks (tests/falsification_release_object.rs drives these):
#   FORJAR_AUDIT_ASSETS_FILE   newline-separated asset names, instead of gh api
#   FORJAR_AUDIT_SUMS_FILE     a SHA256SUMS file, instead of downloading one
#   FORJAR_AUDIT_SIDECAR_DIR   a directory of *.tar.gz.sha256 files
set -euo pipefail

REPO="${FORJAR_AUDIT_REPO:-paiml/forjar}"

die() {
  printf '%s\n' "$*" >&2
  exit 2
}

usage() {
  printf 'usage: release-object-audit.sh check <tag>\n' >&2
  printf '       release-object-audit.sh repair <tag>\n' >&2
  printf '       release-object-audit.sh repair <tag> --apply\n' >&2
  exit 2
}

# `rm -rf` on a variable is a foot-gun exactly once. Every path this script
# removes is a child of the mktemp workdir, so anything else is refused rather
# than deleted.
scrub() {
  local path="${1:-}"
  [ -n "${WORK:-}" ] || die "internal: no audit workdir"
  case "$path" in
  *..*) die "refusing to remove '$path': it contains .." ;;
  "$WORK" | "$WORK"/*) ;;
  *) die "refusing to remove '$path': not under $WORK" ;;
  esac
  if [ -z "$path" ] || [ "$path" = "/" ]; then
    die "refusing to remove an empty or root path"
  fi
  rm -rf -- "$path"
}

# ---------------------------------------------------------------- gathering --

asset_names() {
  if [ -n "${FORJAR_AUDIT_ASSETS_FILE:-}" ]; then
    cat -- "$FORJAR_AUDIT_ASSETS_FILE"
    return 0
  fi
  gh api "repos/$REPO/releases/tags/$1" --jq '.assets[].name'
}

fetch_sums() {
  if [ -n "${FORJAR_AUDIT_SUMS_FILE:-}" ]; then
    cp -- "$FORJAR_AUDIT_SUMS_FILE" "$2/SHA256SUMS"
    return 0
  fi
  gh release download "$1" --repo "$REPO" --pattern 'SHA256SUMS' \
    --dir "$2" --clobber >/dev/null 2>&1 || : >"$2/SHA256SUMS"
  [ -f "$2/SHA256SUMS" ] || : >"$2/SHA256SUMS"
}

fetch_sidecars() {
  mkdir -p "$2/sidecars"
  if [ -n "${FORJAR_AUDIT_SIDECAR_DIR:-}" ]; then
    cp -- "$FORJAR_AUDIT_SIDECAR_DIR"/*.sha256 "$2/sidecars/" 2>/dev/null || true
    return 0
  fi
  gh release download "$1" --repo "$REPO" --pattern '*.tar.gz.sha256' \
    --dir "$2/sidecars" --clobber >/dev/null 2>&1 || true
}

gather_release() {
  asset_names "$1" >"$2/assets.txt"
  fetch_sums "$1" "$2"
  fetch_sidecars "$1" "$2"
}

# ---------------------------------------------------------------- accessors --

own_tarballs() {
  local name
  while IFS= read -r name; do
    case "$name" in
    forjar-"$2"-*.tar.gz) printf '%s\n' "$name" ;;
    *) ;;
    esac
  done <"$1/assets.txt"
}

# `sha256sum` writes "<digest>  <name>"; the binary-mode form prefixes the name
# with `*`, so strip it rather than reporting every asset as missing.
sums_names() {
  awk '{ n=$2; sub(/^\*/, "", n); if (n != "") print n }' "$1/SHA256SUMS"
}

sums_digest() {
  awk -v want="$2" '{ n=$2; sub(/^\*/, "", n); if (n == want) { print $1; exit } }' \
    "$1/SHA256SUMS"
}

count_lines() {
  wc -l <"$1" | tr -d ' '
}

# --------------------------------------------------------------- invariants --

# (a) every archive and sidecar attached to the release belongs to this release.
check_strays() {
  local bad=0 name
  while IFS= read -r name; do
    case "$name" in
    forjar-"$2"-*.tar.gz | forjar-"$2"-*.tar.gz.sha256) ;;
    *.tar.gz | *.tar.gz.sha256)
      printf 'stray: %s\n' "$name"
      bad=1
      ;;
    *) ;;
    esac
  done <"$1/assets.txt"
  return "$bad"
}

# (b) the denominator is non-zero. An EMPTY release would otherwise sail through
# a "no strays" check while publishing nothing — the same hole release.yml's
# staging guard closes for the producing side.
check_denominator() {
  local n
  n=$(count_lines "$1/own.txt")
  if [ "$n" -eq 0 ]; then
    printf 'no-archives: the release carries no forjar-%s-*.tar.gz asset\n' "$2"
    return 1
  fi
  return 0
}

# (c) SHA256SUMS names exactly the archives this release carries.
check_sums() {
  local bad=0 name
  printf 'SHA256SUMS describes %s archives, release has %s\n' \
    "$(count_lines "$1/sums.txt")" "$(count_lines "$1/own.txt")"
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    printf 'sums-extra: %s\n' "$name"
    bad=1
  done < <(comm -23 "$1/sums.txt" "$1/own.txt")
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    printf 'sums-missing: %s\n' "$name"
    bad=1
  done < <(comm -13 "$1/sums.txt" "$1/own.txt")
  return "$bad"
}

# (d) every archive has a sidecar, and the sidecar agrees with SHA256SUMS.
check_sidecars() {
  local bad=0 name side sdig mdig
  while IFS= read -r name; do
    side="$1/sidecars/$name.sha256"
    if [ ! -f "$side" ]; then
      printf 'sidecar-missing: %s.sha256\n' "$name"
      bad=1
      continue
    fi
    sdig=$(awk '{ print $1; exit }' "$side")
    mdig=$(sums_digest "$1" "$name")
    if [ "$sdig" != "$mdig" ]; then
      printf 'sidecar-disagrees: %s sidecar=%s sums=%s\n' "$name" "$sdig" "$mdig"
      bad=1
    fi
  done <"$1/own.txt"
  return "$bad"
}

# ------------------------------------------------------------- subcommands ---

# Every invariant runs; none short-circuits. A report that stops at the first
# violation makes a contaminated object look like it has one problem.
cmd_check() {
  local tag="$1" ver dir fail=0
  ver="${tag#v}"
  dir="$WORK/check"
  scrub "$dir"
  mkdir -p "$dir"
  gather_release "$tag" "$dir"
  own_tarballs "$dir" "$ver" | sort >"$dir/own.txt"
  sums_names "$dir" | sort >"$dir/sums.txt"

  check_strays "$dir" "$ver" || fail=1
  check_denominator "$dir" "$ver" || fail=1
  check_sums "$dir" "$ver" || fail=1
  check_sidecars "$dir" "$ver" || fail=1

  if [ "$fail" -eq 0 ]; then
    printf 'ok: %s describes itself\n' "$tag"
  else
    printf 'CONTAMINATED: %s is not a description of itself\n' "$tag"
  fi
  return "$fail"
}

run_or_show() {
  if [ "$APPLY" = "yes" ]; then
    "$@"
  else
    printf 'would run:'
    printf ' %s' "$@"
    printf '\n'
  fi
}

delete_strays() {
  local name
  while IFS= read -r name; do
    case "$name" in
    forjar-"$2"-*.tar.gz | forjar-"$2"-*.tar.gz.sha256 | SHA256SUMS) ;;
    *.tar.gz | *.tar.gz.sha256)
      run_or_show gh release delete-asset "$1" "$name" --repo "$REPO" --yes
      ;;
    *) ;;
    esac
  done <"$3/assets.txt"
}

# Regenerate BOTH surfaces from the downloaded bytes. Never from an existing
# checksum file: on v1.20.1 the published Linux sidecars named a build that had
# already been clobbered, so a regeneration that trusted them would have
# laundered the wrong digest into SHA256SUMS as well.
regenerate_sums() {
  local dir="$1" tag="$2" ver="$3" f
  ( cd "$dir/bytes" && sha256sum -- forjar-"$ver"-*.tar.gz >SHA256SUMS )
  for f in "$dir"/bytes/forjar-"$ver"-*.tar.gz; do
    ( cd "$dir/bytes" && sha256sum -- "$(basename "$f")" >"$(basename "$f").sha256" )
  done
  run_or_show gh release upload "$tag" \
    "$dir/bytes/SHA256SUMS" --repo "$REPO" --clobber
  for f in "$dir"/bytes/forjar-"$ver"-*.tar.gz.sha256; do
    run_or_show gh release upload "$tag" "$f" --repo "$REPO" --clobber
  done
}

# Strays MUST go before SHA256SUMS is rewritten. binary-release.yml's checksums
# job regenerates SHA256SUMS from `gh release download --pattern '*.tar.gz'`,
# i.e. from whatever is attached to the release right now, so a stray left in
# place is laundered straight back into the authoritative checksum file.
cmd_repair() {
  local tag="$1" ver dir
  ver="${tag#v}"
  dir="$WORK/repair"
  mkdir -p "$dir"
  gather_release "$tag" "$dir"
  delete_strays "$tag" "$ver" "$dir"
  mkdir -p "$dir/bytes"
  gh release download "$tag" --repo "$REPO" \
    --pattern "forjar-$ver-*.tar.gz" --dir "$dir/bytes" --clobber
  regenerate_sums "$dir" "$tag" "$ver"
  if [ "$APPLY" != "yes" ]; then
    printf 'dry run: nothing was changed. Re-run with --apply.\n'
    return 0
  fi
  # Self-verifying: a repair that does not leave the object clean is not one.
  cmd_check "$tag"
}

main() {
  local sub="${1:-check}" tag="${2:-}"
  APPLY="no"
  if [ "${3:-}" = "--apply" ]; then
    APPLY="yes"
  fi
  if [ -z "$tag" ]; then
    usage
  fi
  WORK=$(mktemp -d)
  trap 'scrub "$WORK"' EXIT
  case "$sub" in
  check) cmd_check "$tag" ;;
  repair) cmd_repair "$tag" ;;
  *) die "unknown subcommand: $sub" ;;
  esac
}

main "$@"
