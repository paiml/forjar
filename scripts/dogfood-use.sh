#!/usr/bin/env bash
# forjar dogfood-use — exercise the RELEASE binary on real IaC. BIN = built forjar,
# WORK = scratch dir. Block-style YAML (bashrs-clean). Exit non-zero on misbehaviour.
set -uo pipefail
BIN="${BIN:?}"
WORK="${WORK:?}"
[ -d "$WORK" ] || { echo "no WORK dir" >&2; exit 1; }
cd "$WORK" || exit 1
fail() { echo "DOGFOOD-USE FAIL: $1" >&2; exit 1; }

# A clean, provable config: acyclic, disjoint targets.
{
  printf 'version: "1.0"\nname: dogfood\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n'
  printf 'resources:\n  base:\n    type: file\n    machine: m1\n    path: /etc/base\n    content: b\n'
  printf '  app:\n    type: file\n    machine: m1\n    path: /etc/app\n    content: a\n    depends_on:\n      - base\n'
} > ok.yaml
"$BIN" prove -f ok.yaml --state-dir st >/dev/null 2>&1 || fail "prove rejected a valid config"

# A dependency CYCLE must be BLOCKED (HARD invariant).
{
  printf 'version: "1.0"\nname: bad\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n'
  printf 'resources:\n  a:\n    type: file\n    machine: m1\n    path: /etc/a\n    content: a\n    depends_on:\n      - b\n'
  printf '  b:\n    type: file\n    machine: m1\n    path: /etc/b\n    content: b\n    depends_on:\n      - a\n'
} > cycle.yaml
"$BIN" prove -f cycle.yaml --state-dir st >/dev/null 2>&1 && fail "prove PASSED a cyclic config (must block)"

# A target COLLISION (two files, one path) must be BLOCKED.
{
  printf 'version: "1.0"\nname: bad2\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n'
  printf 'resources:\n  x:\n    type: file\n    machine: m1\n    path: /etc/motd\n    content: x\n'
  printf '  y:\n    type: file\n    machine: m1\n    path: /etc/motd\n    content: y\n'
} > collide.yaml
"$BIN" prove -f collide.yaml --state-dir st >/dev/null 2>&1 && fail "prove PASSED a target collision (must block)"

echo "dogfood-use OK: forjar prove accepts a valid config, BLOCKS a cycle + a target collision"
