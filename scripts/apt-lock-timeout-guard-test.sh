#!/usr/bin/env bash
# Proves the guard discriminates: real tree passes, planted bare apt-get fails.
set -u
here=$(cd "$(dirname "$0")" && pwd)
g="$here/apt-lock-timeout-guard.sh"
"$g" "$here/../.github/workflows" >/dev/null || { echo "FAIL real tree"; exit 1; }
T=$(mktemp -d); trap 'rm -rf -- "${T:?}"' EXIT
printf 'jobs:\n  a:\n    steps:\n      - run: sudo apt-get install -y musl-tools\n' > "$T/x.yml"
"$g" "$T" >/dev/null; [ $? -eq 1 ] || { echo "FAIL mutant not caught"; exit 1; }
mkdir "$T/e"; "$g" "$T/e" >/dev/null; [ $? -eq 2 ] || { echo "FAIL empty not 2"; exit 1; }
echo "PASS apt-lock-timeout-guard (real ok, mutant=1, empty=2)"
