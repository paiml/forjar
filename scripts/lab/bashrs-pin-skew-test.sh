#!/usr/bin/env bash
# Case table for scripts/lab/bashrs-pin-skew.sh (PMAT-1015). LAB: run by
# `make lab-bashrs-pin-skew-test`, never by a PR, merge-queue or release job.
# Every case builds its own fixture trees under a temp dir; no network.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
CHECK="$HERE/bashrs-pin-skew.sh"
WORK="$(mktemp -d)"
trap 'rm -rf -- "${WORK:?}"' EXIT

pass=0
fail=0

# fixture DIR FORJAR_REQ LOCK_VER F16 KFOCUS LAMBDA_PIN LAMBDA_INSTALL
fixture() {
    local d="$1"
    mkdir -p "$d/forjar" "$d/infra/machines/framework16" "$d/infra/machines/kfocus" \
        "$d/infra/machines/lambda-labs" "$d/infra/machines/lib"
    printf '[dependencies]\nbase64 = "0.22.1"\nbashrs = "%s"\nclap = "4"\n' "$2" >"$d/forjar/Cargo.toml"
    printf '[[package]]\nname = "bashrs"\nversion = "%s"\n\n[[package]]\nname = "bashrs-extra"\nversion = "0.0.1"\n' \
        "$3" >"$d/forjar/Cargo.lock"
    printf 'params:\n  # bashrs_version: "6.0.0" (a comment, never read)\n  bashrs_version: "%s"\n' "$4" \
        >"$d/infra/machines/framework16/forjar.yaml"
    printf 'params:\n  bashrs_version: %s\n' "$5" >"$d/infra/machines/kfocus/forjar.yaml"
    {
        printf 'resources:\n  fleet-bashrs-pin:\n    type: file\n    content: |\n      %s\n    mode: "0644"\n' "$6"
        printf '  course-bashrs:\n    # cargo install bashrs --version 1.0.0 (comment)\n'
        # shellcheck disable=SC2016 # a literal $d, written into the fixture
        printf '    script: |\n      cargo install --locked --root "$d" bashrs --version %s\n' "$7"
    } >"$d/infra/machines/lambda-labs/forjar.yaml"
}

# expect NAME WANT_EXIT GREP_PATTERN -- ARGS...
expect() {
    local name="$1" want="$2" pat="$3" out rc
    shift 4
    set +e
    out="$(bash "$CHECK" "$@" 2>&1)"
    rc=$?
    set -e
    if [ "$rc" -eq "$want" ] && grep -Eq -- "$pat" <<<"$out"; then
        pass=$((pass + 1))
        echo "ok   $name"
    else
        fail=$((fail + 1))
        echo "FAIL $name (exit $rc, want $want, pattern '$pat')"
        while IFS= read -r l; do echo "     | $l"; done <<<"$out"
    fi
}

# 1. every pin at latest -> exit 0, 6 rows
fixture "$WORK/green" 7.4.3 7.4.3 7.4.3 7.4.3 7.4.3 7.4.3
expect green 0 'latest=7.4.3 rows=6 skew=0' -- --latest 7.4.3 --forjar "$WORK/green/forjar" --infra "$WORK/green/infra"

# 2. the measured 2026-11-14 shape -> exit 1, every stale row named SKEW
fixture "$WORK/skew" 7.4.2 7.4.2 7.4.2 7.4.2 7.4.1 7.4.1
expect skew-exit 1 'rows=6 skew=6' -- --latest 7.4.3 --forjar "$WORK/skew/forjar" --infra "$WORK/skew/infra"
expect skew-lambda-pin 1 'lambda-labs fleet-bashrs-pin content	7\.4\.1	7\.4\.3	SKEW' -- \
    --latest 7.4.3 --forjar "$WORK/skew/forjar" --infra "$WORK/skew/infra"
expect skew-lambda-install 1 'lambda-labs cargo install --version	7\.4\.1	7\.4\.3	SKEW' -- \
    --latest 7.4.3 --forjar "$WORK/skew/forjar" --infra "$WORK/skew/infra"
expect skew-forjar-floor 1 'forjar Cargo.toml \(requirement floor\)	7\.4\.2	7\.4\.3	SKEW' -- \
    --latest 7.4.3 --forjar "$WORK/skew/forjar" --infra "$WORK/skew/infra"

