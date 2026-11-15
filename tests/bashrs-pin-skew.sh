#!/usr/bin/env bash
# Case table for scripts/bashrs-pin-skew.sh (PMAT-1015). The check is LAB, so the
# last case refuses any workflow under .github/workflows that names it.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 2
chk=scripts/bashrs-pin-skew.sh
tmp=$(mktemp -d)
trap 'rm -rf -- "${tmp:?}"' EXIT
fail=0

# expect <name> <want-exit> <grep-pattern-or-empty> -- <args...>
expect() {
  local name="$1" want="$2" pat="$3"; shift 4
  local got=0 out
  out=$(bash "$chk" "$@" 2>&1) || got=$?
  if [ "$got" -ne "$want" ]; then
    echo "FAIL $name: exit $got, want $want"; printf '%s\n' "$out" | sed 's/^/    /'; fail=1; return
  fi
  if [ -n "$pat" ] && ! printf '%s\n' "$out" | grep -Eq -- "$pat"; then
    echo "FAIL $name: output lacks /$pat/"; printf '%s\n' "$out" | sed 's/^/    /'; fail=1; return
  fi
  echo "ok $name"
}

mk() { mkdir -p "$tmp/$1/$2"; cat > "$tmp/$1/$2/forjar.yaml"; }
printf '[dependencies]\nbashrs = "7.4.3"\n' > "$tmp/cargo-ok.toml"
printf '[dependencies]\nbashrs = { version = "=7.4.2" }\n' > "$tmp/cargo-old.toml"
printf '[dependencies]\nserde = "1"\n' > "$tmp/cargo-none.toml"

# One machine per pin form, all at 7.4.3; comment lines carry stale numbers
# that must be ignored.
mk aligned a <<'EOF'
params:
  # bashrs_version: "7.0.1"
  bashrs_version: "7.4.3"
resources:
  stack-tool-bashrs:
    type: package
    # version: "7.4.1"
    version: "7.4.3"
    packages: [bashrs]
  stack-tool-pv:
    type: package
    version: "0.72.3"
EOF
mk aligned b <<'EOF'
resources:
  fleet-bashrs-pin:
    type: file
    content: |
      7.4.3
    mode: "0644"
  # was: cargo install bashrs --version 7.0.1 (bashrs@7.0.1)
  course-bashrs:
    type: file
    command: |
      cargo install --locked bashrs --version 7.4.3
      cargo install pv --version 0.72.3
EOF
mk aligned none <<'EOF'
resources:
  unrelated:
    type: file
    version: "9.9.9"
EOF
expect aligned 0 'pins=5 skewed=0' -- --latest v7.4.3 --machines "$tmp/aligned" --cargo "$tmp/cargo-ok.toml"

# Every form detects its own skew, and only the skewed row says SKEW.
mk skew param <<'EOF'
params:
  bashrs_version: "7.4.2"
EOF
mk skew stack <<'EOF'
resources:
  stack-tool-bashrs:
    version: "7.4.2"
EOF
mk skew pinfile <<'EOF'
resources:
  fleet-bashrs-pin:
    content: |

      7.4.1
EOF
mk skew install <<'EOF'
resources:
  x:
    command: cargo install bashrs@7.4.1
EOF
mk skew fine <<'EOF'
params:
  bashrs_version: "7.4.3"
EOF
expect skew-exit 1 'pins=6 skewed=4' -- --latest 7.4.3 --machines "$tmp/skew" --cargo "$tmp/cargo-ok.toml"
expect skew-param 1 'param[[:space:]]+param[[:space:]]+7\.4\.2[[:space:]]+SKEW' -- --latest 7.4.3 --machines "$tmp/skew" --cargo "$tmp/cargo-ok.toml"
expect skew-stack 1 'stack[[:space:]]+stack-tool[[:space:]]+7\.4\.2[[:space:]]+SKEW' -- --latest 7.4.3 --machines "$tmp/skew" --cargo "$tmp/cargo-ok.toml"
expect skew-pinfile 1 'pinfile[[:space:]]+pin-file[[:space:]]+7\.4\.1[[:space:]]+SKEW' -- --latest 7.4.3 --machines "$tmp/skew" --cargo "$tmp/cargo-ok.toml"
expect skew-install 1 'install[[:space:]]+install[[:space:]]+7\.4\.1[[:space:]]+SKEW' -- --latest 7.4.3 --machines "$tmp/skew" --cargo "$tmp/cargo-ok.toml"
expect skew-fine-ok 1 'fine[[:space:]]+param[[:space:]]+7\.4\.3[[:space:]]+ok' -- --latest 7.4.3 --machines "$tmp/skew" --cargo "$tmp/cargo-ok.toml"

