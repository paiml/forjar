#!/usr/bin/env bash
# forjar#692 -- the complexity limits the pre-commit hook says `gate` enforces.
#
# Usage: scripts/ci/complexity-diff-scope.sh <base-commit>
#
# The pre-commit hook calls itself FEEDBACK and says `ci / gate` enforces the
# thresholds. Until this script, nothing in CI did: a commit made where the hook
# is not installed reached main unchecked, and the next PR to merge main had its
# merge commit refused for a function it never wrote (#683 -> #693).
#
# WHAT IS JUDGED. Exactly what the hook judges, with the same pmat command:
# `pmat analyze complexity --file <f> --diff-scope`, for every Rust file the
# change adds or modifies. A function is refused only when the change TOUCHES it
# and it is over a limit AND worse than it was at the base -- debt that grew.
# Untouched functions never count, so main's code merged into a PR is never
# billed to the PR.
#
# WHICH BASE. The caller passes the commit the change is measured against (the
# changed-class action's `base`: the PR's base tip, or the tip a push moved
# from). The diff is taken from `git merge-base <base> HEAD`, never `HEAD^` or
# `HEAD`: in a merge commit those are the branch, and every function main
# brought in looks new.
#
# HOW. `--diff-scope` reads the staged blob, `HEAD:<path>` and
# `git diff --cached`. A throwaway worktree at HEAD is soft-reset to the merge
# base, so HEAD is the base and the index is the change -- the question the
# hook asks at commit time, asked once for the whole change. The caller's
# checkout is never touched.
#
# THE LIMITS come from pmat.toml `[quality]`, the file `pmat hooks` generates
# the hook from. No second copy lives here.
#
# Exit 0: every changed Rust file judged, none refused.
# Exit 1: at least one touched function grew past a limit.
# Exit 2: UNMEASURED -- no pmat, no limits, or no usable base. Never a pass.
set -euo pipefail

unmeasured() {
    echo "complexity: UNMEASURED -- $*" >&2
    exit 2
}

base="${1:-}"
[ -n "$base" ] || unmeasured "no base commit was given"
git rev-parse --verify --quiet "${base}^{commit}" >/dev/null ||
    unmeasured "base '$base' is not a commit in this clone (is the checkout shallow?)"
command -v pmat >/dev/null || unmeasured "pmat is not on PATH"

root=$(git rev-parse --show-toplevel)
config="$root/pmat.toml"
[ -f "$config" ] || unmeasured "$config is missing"

# One key out of one table. A key outside [quality] must not be read as the
# limit, and a missing key is UNMEASURED rather than pmat's built-in default
# (10/15 for --diff-scope, not the hook's 30/25).
quality_key() {
    awk -v key="$1" '
        /^[[:space:]]*\[/ { in_q = ($0 ~ /^[[:space:]]*\[quality\][[:space:]]*$/); next }
        in_q && $0 ~ "^[[:space:]]*" key "[[:space:]]*=" {
            sub(/^[^=]*=[[:space:]]*/, ""); sub(/[[:space:]]*(#.*)?$/, ""); print; exit
        }' "$config"
}
max_cyc=$(quality_key max_complexity)
max_cog=$(quality_key max_cognitive_complexity)
case "$max_cyc" in '' | *[!0-9]*) unmeasured "pmat.toml [quality] max_complexity is '$max_cyc', not a number" ;; esac
case "$max_cog" in '' | *[!0-9]*) unmeasured "pmat.toml [quality] max_cognitive_complexity is '$max_cog', not a number" ;; esac

merge_base=$(git merge-base "$base" HEAD) ||
    unmeasured "no merge base between $base and HEAD"

# --no-renames: a moved file is judged as added, so a rename cannot carry a
# function past the check as "unchanged".
mapfile -t files < <(git diff --name-only --no-renames --diff-filter=AM "$merge_base" HEAD -- '*.rs')

echo "complexity: limits Cyclomatic $max_cyc, Cognitive $max_cog (from pmat.toml [quality])"
echo "complexity: base $base, merge base $merge_base, HEAD $(git rev-parse HEAD)"
echo "complexity: ${#files[@]} changed Rust file(s) to judge"

if [ "${#files[@]}" -eq 0 ]; then
    echo "complexity: 0 of 0 Rust files refused -- the change touches no Rust source"
    exit 0
fi

scratch=$(mktemp -d)
wt="$scratch/wt"
cleanup() {
    git worktree remove --force "$wt" >/dev/null 2>&1 || true
    rmdir "$scratch" 2>/dev/null || true
}
trap cleanup EXIT
git worktree add --quiet --detach "$wt" HEAD
git -C "$wt" reset --quiet --soft "$merge_base"

refused=0
judged=0
for f in "${files[@]}"; do
    judged=$((judged + 1))
    if out=$(cd "$wt" && NO_COLOR=1 pmat analyze complexity --file "$f" --diff-scope \
        --max-cyclomatic "$max_cyc" --max-cognitive "$max_cog" 2>&1); then
        echo "  ok       $f"
    else
        refused=$((refused + 1))
        echo "  REFUSED  $f"
        printf '%s\n' "$out" | sed 's/^/           /'
    fi
done

echo "complexity: $refused of $judged Rust file(s) refused"
if [ "$refused" -ne 0 ]; then
    echo "::error::$refused file(s) carry a touched function past Cyclomatic $max_cyc / Cognitive $max_cog -- split the function; the limits live in pmat.toml"
    exit 1
fi
