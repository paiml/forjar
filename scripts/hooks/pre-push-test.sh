#!/usr/bin/env bash
# Pins the pre-push policy (#859): only `cargo fmt --check` blocks a push; check,
# clippy, test and the quorum gate run and warn. Drives the real hook against a
# stub `cargo` and a stub quorum gate in a scratch tree.
set -u
here=$(cd "$(dirname "$0")" && pwd) || exit 2
hook="$here/pre-push"
T=$(mktemp -d) || exit 2
trap 'rm -rf -- "${T:?}"' EXIT
mkdir -p "$T/bin" "$T/repo/scripts"
touch "$T/repo/Cargo.toml"
# Stub cargo: subcommand fails when its name is in $FAIL_CARGO (space separated).
cat > "$T/bin/cargo" <<'STUB'
#!/usr/bin/env bash
case " ${FAIL_CARGO:-} " in *" $1 "*) echo "stub $1 failed"; exit 1 ;; esac
exit 0
STUB
cat > "$T/repo/scripts/quorum-gate.sh" <<'STUB'
#!/usr/bin/env bash
[ "${FAIL_QUORUM:-0}" = 1 ] && { echo "stub quorum failed"; exit 1; }
exit 0
STUB
chmod +x "$T/bin/cargo" "$T/repo/scripts/quorum-gate.sh"

fail=0
# run <name> <want-rc> <FAIL_CARGO> <FAIL_QUORUM>
run() {
  local name=$1 want=$2 got
  printf 'refs/heads/f 1111111111111111111111111111111111111111 refs/heads/f 0\n' |
    (cd "$T/repo" && PATH="$T/bin:$PATH" FAIL_CARGO="$3" FAIL_QUORUM="$4" "$hook") >"$T/out" 2>&1
  got=$?
  if [ "$got" -ne "$want" ]; then
    echo "FAIL $name: want rc=$want got rc=$got"
    fail=$((fail + 1))
  fi
}
run all-green            0 ""            0
run fmt-blocks           1 "fmt"         0
run check-warns          0 "check"       0
run clippy-warns         0 "clippy"      0
run test-warns           0 "test"        0
run quorum-warns         0 ""            1
run everything-but-fmt   0 "check clippy test" 1
run fmt-plus-others      1 "fmt clippy"  1
# A warned step must still be visible in the output.
run clippy-warns         0 "clippy"      0
grep -q "WARN" "$T/out" || { echo "FAIL warn-visible: no WARN line"; fail=$((fail + 1)); }
[ "$fail" -eq 0 ] && echo "pre-push policy: all cases pass" || exit 1
