#!/usr/bin/env bash
# `make pr-light` and its pre-push block (PMAT-855; operator C2 item 2), proven against a fixture clone with a tiny real crate --
# never against this repo's ci.yml or tests (that would build forjar inside the test).
#
#   CONTRACT  a commit with no `Pmat-Ticket:` trailer makes pr-light RED; a clean branch is GREEN; a planted tree-guard
#             violation is RED; no origin/main is RED; no `# pr-light` marker is RED; a marker that is not a plain
#             `cargo test --locked --test NAME` is RED. The trailer leg judges the head sha it is given.
#   HOOK      the pr-light block of scripts/hooks/pre-push (the real text, cut before the slow checks) exits 1 for a pushed
#             trailer-less sha, 0 for a clean one and for a deletion.
#   INSTALL   scripts/install-hooks.sh replaces an older copy of the forjar hook and still refuses a foreign one.
#   MUTANTS   pr-light.sh with (a) `exit 0` first, (b) the trailer leg removed, (c) the guard leg removed, and (d) a hook with no
#             pr-light block: the contract/hook checks must FAIL against each.
set -uo pipefail
repo=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "${work:?}"' EXIT
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t CARGO_TARGET_DIR="$work/target"
total_fail=0
fails=0
expect() { if [ "$2" = "$3" ]; then printf 'ok   %s\n' "$1"; else printf 'FAIL %s: want %s got %s\n' "$1" "$2" "$3"; fails=$((fails + 1)); fi; }
cm() { git -C "$1" -c core.hooksPath=/dev/null commit -q --allow-empty -m "$2"; }
tolow() { if [ "$1" -ne 0 ]; then echo 1; else echo 0; fi; }

