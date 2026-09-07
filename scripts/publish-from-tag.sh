#!/usr/bin/env bash
# publish-from-tag.sh — publish the workspace to crates.io from a detached
# worktree of a release tag. Never from a possibly-dirty working tree, and
# never with `--allow-dirty`.
#
# WHY (PMAT-165): publishing from the ambient working tree lets untracked
# state — agent memory, other worktrees, scratch files — dirty the tree that
# `cargo publish` sees, and `--allow-dirty` has been the standing temptation
# to paper over that. The sanctioned path checks the TAG out into its own
# detached worktree, asserts it is clean there, and publishes crate by crate
# from that worktree — never with a registry token surviving in the
# environment beyond what cargo's own credentials file supplies.
#
# Usage:
#   publish-from-tag.sh vX.Y.Z
#   DRY_RUN=1 publish-from-tag.sh vX.Y.Z   # dry-run every crate, publish none
#
# Refusals (exit 2, before any cargo call):
#   * no TAG, or TAG not shaped like vX.Y.Z
#   * TAG does not exist
#   * TAG's commit is not on origin/main
#   * TAG's Cargo.toml [package] version disagrees with the tag name
#   * a published crate never appears on the index within the bounded poll
# A dirty worktree refuses with exit 3.
#
# Offline test hooks (tests/falsification_publish_from_tag.rs drives these): a
# shim `cargo` placed first on PATH answers `metadata`, `search` and `info`,
# and logs every argv line plus its cwd, the live worktree count and a
# directory listing, so the suite can assert the exact sequence and
# environment of every cargo invocation without ever calling crates.io.
# `PUBLISH_POLL_DELAYS` overrides the index backoff schedule so the bounded
# poll can be exercised without real sleeping. `TMPDIR` scopes the worktree
# and scratch directories, so a crashed run leaks nothing into a shared /tmp.
#
# MUTATION GUARD (PMAT-187). It is NOT `--detach`: `git worktree add` detaches
# at a tag whether or not the flag is passed, so removing it changes nothing
# any test can see. The guard is that cargo runs inside a WORKTREE OF THIS
# REPOSITORY. Replacing `git worktree add` below with `git clone` or `cp -r`
# would still give cargo a clean, tag-shaped tree — and would still pass a
# naive "cwd is not the repo" assertion — but the shim's
# `git rev-parse --git-common-dir` would then resolve to the copy's own .git
# and `git worktree list` would show one worktree instead of two, which is
# exactly what case (c) of the falsification suite pins.
set -euo pipefail

die() {
  printf '%s\n' "$*" >&2
  exit 2
}

usage() {
  printf 'usage: publish-from-tag.sh vX.Y.Z\n' >&2
  printf '       DRY_RUN=1 publish-from-tag.sh vX.Y.Z\n' >&2
  exit 2
}

# Every command this script runs is echoed to stderr before it runs, so a
# human — or a test log — can see the exact sequence without guessing.
run() {
  printf '+ %s\n' "$*" >&2
  "$@"
}

# ------------------------------------------------------------ TAG refusals --

TAG="${1:-}"
[[ -n "$TAG" ]] || usage

