#!/usr/bin/env bash
# Falsifier for scripts/assemble-changelog.sh (BUILD-FIRST R3).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"; fx="$here/changelog_fragments_fixture"
asm="$here/../scripts/assemble-changelog.sh"
w="$(mktemp -d)"; trap 'rm -rf "${w:?}"' EXIT
n=0; ok() { n=$((n+1)); echo "ok $n - $1"; }; bad() { echo "FAIL: $1" >&2; exit 1; }
fresh() { rm -rf "${w:?}/r"; cp -r "$fx/base" "$w/r"; }
rmy="$w/r/docs/roadmaps/roadmap.yaml"

fresh; "$asm" --root "$w/r" >/dev/null
diff -u "$fx/expected_CHANGELOG.md" "$w/r/CHANGELOG.md" || bad "CHANGELOG differs"
ok "fragment assembles to the expected CHANGELOG"
grep -q '^- id: PMAT-1$' "$rmy" && [ "$(grep -c '^- id:' "$rmy")" = 2 ] || bad "row not appended"
ok "roadmap row appended"
fresh; printf '**Entry two (PMAT-2).**\nBody two.\n' > "$w/r/changelog.d/PMAT-2.md"; "$asm" --root "$w/r" >/dev/null
[ "$(grep -n 'Entry one\|Entry two\|^## \[1.0.0\]' "$w/r/CHANGELOG.md" | cut -d: -f1 | paste -sd,)" = "5,8,11" ] || bad "order wrong"
ok "second fragment ordered by name, before the old release"
fresh; cp "$w/r/changelog.d/PMAT-1.md" "$w/r/changelog.d/PMAT-1-again.md"
if "$asm" --check --root "$w/r" 2>"$w/e"; then bad "dup changelog id accepted"; fi
grep -q 'duplicate changelog id PMAT-1' "$w/e" || bad "dup message missing"
ok "duplicate changelog id detected"
fresh; mv "$w/r/docs/roadmaps/fragments/PMAT-1.yaml" "$w/r/docs/roadmaps/fragments/PMAT-9.yaml"
if "$asm" --check --root "$w/r" 2>/dev/null; then bad "name/id mismatch accepted"; fi
ok "roadmap name/id mismatch detected"
fresh; printf -- '- id: PMAT-0\n  status: completed\n' > "$w/r/docs/roadmaps/fragments/PMAT-0.yaml"; "$asm" --root "$w/r" >/dev/null
[ "$(grep -c '^- id: PMAT-0$' "$rmy")" = 1 ] && grep -q 'completed' "$rmy" && ! grep -q 'planned' <(sed -n '/PMAT-0/,/PMAT-1/p' "$rmy" | head -2) || bad "replace failed"
ok "existing roadmap id replaced in place"
fresh; "$asm" --consume --root "$w/r" >/dev/null; [ ! -e "$w/r/changelog.d/PMAT-1.md" ] || bad "not consumed"
ok "--consume deletes fragments"
echo "1..$n"
