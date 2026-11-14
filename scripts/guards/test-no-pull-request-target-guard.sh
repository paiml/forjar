#!/bin/sh
# test-no-pull-request-target-guard.sh -- the guard must go red on each bad form and green on clean ones.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
guard="$here/no-pull-request-target-guard.sh"
tmp=$(mktemp -d)
[ -n "$tmp" ] && [ -d "$tmp" ] || exit 2
trap 'chmod -R u+rwx "${tmp:?}" 2>/dev/null; rm -rf "${tmp:?}"' EXIT
fail=0

# expect <name> <want-rc> <dir> [allowlist]
expect() {
  name=$1 want=$2 d=$3 al=${4:-$tmp/none}
  rc=0
  sh "$guard" "$d" "$al" >/dev/null 2>&1 || rc=$?
  if [ "$rc" -ne "$want" ]; then
    echo "FAIL $name: want rc=$want got rc=$rc"
    fail=$((fail + 1))
  else
    echo "ok   $name"
  fi
}

mk() { mkdir -p "$tmp/$1"; }

mk clean;   printf 'on:\n  pull_request:\n    types: [opened]\n' > "$tmp/clean/a.yml"
mk map;     printf 'on:\n  pull_request_target:\n    types: [opened]\n' > "$tmp/map/a.yml"
mk inline;  printf 'on: [push, pull_request_target]\n' > "$tmp/inline/a.yml"
mk expr;    printf "concurrency:\n  x: \${{ github.event_name == 'pull_request_target' }}\n" > "$tmp/expr/a.yaml"
mk comment; printf '# pull_request_target is banned\non:\n  pull_request:\n' > "$tmp/comment/a.yml"
mk trail;   printf 'on: [push] # pull_request_target\n' > "$tmp/trail/a.yml"
mk qhash;   printf '{"name": "ci #1", "on": {"pull_request_target": {"types": ["opened"]}}, "jobs": {}}\n' > "$tmp/qhash/a.yml"
mk escu;    printf 'on: { "pull_request\\u005ftarget": {} }\n' > "$tmp/escu/a.yml"
mk escx;    printf 'on: { "pull_request\\x5ftarget": {} }\n' > "$tmp/escx/a.yml"
mk case;    printf 'on: [Pull_Request_Target]\n' > "$tmp/case/a.yml"
mk unread;  printf 'on:\n  pull_request_target:\n' > "$tmp/unread/a.yml"; chmod 000 "$tmp/unread/a.yml"
mk dangle;  printf 'on:\n  pull_request:\n' > "$tmp/dangle/a.yml"; ln -s "$tmp/gone.yml" "$tmp/dangle/b.yml"
mk ident;   printf 'jobs:\n  j:\n    steps:\n      - run: cargo test --test falsification_no_pull_request_target\n' > "$tmp/ident/a.yml"
mk empty
mk allowed; printf 'on:\n  pull_request_target:\n' > "$tmp/allowed/closer.yml"
mk allowco; printf 'on:\n  pull_request_target:\njobs:\n  j:\n    steps:\n      - uses: actions/checkout@v4\n' > "$tmp/allowco/closer.yml"
mk allowCo; printf 'on:\n  pull_request_target:\njobs:\n  j:\n    steps:\n      - uses: Actions/Checkout@v4\n' > "$tmp/allowCo/closer.yml"
mk allowrun; printf 'on:\n  pull_request_target:\njobs:\n  j:\n    steps:\n      - run: git clone x\n' > "$tmp/allowrun/closer.yml"
printf 'closer.yml  # metadata only\n' > "$tmp/allow.txt"
mk allowre;  printf 'on:\n  pull_request_target:\n' > "$tmp/allowre/a.b.yml"
printf 'aXb.yml\n' > "$tmp/allow-regex.txt"
printf 'closer.yml\n' > "$tmp/allow-unread.txt"; chmod 000 "$tmp/allow-unread.txt"

expect clean-passes              0 "$tmp/clean"
expect map-form-fails            1 "$tmp/map"
expect inline-form-fails         1 "$tmp/inline"
expect expression-form-fails     1 "$tmp/expr"
expect comment-mention-fails     1 "$tmp/comment"
expect trailing-comment-fails    1 "$tmp/trail"
expect quoted-hash-fails         1 "$tmp/qhash"
expect unicode-escape-fails      1 "$tmp/escu"
expect hex-escape-fails          1 "$tmp/escx"
expect mixed-case-fails          1 "$tmp/case"
expect dangling-symlink-is-io    2 "$tmp/dangle"
expect longer-identifier-passes  0 "$tmp/ident"
expect zero-files-not-a-pass     2 "$tmp/empty"
expect missing-dir-is-usage      2 "$tmp/nope"
expect allowlisted-passes        0 "$tmp/allowed" "$tmp/allow.txt"
expect allowlisted-checkout      1 "$tmp/allowco" "$tmp/allow.txt"
expect allowlisted-Checkout-case 1 "$tmp/allowCo" "$tmp/allow.txt"
expect allowlisted-run           1 "$tmp/allowrun" "$tmp/allow.txt"
expect allowlist-is-not-regex    1 "$tmp/allowre" "$tmp/allow-regex.txt"
expect not-allowlisted-fails     1 "$tmp/allowed"
# root reads mode-000 files, so the unreadable cases are measured only for a non-root user.
if [ "$(id -u)" -ne 0 ]; then
  expect unreadable-workflow-is-io  2 "$tmp/unread"
  expect unreadable-allowlist-is-io 2 "$tmp/allowed" "$tmp/allow-unread.txt"
else
  expect unreadable-workflow-not-0  1 "$tmp/unread"
fi

# the repo's own workflows must be clean
repo=$(cd "$here/../.." && pwd)
if [ -d "$repo/.github/workflows" ]; then
  rc=0
  sh "$guard" "$repo/.github/workflows" "$repo/.github/pull-request-target-allowlist.txt" >/dev/null 2>&1 || rc=$?
  if [ "$rc" -ne 0 ]; then echo "FAIL repo-workflows: rc=$rc"; fail=$((fail + 1)); else echo "ok   repo-workflows"; fi
fi

[ "$fail" -eq 0 ] || { echo "$fail failure(s)"; exit 1; }
echo "all passed"
