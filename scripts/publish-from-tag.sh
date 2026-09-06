#!/usr/bin/env bash
# publish-from-tag.sh — publish the workspace to crates.io from a DETACHED
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
# A dirty detached worktree (tracked drift, or untracked files other than
# build litter under target/) refuses with exit 3.
#
# Offline test hook (tests/falsification_publish_from_tag.rs drives this): a
# shim `cargo` placed first on PATH answers `metadata` and `search`, and logs
# every argv line plus its cwd, so the test can assert the exact sequence and
# environment of every cargo invocation without ever calling crates.io.
#
# NOTE ON --detach: removing it from `git worktree add` makes checking out a
# tag ref fail outright (a tag is not a branch you can attach a worktree to),
# so this script cannot silently regress to a non-detached, dirtiable
# worktree — that is the mutation guard for this whole file.
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

WT="$(mktemp -d /tmp/forjar-publish-XXXXXX)"

cleanup() {
  printf '+ git worktree remove --force %s\n' "$WT" >&2
  git worktree remove --force "$WT" >/dev/null 2>&1 || true
}
trap cleanup EXIT

run git worktree add --detach "$WT" "$TAG"

# Tracked drift or stray untracked files in the checked-out tag are a defect
# in this script (or the tag), not something to paper over. `target/` is
# build litter, exempt.
dirty="$(cd "$WT" && git status --porcelain --ignored | grep -v '^!! target/' || true)"
if [[ -n "$dirty" ]]; then
  printf '%s\n' "$dirty" >&2
  exit 3
fi

cd "$WT" || exit 2

# ------------------------------------------------------ topological order --

metadata_file="$WT/.publish-metadata.json"
pkgs_file="$WT/.publish-pkgs.tsv"
order_file="$WT/.publish-order.tsv"

run cargo metadata --no-deps --format-version=1 >"$metadata_file"

# Held in a variable, referenced by name, rather than written inline on the
# redirected line below: bashrs's shell-aware parser scans literal `|`
# characters wherever they occur, including inside a quoted jq program, and
# misreads them as real shell pipes preceding the `>` redirect.
pkgs_filter='.packages[] | [.name, .version, (.publish|tostring), ([.dependencies[] | select(.path != null) | .name] | join(","))] | @tsv'
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

# `cargo search NAME --limit 1` prints `name = "version"    # description`,
# possibly among other near-matches; grep -F anchors the exact assignment
# rather than trusting a prefix/partial match on the name.
already_published() {
  local name="$1" version="$2"
  run cargo search "$name" --limit 1 | grep -qF "${name} = \"${version}\""
}

# Bounded backoff over a fixed sequence of delays, never a single fixed
# sleep: the index is eventually consistent after a publish, and a
# dependent crate resolving against it too early is a spurious failure, not
# a real one.
poll_index() {
  local name="$1" version="$2" delay
  for delay in 1 2 4 8 8 8 8 8; do
    already_published "$name" "$version" && return 0
    sleep "$delay"
  done
  die "refusing: $name $version never appeared on the index after publishing"
}

last_name="$(tail -n1 "$order_file" | cut -f1)"
while IFS=$'\t' read -r name version; do
  if already_published "$name" "$version"; then
    printf 'skip: %s %s is already on the index\n' "$name" "$version" >&2
    continue
  fi

  run env -u CARGO_REGISTRY_TOKEN cargo publish --dry-run --locked -p "$name"

  if [[ "${DRY_RUN:-0}" = "1" ]]; then
    continue
  fi

  run env -u CARGO_REGISTRY_TOKEN cargo publish --locked -p "$name"

  # Between a dependency and its dependents, wait for the index to catch up.
  # Skip it after the last crate — nothing downstream depends on it here.
  if [[ "$name" != "$last_name" ]]; then
    poll_index "$name" "$version"
  fi
done <"$order_file"