# forjar's own Cargo.toml requirement is a pin too (an exact `=` requirement reads the same).
expect cargo-skew 1 'cargo-req[[:space:]]+7\.4\.2[[:space:]]+SKEW' -- --latest 7.4.3 --machines "$tmp/aligned" --cargo "$tmp/cargo-old.toml"

# Pin syntax variants (fix round 1). Every pin is read to its exact version or
# reported CANNOT-PARSE; nothing is silently skipped, and a trailing comment can
# never supply the version. Latest is 7.4.3 throughout.
mk forms trail <<'EOF2'
resources:
  x:
    command: cargo install bashrs@7.4.1 # latest is bashrs --version 7.4.3
EOF2
mk forms trail2 <<'EOF2'
resources:
  x:
    command: cargo install bashrs --version 7.4.2   # bump to bashrs@7.4.3 later
EOF2
mk forms caret <<'EOF2'
params:
  bashrs_version: "^7.4.3"
EOF2
mk forms tilde <<'EOF2'
params:
  bashrs_version: "~7.4.2"
EOF2
mk forms ge <<'EOF2'
params:
  bashrs_version: ">=7.4.1"
EOF2
mk forms eq <<'EOF2'
params:
  bashrs_version: "=7.4.0"
EOF2
mk forms vpre <<'EOF2'
params:
  bashrs_version: v7.4.5
EOF2
mk forms single <<'EOF2'
params:
  bashrs_version: '7.4.6'
EOF2
mk forms rc <<'EOF2'
params:
  bashrs_version: "7.4.3-rc.1"
EOF2
mk forms inline <<'EOF2'
params:
  bashrs_version: "7.4.7" # was 7.4.3
EOF2
mk forms word <<'EOF2'
params:
  bashrs_version: latest
EOF2
mk forms shellvar <<'EOF2'
resources:
  x:
    command: cargo install bashrs --version $BASHRS_VER
EOF2
mk forms query <<'EOF2'
resources:
  x:
    command: bashrs --version | head -n 1
EOF2
printf '[dependencies]\nbashrs = { version = "7.4.1", features = ["x"] } # latest is 7.4.3\n' > "$tmp/cargo-brace-comment.toml"
printf '[dependencies]\nbashrs = "~7.4.2"\n' > "$tmp/cargo-tilde.toml"
printf '[dependencies]\nbashrs = "7.4.3-rc.1"\n' > "$tmp/cargo-rc.toml"
printf '[dependencies]\nbashrs = ">=7.4.1, <8"\n' > "$tmp/cargo-range.toml"
printf '[dependencies]\nbashrs = { path = "../bashrs" }\n' > "$tmp/cargo-path.toml"
F=(--latest 7.4.3 --machines "$tmp/forms" --cargo "$tmp/cargo-ok.toml")
expect form-trail 1 '^trail[[:space:]]+install[[:space:]]+7\.4\.1[[:space:]]+SKEW' -- "${F[@]}"
expect form-trail2 1 '^trail2[[:space:]]+install[[:space:]]+7\.4\.2[[:space:]]+SKEW' -- "${F[@]}"
expect form-caret 1 '^caret[[:space:]]+param[[:space:]]+7\.4\.3[[:space:]]+ok' -- "${F[@]}"
expect form-tilde 1 '^tilde[[:space:]]+param[[:space:]]+7\.4\.2[[:space:]]+SKEW' -- "${F[@]}"
expect form-ge 1 '^ge[[:space:]]+param[[:space:]]+7\.4\.1[[:space:]]+SKEW' -- "${F[@]}"
expect form-eq 1 '^eq[[:space:]]+param[[:space:]]+7\.4\.0[[:space:]]+SKEW' -- "${F[@]}"
expect form-vprefix 1 '^vpre[[:space:]]+param[[:space:]]+7\.4\.5[[:space:]]+SKEW' -- "${F[@]}"
expect form-single-quote 1 '^single[[:space:]]+param[[:space:]]+7\.4\.6[[:space:]]+SKEW' -- "${F[@]}"
expect form-prerelease 1 '^rc[[:space:]]+param[[:space:]]+7\.4\.3-rc\.1[[:space:]]+SKEW' -- "${F[@]}"
expect form-inline-comment 1 '^inline[[:space:]]+param[[:space:]]+7\.4\.7[[:space:]]+SKEW' -- "${F[@]}"
expect form-word-unparseable 1 '^word[[:space:]]+param[[:space:]]+UNPARSEABLE[[:space:]]+CANNOT-PARSE' -- "${F[@]}"
expect form-shellvar-unparseable 1 '^shellvar[[:space:]]+install[[:space:]].*CANNOT-PARSE' -- "${F[@]}"
expect form-version-query-skipped 1 'unparseable=2' -- "${F[@]}"
mkdir -p "$tmp/only-query"; mk only-query q <<'EOF2'
resources:
  x:
    command: bashrs --version | head -n 1
