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

# Review fix 3: a run without --consume, repeated, changes nothing the second time.
fresh; "$asm" --root "$w/r" >/dev/null; "$asm" --root "$w/r" >/dev/null
diff -u "$fx/expected_CHANGELOG.md" "$w/r/CHANGELOG.md" || bad "second run duplicated the changelog entry"
[ "$(grep -c '^- id:' "$rmy")" = 2 ] && [ "$(grep -c '^- id: PMAT-1$' "$rmy")" = 1 ] || bad "second run duplicated the roadmap row"
ok "repeated run without --consume is idempotent"
fresh; printf '**Entry two (PMAT-2).**\nBody two.\n' > "$w/r/changelog.d/PMAT-2.md"
"$asm" --root "$w/r" >/dev/null; cp "$w/r/CHANGELOG.md" "$w/once"; "$asm" --root "$w/r" >/dev/null
diff -u "$w/once" "$w/r/CHANGELOG.md" || bad "second run with two fragments changed CHANGELOG"
[ "$(grep -c 'Entry two' "$w/r/CHANGELOG.md")" = 1 ] || bad "Entry two duplicated"
ok "idempotent with several fragments"

# Review fix 4: rewritten files are 644, not mktemp's 600 (umask 077 is the worst case).
fresh; (umask 077; "$asm" --root "$w/r" >/dev/null)
[ "$(stat -c %a "$w/r/CHANGELOG.md")" = 644 ] || bad "CHANGELOG.md mode $(stat -c %a "$w/r/CHANGELOG.md")"
[ "$(stat -c %a "$rmy")" = 644 ] || bad "roadmap.yaml mode $(stat -c %a "$rmy")"
ok "assembled files are mode 644"

# Review fix 5: ids compare case-insensitively.
fresh; cp "$w/r/changelog.d/PMAT-1.md" "$w/r/changelog.d/pmat-1-again.md"
if "$asm" --check --root "$w/r" 2>"$w/e"; then bad "lowercase dup changelog id accepted"; fi
grep -q 'duplicate changelog id PMAT-1' "$w/e" || bad "lowercase dup changelog message missing"
ok "lowercase duplicate changelog id detected"
fresh; printf -- '- id: pmat-1\n  status: done\n' > "$w/r/docs/roadmaps/fragments/pmat-1.yaml"
if "$asm" --check --root "$w/r" 2>"$w/e"; then bad "lowercase dup roadmap id accepted"; fi
grep -q 'duplicate roadmap id PMAT-1' "$w/e" || bad "lowercase dup roadmap message missing"
ok "lowercase duplicate roadmap id detected"
fresh; rm "$w/r/docs/roadmaps/fragments/PMAT-1.yaml"
printf -- '- id: pmat-0\n  status: completed\n' > "$w/r/docs/roadmaps/fragments/pmat-0.yaml"; "$asm" --root "$w/r" >/dev/null
[ "$(grep -ci '^- id: pmat-0$' "$rmy")" = 1 ] || bad "lowercase fragment did not replace the PMAT-0 row"
ok "lowercase roadmap fragment replaces the existing row"

# Review fix 2: this repo's own roadmap carries the PMAT-1008 row, ids unique.
real="$here/.."
[ "$(grep -c '^- id: PMAT-1008$' "$real/docs/roadmaps/roadmap.yaml")" = 1 ] || bad "roadmap.yaml lacks exactly one PMAT-1008 row"
dups="$(sed -nE 's/^- id: *(.*[^ ]) *$/\1/p' "$real/docs/roadmaps/roadmap.yaml" | tr '[:lower:]' '[:upper:]' | sort | uniq -d)"
[ -z "$dups" ] || bad "duplicate ids in roadmap.yaml: $dups"
"$asm" --check --root "$real" >/dev/null || bad "this repo's fragments fail --check"
ok "repo roadmap has the PMAT-1008 row, unique ids, fragments check"

# Review fix 1: CI runs this file, in the required dogfood-guards job, with no soft-fail.
job="$(awk '/^  dogfood-guards:/{on=1;next} on&&/^  [a-z][a-z_-]*:$/{exit} on' "$real/.github/workflows/ci.yml")"
grep -q 'run: bash tests/changelog-fragments.sh$' <<<"$job" || bad "dogfood-guards does not run tests/changelog-fragments.sh"
! grep -q 'continue-on-error' <<<"$job" || bad "dogfood-guards has continue-on-error"
grep -q 'dogfood-guards' <(awk '/^  gate:/{on=1} on&&/needs:/{print;exit}' "$real/.github/workflows/ci.yml") || bad "gate does not need dogfood-guards"
ok "CI dogfood-guards runs this falsifier"
echo "1..$n"
