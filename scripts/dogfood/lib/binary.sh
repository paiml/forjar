#!/usr/bin/env bash
# Resolve THE binary under test, and prove it is the tree's binary.
#
# Prints one absolute path on stdout. Everything else goes to stderr, so a
# caller can write `BIN="$(bash lib/binary.sh)"` and get a path or a failure,
# never a path with a build log stuck to the front of it.
#
# Never `which forjar`. A dogfood run that measures whatever `forjar` PATH
# happens to resolve is measuring the last release the operator installed, and
# it will pass on the day the tree is broken. Two things are asserted here:
#
#   1. the path is under `cargo metadata`'s target_directory — which on this
#      machine is NOT `<repo>/target`, it is shared across every worktree, so
#      hardcoding `./target/release/forjar` silently reads a stale binary or
#      none at all;
#   2. `--version` equals the version in Cargo.toml. A shared target directory
#      is exactly how a sibling worktree's binary ends up standing in for this
#      one.
#
# FORJAR_DOGFOOD_BIN overrides both, for `make dogfood-published` — which
# deliberately measures an INSTALLED artifact rather than this tree. The
# version check is then made against that binary's own claim, not Cargo.toml.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

want="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
if [ -z "$want" ]; then
  echo "cannot read version from Cargo.toml" >&2
  exit 1
fi

if [ -n "${FORJAR_DOGFOOD_BIN:-}" ]; then
  bin="$FORJAR_DOGFOOD_BIN"
  if [ ! -x "$bin" ]; then
    echo "FORJAR_DOGFOOD_BIN=${bin} is not executable" >&2
    exit 1
  fi
  printf '%s\n' "$bin"
  exit 0
fi

cargo build --release >&2

target="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
bin="${target}/release/forjar"
if [ ! -x "$bin" ]; then
  echo "no release binary at ${bin} after 'cargo build --release'" >&2
  exit 1
fi

got="$("$bin" --version | awk '{print $2}')"
if [ "$got" != "$want" ]; then
  echo "binary at ${bin} reports ${got}, Cargo.toml says ${want} — the shared target directory is holding another worktree's build, so every gate downstream would measure the wrong tree" >&2
  exit 1
fi

printf '%s\n' "$bin"
