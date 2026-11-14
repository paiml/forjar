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

# Review b529e13 L2: a whitespace-only changelog fragment is empty; --check and
# --consume both refuse it and the fragment survives.
for body in '\n' '  \n\t\n'; do
  fresh; printf "$body" > "$w/r/changelog.d/PMAT-3.md"
  if "$asm" --check --root "$w/r" 2>"$w/e"; then bad "whitespace-only fragment passed --check"; fi
  grep -q 'PMAT-3.md is empty' "$w/e" || bad "whitespace-only message missing"
  if "$asm" --consume --root "$w/r" 2>/dev/null; then bad "whitespace-only fragment passed --consume"; fi
  [ -e "$w/r/changelog.d/PMAT-3.md" ] && [ -e "$w/r/changelog.d/PMAT-1.md" ] || bad "--consume deleted fragments on a refused run"
  diff -q "$fx/base/CHANGELOG.md" "$w/r/CHANGELOG.md" >/dev/null || bad "refused run wrote CHANGELOG.md"
done
ok "whitespace-only changelog fragment refused by --check and --consume, nothing deleted"

# Review b529e13 L1: the dogfood registry (Gate A/T, release-goal) is roadmap.yaml
# WITH its fragments: a fragment row is a row, and replaces a same-id row.
reg() { # $1 repo, $2 ref -> "<ids>|<statuses>" as Gate A/T read them
  ( cd "$1"; fail() { echo "FAIL: $*" >&2; exit 1; }; GH=:
    # shellcheck source=scripts/dogfood/lib/window.sh
    . "$here/../scripts/dogfood/lib/window.sh"
    DOGFOOD_ROADMAP_REF="$2" dogfood_roadmap_rows
    printf '%s|%s\n' "$(paste -sd, <<<"$DOGFOOD_ROW_IDS")" "$(paste -sd, <<<"$DOGFOOD_ROW_STATUSES")" )
}
want="PMAT-0,PMAT-1,PMAT-7|PMAT-0 completed,PMAT-1 planned,PMAT-7 inprogress"
fresh; printf -- '- id: PMAT-7\n  status: inprogress\n' > "$w/r/docs/roadmaps/fragments/PMAT-7.yaml"
printf -- '- id: PMAT-0\n  status: completed\n' > "$w/r/docs/roadmaps/fragments/PMAT-0.yaml"
got="$(reg "$w/r" worktree)"; [ "$got" = "$want" ] || bad "worktree registry: $got"
ok "registry (worktree) counts fragment rows and lets them replace same-id rows"
git -C "$w/r" init -q && git -C "$w/r" add -A && git -C "$w/r" -c user.name=t -c user.email=t@t commit -qm t
rm "${w:?}/r/docs/roadmaps/fragments/PMAT-7.yaml"   # the working tree is not HEAD
got="$(reg "$w/r" HEAD)"; [ "$got" = "$want" ] || bad "HEAD registry: $got"
ok "registry (HEAD) reads the committed fragments, not the working tree"
fresh; rm -rf "${w:?}/r/docs/roadmaps/fragments"
got="$(reg "$w/r" worktree)"; [ "$got" = "PMAT-0|PMAT-0 planned" ] || bad "no-fragments registry: $got"
ok "registry without a fragments directory is roadmap.yaml alone"

# This PR dogfoods its own scheme: PMAT-1008 is a fragment, NOT a roadmap.yaml
# edit, and the registry Gate A reads still finds it.
real="$here/.."
[ -f "$real/docs/roadmaps/fragments/PMAT-1008.yaml" ] || bad "PMAT-1008 roadmap fragment missing"
[ -f "$real/changelog.d/PMAT-1008.md" ] || bad "PMAT-1008 changelog fragment missing"
! grep -q '^- id: PMAT-1008$' "$real/docs/roadmaps/roadmap.yaml" || bad "roadmap.yaml carries PMAT-1008 directly; it must come from its fragment"
grep -qE '(^|,)PMAT-1008(,|\|)' <<<"$(reg "$real" worktree)" || bad "registry does not list PMAT-1008"
dups="$(sed -nE 's/^- id: *(.*[^ ]) *$/\1/p' "$real/docs/roadmaps/roadmap.yaml" | tr '[:lower:]' '[:upper:]' | sort | uniq -d)"
[ -z "$dups" ] || bad "duplicate ids in roadmap.yaml: $dups"
"$asm" --check --root "$real" >/dev/null || bad "this repo's fragments fail --check"
ok "PMAT-1008 comes from its fragment, the registry lists it, fragments check"
# The CI wiring (dogfood-guards runs this file, no soft-fail, gate needs it) is
# parsed as YAML in tests/falsification_changelog_fragments_run_in_ci.rs.
echo "1..$n"