if ! [[ "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  die "refusing: '$TAG' is not shaped like vX.Y.Z"
fi

git rev-parse -q --verify "refs/tags/$TAG" >/dev/null ||
  die "refusing: tag '$TAG' does not exist"

run git fetch -q origin main
if ! git merge-base --is-ancestor "$TAG" origin/main; then
  die "refusing: '$TAG' is not an ancestor of origin/main"
fi

# The [package] version in Cargo.toml AT THE TAG, never from the ambient
# working tree — the two can disagree if main has moved since the tag.
manifest_version() {
  sed -n '/^\[package\]/,/^\[/{/^version[[:space:]]*=/{s/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p}}'
}

tag_version="${TAG#v}"
cargo_toml_version="$(git show "$TAG:Cargo.toml" | manifest_version | head -n1)"
[[ -n "$cargo_toml_version" ]] ||
  die "refusing: no [package] version found in $TAG:Cargo.toml"
[[ "$cargo_toml_version" = "$tag_version" ]] ||
  die "refusing: Cargo.toml version '$cargo_toml_version' at $TAG does not match tag version '$tag_version'"

# --------------------------------------------------------- detached worktree --

# Two directories, both under $TMPDIR and both removed by the same trap. WT is
# the tag checkout cargo publishes from; SCRATCH holds this script's own
# metadata/order files. They are separate on purpose (PMAT-184): a scratch
# file written inside WT is an untracked file in the tree `cargo publish`
# packages, and cargo then refuses the crate unless it is handed
# `--allow-dirty` — the one flag this whole pattern exists to avoid.
WT="$(mktemp -d "${TMPDIR:-/tmp}/forjar-publish-XXXXXX")"
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/forjar-publish-scratch-XXXXXX")"

cleanup() {
  printf '+ git worktree remove --force %s\n' "$WT" >&2
  git worktree remove --force "$WT" >/dev/null 2>&1 || true
  # `:?` is not decoration: an unset or empty variable here would make
  # this `rm -rf` a command about /, and this runs from a trap.
  rm -rf "${SCRATCH:?}" "${WT:?}"
}
trap cleanup EXIT

run git worktree add --detach "$WT" "$TAG"

# A freshly checked-out tag has nothing untracked and nothing ignored — not
# even target/, which cargo has not had a chance to create yet. Anything at
# all here is a defect in this script or in the tag, so the check is strict:
# `--ignored` included, no exemptions.
assert_worktree_pristine() {
  local dirty
  dirty="$(git -C "$WT" status --porcelain --ignored)"
  if [[ -n "$dirty" ]]; then
    printf 'refusing: the tag worktree is not pristine:\n%s\n' "$dirty" >&2
    exit 3
  fi
}

# Re-asserted immediately before every `cargo publish`, because cleanliness at
# checkout time says nothing about cleanliness after this script (or a
# previous crate's dry-run) has run. This one mirrors what cargo publish
# itself refuses — gitignored build litter under target/ is cargo's own doing
# and does not count, untracked files that are NOT ignored do.
assert_worktree_publishable() {
  local dirty
  dirty="$(git -C "$WT" status --porcelain)"
  if [[ -n "$dirty" ]]; then
    printf 'refusing: the worktree is dirty before publishing %s:\n%s\n' "$1" "$dirty" >&2
    exit 3
  fi
}

assert_worktree_pristine

cd "$WT" || exit 2

# ------------------------------------------------------ topological order --

metadata_file="$SCRATCH/metadata.json"
pkgs_file="$SCRATCH/pkgs.tsv"
order_file="$SCRATCH/order.tsv"

run cargo metadata --no-deps --format-version=1 >"$metadata_file"

# Held in a variable, referenced by name, rather than written inline on the
# redirected line below: bashrs's shell-aware parser scans literal `|`
# characters wherever they occur, including inside a quoted jq program, and
# misreads them as real shell pipes preceding the `>` redirect.
#
# Only NORMAL (kind == null) and BUILD dependencies are publish-order edges
# (PMAT-186). A dev-dependency is resolved for `cargo test`, never for
# packaging, so counting it turns the perfectly legal "crate A depends on
# crate B, B dev-depends back on A" shape into a phantom cycle that refuses a
# release that cargo itself would publish happily.
pkgs_filter='.packages[] | [.name, .version, (.publish|tostring), ([.dependencies[] | select(.path != null) | select(.kind == null or .kind == "build") | .name] | join(","))] | @tsv'
jq -r "$pkgs_filter" "$metadata_file" >"$pkgs_file"

# Kahn's topological sort over path-dependency edges: a crate is emitted only
# once every local crate it path-depends on has already been emitted. A
# dependency name this file has never heard of (not a workspace member) is
# not an ordering constraint and is ignored. Crates with `publish = false`
# are excluded up front. One line per program: bashrs's parser misreads the
# brackets of a pretty-printed multi-line jq/awk program as unbalanced shell
# test syntax, so the whole filter stays on one line.
topo_sort() {
  awk -F'\t' '{ name[NR]=$1; version[NR]=$2; publish[NR]=$3; deps[NR]=$4; n=NR } END { remaining=n; for (i=1; i<=n; i++) { if (publish[i] == "[]") { done[i]=1; remaining-- } }; while (remaining > 0) { progressed=0; for (i=1; i<=n; i++) { if (done[i]) continue; ready=1; if (deps[i] != "") { m=split(deps[i], darr, ","); for (j=1; j<=m; j++) { dep=darr[j]; for (k=1; k<=n; k++) { if (name[k] == dep && !done[k]) ready=0 } } }; if (ready) { print name[i] "\t" version[i]; done[i]=1; remaining--; progressed=1 } }; if (!progressed) { print "cycle" > "/dev/stderr"; exit 1 } } }' "$1"
}

topo_sort "$pkgs_file" >"$order_file" ||
  die "refusing: dependency cycle among workspace crates, cannot order publishing"

# --------------------------------------------------------------- publishing --

# `cargo info NAME@VERSION` asks the registry about ONE exact version and
# answers with its exit code (PMAT-185). The `cargo search` it replaces
# reports only the newest version of a prefix-matched name, so it could never
# see an older version and would happily accept another crate's line as proof.
CARGO_INFO_AVAILABLE=""
cargo_info_available() {
  if [[ -z "$CARGO_INFO_AVAILABLE" ]]; then
    if cargo info --help >/dev/null 2>&1; then
      CARGO_INFO_AVAILABLE="yes"
    else
      CARGO_INFO_AVAILABLE="no"
    fi
  fi
  [[ "$CARGO_INFO_AVAILABLE" = "yes" ]]
}

# Fallback for a cargo too old to have `cargo info`: parse `cargo search`, but
# anchor the match at column 1 of the line and require the exact name AND the
# exact version. `index(line, needle) == 1` is an anchored literal match with
# no regex metacharacters to escape — `grep -F` alone would accept
# `other-demo = "0.0.1"` as evidence about `demo`.
search_says_published() {
  local name="$1" version="$2"
  run cargo search "$name" --limit 1 |
    awk -v n="$name" -v v="$version" 'index($0, n " = \"" v "\"") == 1 { found = 1 } END { exit !found }'
}

already_published() {
  local name="$1" version="$2"
  if cargo_info_available; then
    printf '+ cargo info %s@%s --registry crates-io\n' "$name" "$version" >&2
    cargo info "$name@$version" --registry crates-io >/dev/null 2>&1
    return
  fi
  search_says_published "$name" "$version"
}

# Bounded backoff over a delay sequence, never a single fixed sleep: the index
# is eventually consistent after a publish, and a dependent crate resolving
# against it too early is a spurious failure, not a real one. When the bound
# is exhausted the run refuses (exit 2) naming the crate and version, rather
# than publishing a dependent against an index that has not caught up.
poll_index() {
  local name="$1" version="$2" delay
  local -a delays=()
  IFS=' ' read -r -a delays <<<"${PUBLISH_POLL_DELAYS:-1 2 4 8 8 8 8 8}"
  for delay in "${delays[@]}"; do
    already_published "$name" "$version" && return 0
    sleep "$delay"
  done
  already_published "$name" "$version" && return 0
  die "refusing: $name $version never appeared on the index after publishing (bounded poll exhausted)"
}

last_name="$(tail -n1 "$order_file" | cut -f1)"
while IFS=$'\t' read -r name version; do
  if already_published "$name" "$version"; then
    printf 'skip: %s %s is already on the index\n' "$name" "$version" >&2
    continue
  fi

  assert_worktree_publishable "$name (dry run)"
  run env -u CARGO_REGISTRY_TOKEN cargo publish --dry-run --locked -p "$name"

  if [[ "${DRY_RUN:-0}" = "1" ]]; then
    continue
  fi

  assert_worktree_publishable "$name"
  run env -u CARGO_REGISTRY_TOKEN cargo publish --locked -p "$name"

  # Between a dependency and its dependents, wait for the index to catch up.
  # Skip it after the last crate — nothing downstream depends on it here.
  if [[ "$name" != "$last_name" ]]; then
    poll_index "$name" "$version"
  fi
done <"$order_file"