# mkfx <dir> <pr-light.sh to install>
mkfx() {
  local d=$1 prl=$2
  git init -q --bare -b main "$d.git"
  git clone -q "$d.git" "$d" 2>/dev/null
  git -C "$d" checkout -q -b main
  mkdir -p "$d/scripts" "$d/src" "$d/tests" "$d/.github/workflows"
  cp "$repo/scripts/ticket-cite-lint.sh" "$d/scripts/"
  cp "$prl" "$d/scripts/pr-light.sh"
  chmod +x "$d"/scripts/*.sh
  printf '[package]\nname = "fx"\nversion = "0.0.0"\nedition = "2021"\n[workspace]\n' > "$d/Cargo.toml"
  : > "$d/src/lib.rs"
  printf '#[test]\nfn planted_absent() {\n    assert!(!std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/PLANTED")).exists());\n}\n' > "$d/tests/guard_a.rs"
  cat > "$d/.github/workflows/ci.yml" <<'Y'
name: fx
jobs:
  examples-validate:
    steps:
      - name: Guard A
        # pr-light  (marker)
        run: cargo test --locked --test guard_a
      - name: Not a guard
        run: echo unmarked
Y
  ( cd "$d" && cargo generate-lockfile --offline -q 2>/dev/null )
  git -C "$d" add -A
  cm "$d" "$(printf 'fx base\n\nPmat-Ticket: PMAT-1')"
  git -C "$d" -c core.hooksPath=/dev/null push -q origin main 2>/dev/null
  git -C "$d" fetch -q origin
  git -C "$d" checkout -q -b work
}
prl() { ( cd "$1" && ./scripts/pr-light.sh "${2:-HEAD}" >"$1/.out" 2>&1 ) || return 1; }

contract() {
  local d=$1 rc
  cm "$d" "$(printf 'feat: ok\n\nPmat-Ticket: PMAT-2\n\nCo-Authored-By: t <t@t>')"
  prl "$d"; expect "clean branch is GREEN" 0 "$?"
  cm "$d" "feat: no trailer"
  prl "$d"; expect "a commit without a Pmat-Ticket trailer is RED" 1 "$?"
  bad=$(git -C "$d" rev-parse HEAD)
  git -C "$d" reset -q --hard HEAD~1
  prl "$d" "$bad"; expect "the trailer leg judges the head sha it is given" 1 "$?"
  touch "$d/PLANTED"
  prl "$d"; expect "a planted tree-guard violation is RED" 1 "$?"
  rm -f "$d/PLANTED"
  prl "$d"; expect "green again once the violation is gone" 0 "$?"
  cp "$d/.github/workflows/ci.yml" "$d/ci.bak"
  sed -i 's/# pr-light/# nothing/' "$d/.github/workflows/ci.yml"
  prl "$d"; expect "no '# pr-light' marker is RED (0 guards is not a pass)" 1 "$?"
  cp "$d/ci.bak" "$d/.github/workflows/ci.yml"
  sed -i 's|run: cargo test --locked --test guard_a|run: true|' "$d/.github/workflows/ci.yml"
  prl "$d"; expect "a marker that is not a plain cargo test is RED" 1 "$?"
  cp "$d/ci.bak" "$d/.github/workflows/ci.yml"
  git -C "$d" update-ref -d refs/remotes/origin/main
  prl "$d"; expect "no origin/main is RED (unmeasured is not a pass)" 1 "$?"
  git -C "$d" fetch -q origin
}

# hookcheck <dir> <hook file>: the pr-light block of the hook, cut before the slow checks
hookcheck() {
  local d=$1 hk=$2 good bad rc zero=0000000000000000000000000000000000000000
  head -n "$(( $(grep -n '^run_check "Format check"' "$hk" | cut -d: -f1) - 1 ))" "$hk" > "$d/.hk.sh" 2>/dev/null
  cm "$d" "$(printf 'feat: ok\n\nPmat-Ticket: PMAT-2')"; good=$(git -C "$d" rev-parse HEAD)
  cm "$d" "feat: no trailer"; bad=$(git -C "$d" rev-parse HEAD)
  ( cd "$d" && printf 'refs/heads/work %s refs/heads/work %s\n' "$bad" "$zero" | bash .hk.sh >"$d/.out" 2>&1 ); rc=$?
  expect "hook refuses a pushed trailer-less sha" 1 "$(tolow "$rc")"
  ( cd "$d" && printf 'refs/heads/work %s refs/heads/work %s\n' "$good" "$zero" | bash .hk.sh >"$d/.out" 2>&1 ); rc=$?
  expect "hook lets a clean sha through" 0 "$rc"
  ( cd "$d" && printf '(delete) %s refs/heads/work %s\n' "$zero" "$good" | bash .hk.sh >"$d/.out" 2>&1 ); rc=$?
  expect "hook does not judge a branch deletion" 0 "$rc"
  git -C "$d" reset -q --hard "$good"
}

echo "-- contract: the shipped pr-light.sh"
mkfx "$work/real" "$repo/scripts/pr-light.sh"
fails=0; contract "$work/real"; total_fail=$((total_fail + fails))

echo "-- hook block: the shipped scripts/hooks/pre-push"
fails=0; hookcheck "$work/real" "$repo/scripts/hooks/pre-push"; total_fail=$((total_fail + fails))

echo "-- installer"
fails=0
d="$work/inst"; mkdir -p "$d/scripts/hooks" "$d/.git/hooks"
git -C "$d" init -q
cp "$repo/scripts/install-hooks.sh" "$d/scripts/"; cp "$repo/scripts/hooks/pre-push" "$d/scripts/hooks/"
printf '#!/usr/bin/env bash\n# forjar pre-push: quality gate + quorum gate.\necho an older tracked copy\n' > "$d/.git/hooks/pre-push"
( cd "$d" && bash scripts/install-hooks.sh >/dev/null 2>&1 ); expect "installer replaces an older copy of the forjar hook" 0 "$?"
cmp -s "$d/scripts/hooks/pre-push" "$d/.git/hooks/pre-push" && r=y || r=n; expect "the installed hook is the tracked one" y "$r"
printf '#!/bin/sh\necho foreign\n' > "$d/.git/hooks/pre-push"
( cd "$d" && bash scripts/install-hooks.sh >/dev/null 2>&1 ); expect "installer still refuses a foreign hook" 1 "$(tolow "$?")"
total_fail=$((total_fail + fails))

echo "-- mutants (each must be CAUGHT)"
sed '/^set -uo pipefail$/a exit 0' "$repo/scripts/pr-light.sh" > "$work/mut-a.sh"
grep -v 'scripts/ticket-cite-lint.sh origin/main' "$repo/scripts/pr-light.sh" > "$work/mut-b.sh"
sed 's|^    cargo test --locked "\${args\[@\]}" .*|    true|' "$repo/scripts/pr-light.sh" > "$work/mut-c.sh"
for m in a b c; do
  cmp -s "$work/mut-$m.sh" "$repo/scripts/pr-light.sh" && { echo "FAIL mutant ($m) was not applied"; total_fail=$((total_fail + 1)); continue; }
  mkfx "$work/mut-$m" "$work/mut-$m.sh"
  fails=0; contract "$work/mut-$m" >/dev/null 2>&1
  if [ "$fails" -gt 0 ]; then echo "ok   mutant ($m) caught ($fails contract failure(s))"
  else echo "FAIL mutant ($m) SURVIVED"; total_fail=$((total_fail + 1)); fi
done
awk '/^# PMAT-855 \(operator C2 item 2\): pr-light FIRST/ { skip = 1 } skip && /^run_check "Format check"/ { skip = 0 } !skip' "$repo/scripts/hooks/pre-push" > "$work/hook-d"
cmp -s "$work/hook-d" "$repo/scripts/hooks/pre-push" && { echo "FAIL mutant (d) was not applied"; total_fail=$((total_fail + 1)); }
mkfx "$work/mut-d" "$repo/scripts/pr-light.sh"
fails=0; hookcheck "$work/mut-d" "$work/hook-d" >/dev/null 2>&1
if [ "$fails" -gt 0 ]; then echo "ok   mutant (d: hook without pr-light) caught ($fails failure(s))"
else echo "FAIL mutant (d) SURVIVED"; total_fail=$((total_fail + 1)); fi

if [ "$total_fail" -ne 0 ]; then echo "pr-light-test: $total_fail FAILED"; exit 1; fi
echo "pr-light-test: all ok"