# 3. one pin behind -> exit 1, skew=1 (discriminates a single row)
fixture "$WORK/one" 7.4.3 7.4.3 7.4.3 7.4.2 7.4.3 7.4.3
expect one-row 1 'kfocus params.bashrs_version	7\.4\.2	7\.4\.3	SKEW' -- \
    --latest 7.4.3 --forjar "$WORK/one/forjar" --infra "$WORK/one/infra"
expect one-count 1 'skew=1$' -- --latest 7.4.3 --forjar "$WORK/one/forjar" --infra "$WORK/one/infra"

# 4. exact-pin and table forms of the requirement are read
fixture "$WORK/exact" 7.4.3 7.4.3 7.4.3 7.4.3 7.4.3 7.4.3
sed -i 's/^bashrs = .*/bashrs = { version = "=7.4.3", default-features = false }/' "$WORK/exact/forjar/Cargo.toml"
expect table-form 0 'skew=0' -- --latest 7.4.3 --forjar "$WORK/exact/forjar" --infra "$WORK/exact/infra"

# 5. not_measured, never a verdict: no infra, no latest, no requirement, no lock
expect no-infra 2 'NOT_MEASURED no infra checkout' -- --latest 7.4.3 --forjar "$WORK/green/forjar"
expect bad-latest 2 'NOT_MEASURED latest bashrs tag unknown' -- --latest nightly --forjar "$WORK/green/forjar" --infra "$WORK/green/infra"
fixture "$WORK/noreq" 7.4.3 7.4.3 7.4.3 7.4.3 7.4.3 7.4.3
sed -i '/^bashrs = /d' "$WORK/noreq/forjar/Cargo.toml"
expect no-requirement 2 'NOT_MEASURED no bashrs requirement' -- --latest 7.4.3 --forjar "$WORK/noreq/forjar" --infra "$WORK/noreq/infra"
fixture "$WORK/nolock" 7.4.3 7.4.3 7.4.3 7.4.3 7.4.3 7.4.3
rm -f -- "${WORK:?}/nolock/forjar/Cargo.lock"
expect no-lock 2 'NOT_MEASURED forjar Cargo.lock unreadable' -- --latest 7.4.3 --forjar "$WORK/nolock/forjar" --infra "$WORK/nolock/infra"
mkdir -p "$WORK/empty-infra/machines/x"
printf 'params: {}\n' >"$WORK/empty-infra/machines/x/forjar.yaml"
expect no-host-pins 2 'NOT_MEASURED no bashrs pin found' -- --latest 7.4.3 --forjar "$WORK/green/forjar" --infra "$WORK/empty-infra"

# 6. latest discovered from tags (sort -V: v7.4.10 > v7.4.9; non-semver tags ignored)
git init -q --template= "$WORK/tags"
git -C "$WORK/tags" -c core.hooksPath=/dev/null -c user.name=t -c user.email=t@t commit -q --allow-empty -m t
for t in v7.4.2 v7.4.9 v7.4.10 nightly v8.0.0-rc1; do git -C "$WORK/tags" tag "$t"; done
fixture "$WORK/ten" 7.4.10 7.4.10 7.4.10 7.4.10 7.4.10 7.4.10
BASHRS_REMOTE="$WORK/tags" expect tag-discovery 0 'latest=7\.4\.10 rows=6 skew=0' -- \
    --forjar "$WORK/ten/forjar" --infra "$WORK/ten/infra"

# 7. --rev reads committed objects, not the working tree
git -C "$WORK/skew/infra" init -q --template=
git -C "$WORK/skew/infra" add -A
git -C "$WORK/skew/infra" -c core.hooksPath=/dev/null -c user.name=t -c user.email=t@t commit -q -m t
sed -i 's/7\.4\.1/7.4.3/' "$WORK/skew/infra/machines/lambda-labs/forjar.yaml"
git -C "$WORK/skew/forjar" init -q --template=
git -C "$WORK/skew/forjar" add -A
git -C "$WORK/skew/forjar" -c core.hooksPath=/dev/null -c user.name=t -c user.email=t@t commit -q -m t
expect rev-reads-commit 1 'lambda-labs fleet-bashrs-pin content	7\.4\.1' -- \
    --latest 7.4.3 --rev HEAD --forjar "$WORK/skew/forjar" --infra "$WORK/skew/infra"
expect worktree-reads-edit 1 'lambda-labs fleet-bashrs-pin content	7\.4\.3	7\.4\.3	OK' -- \
    --latest 7.4.3 --forjar "$WORK/skew/forjar" --infra "$WORK/skew/infra"

echo "bashrs-pin-skew-test: ${pass} passed, ${fail} failed"
[ "$fail" -eq 0 ]