EOF2
expect form-query-is-no-pin 2 'NOT MEASURED: no bashrs pin' -- --latest 7.4.3 --machines "$tmp/only-query" --cargo "$tmp/cargo-ok.toml"
expect cargo-brace-comment 1 'cargo-req[[:space:]]+7\.4\.1[[:space:]]+SKEW' -- --latest 7.4.3 --machines "$tmp/aligned" --cargo "$tmp/cargo-brace-comment.toml"
expect cargo-tilde 1 'cargo-req[[:space:]]+7\.4\.2[[:space:]]+SKEW' -- --latest 7.4.3 --machines "$tmp/aligned" --cargo "$tmp/cargo-tilde.toml"
expect cargo-prerelease 1 'cargo-req[[:space:]]+7\.4\.3-rc\.1[[:space:]]+SKEW' -- --latest 7.4.3 --machines "$tmp/aligned" --cargo "$tmp/cargo-rc.toml"
expect cargo-range-unparseable 1 'cargo-req[[:space:]].*CANNOT-PARSE' -- --latest 7.4.3 --machines "$tmp/aligned" --cargo "$tmp/cargo-range.toml"
expect cargo-path-only-not-measured 2 'no bashrs requirement' -- --latest 7.4.3 --machines "$tmp/aligned" --cargo "$tmp/cargo-path.toml"

# Not measured is exit 2, never a pass.
expect no-pins 2 'NOT MEASURED: no bashrs pin' -- --latest 7.4.3 --machines "$tmp/aligned/none" --cargo "$tmp/cargo-ok.toml"
mkdir -p "$tmp/empty"
expect empty-tree 2 'NOT MEASURED: no bashrs pin' -- --latest 7.4.3 --machines "$tmp/empty" --cargo "$tmp/cargo-ok.toml"
expect missing-dir 2 'machines dir .* is missing' -- --latest 7.4.3 --machines "$tmp/absent" --cargo "$tmp/cargo-ok.toml"
expect missing-cargo 2 'Cargo.toml .* is missing' -- --latest 7.4.3 --machines "$tmp/aligned" --cargo "$tmp/absent.toml"
expect no-cargo-req 2 'no bashrs requirement' -- --latest 7.4.3 --machines "$tmp/aligned" --cargo "$tmp/cargo-none.toml"
expect bad-latest 2 'is not X.Y.Z' -- --latest 7.4 --machines "$tmp/aligned" --cargo "$tmp/cargo-ok.toml"
expect no-args 2 'usage' --

# LAB: no workflow may run this check (no PR, merge-queue or release job).
if grep -rln 'bashrs-pin-skew' .github/workflows >/dev/null 2>&1; then
  echo "FAIL lab-only: a workflow runs the LAB check: $(grep -rln 'bashrs-pin-skew' .github/workflows | tr '\n' ' ')"; fail=1
else
  echo "ok lab-only: no workflow under .github/workflows runs bashrs-pin-skew"
fi
exit "$fail"
