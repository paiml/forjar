#!/usr/bin/env bash
# Install forjar's tracked git hooks into .git/hooks/.
#
# Hooks are not versioned by git, so a contributor who never runs this is never
# gated -- a limitation docs/specifications/quorum-spec.md names explicitly rather
# than hides. CI must therefore mirror the same checks (.github/workflows/quorum.yml);
# this only buys fast local feedback.
set -euo pipefail

# WORK FROM THE REPO ROOT WITH RELATIVE PATHS.
#
# Not cosmetic: bashrs SEC010 rejects `mkdir`/`cp` on a path that starts with `/`,
# and `git rev-parse --show-toplevel` is always absolute, so an absolute-path
# installer can never lint clean. forjar's own `validate_before_exec` refuses a
# script on a bashrs ERROR, so "it is only a linter" is not true in this repo --
# an unlintable helper is one a forjar `task` could never run.
cd "$(git rev-parse --show-toplevel)" || exit 1

src="scripts/hooks"
dst=".git/hooks"

[ -d "$src" ] || { echo "✗ no $src"; exit 1; }
[ -d ".git" ] || { echo "✗ .git is not a directory here"; exit 1; }

mkdir -p "$dst"

for hook in "$src"/*; do
    [ -f "$hook" ] || continue
    name="$(basename "$hook")"
    # A hook name must be a bare filename. Anything with a separator or a dot-dot
    # is a traversal attempt, not a hook.
    case "$name" in
        */*|*..*|"") echo "✗ refusing hook name '$name'"; exit 1 ;;
    esac

    target="$dst/$name"
    # NEVER CLOBBER A DIFFERING LOCAL HOOK. Refuse and let the operator decide,
    # rather than quietly copying their file to a .bak the way an earlier draft
    # did: a script that silently duplicates a developer's customisations is how
    # `.git/hooks` fills with stale copies nobody dares delete. (It also could
    # not lint clean -- bashrs SEC010 on the backup `cp` -- and in this repo an
    # unlintable script is one forjar's own I8 gate would refuse to run.)
    if [ -f "$target" ] && ! cmp -s "$hook" "$target"; then
        echo "✗ $target exists and differs from the tracked hook."
        echo "  Move or delete it, then re-run. Not overwriting it for you."
        exit 1
    fi
    cp "$hook" "$target"
    chmod +x "$target"
    echo "✓ installed $name"
done
