#!/usr/bin/env bash
# forjar#893 — install the control-plane binary ONLY from a clean origin/main
# tip or an exact release tag, with the features BD-3 needs, and prove the
# installed binary is that build.
#
#   scripts/install-lambda.sh --check   refusals only; installs nothing
#   scripts/install-lambda.sh           check, install, verify
#
# INSTALL_LAMBDA_REPO     repo to inspect (default: the checkout holding this script)
# INSTALL_LAMBDA_NO_FETCH set to skip `git fetch origin` (tests)
set -eu

repo="${INSTALL_LAMBDA_REPO:-$(cd "$(dirname "$0")/.." && pwd)}"
mode="${1:-install}"

die() {
    echo "install-lambda: REFUSED: $*" >&2
    exit 1
}

cd "$repo"
[ -d .git ] || [ -f .git ] || die "$repo is not a git checkout"

if [ -z "${INSTALL_LAMBDA_NO_FETCH:-}" ]; then
    git fetch -q origin || die "git fetch origin failed"
fi

if [ -n "$(git status --porcelain)" ]; then
    die "dirty tree in $repo (git status --porcelain is non-empty)"
fi

head="$(git rev-parse HEAD)"
main_tip="$(git rev-parse --verify -q origin/main || true)"
tag="$(git describe --tags --exact-match 2>/dev/null || true)"

if [ "$head" != "$main_tip" ] && [ -z "$tag" ]; then
    die "HEAD ${head} is neither origin/main tip nor an exact tag"
fi

if [ "$mode" = "--check" ]; then
    echo "install-lambda: check ok: HEAD ${head} (${tag:-origin/main tip})"
    exit 0
fi

cargo install --path . --locked --features cli,encryption --force

installed="$(forjar --version)"
short="$(printf '%s' "$head" | cut -c1-8)"
case "$installed" in
    *"$short"*) ;;
    *) die "installed forjar --version is '${installed}', expected sha ${short}" ;;
esac
forjar secrets keygen >/dev/null || die "installed forjar cannot run 'secrets keygen'"
echo "install-lambda: installed ${installed}; secrets keygen ok"
