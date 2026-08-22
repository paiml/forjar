---
allowed-tools: Bash(cargo:*), Bash(forjar:*), Bash(pmat:*), Bash(gh:*), Bash(git:*), Bash(find:*), Bash(head:*), Bash(tail:*), Bash(wc:*), Bash(grep:*), Bash(diff:*), Bash(timeout:*), Bash(jq:*), Bash(python3:*), Bash(echo:*), Bash(cat:*), Bash(rm:*), Bash(mktemp:*), Bash(mkdir:*), Bash(sh:*), Bash(bashrs:*), Read, Glob, Grep, Agent
description: Dogfood forjar — rebuild, install, exercise every command, prove the C1–C10 claims and contracts against a sandboxed stack, check quality, file bugs
effort: high
---

# Forjar CLI Exhaustive QA — Contract-First Dogfood

**Contracts** (`contracts/`): `idempotent-apply-v1`, `plan-apply-equivalence-v1`,
`destroy-undo-roundtrip-v1`, `dag-ordering-v1`, `blake3-state-v1`,
`execution-safety-v1`, `recipe-determinism-v1`, `codegen-dispatch-v1`,
`apply-summary-distinguishability-v1`, plus `binding.yaml` (AllImplemented).
**Claims**: README falsifiable claims **C1–C10**.
**Pattern**: modeled on `../aprender/.claude/skills/dogfood/SKILL.md`, adapted for an IaC tool.

## ⚠️ SAFETY — read before running anything

forjar is **Infrastructure as Code**: `apply`, `destroy`, `undo`, `import`,
`reseal`, `state-*`, `lock-rotate-keys`, package/service/mount/network providers
**mutate real state or the host**. This dogfood is a **read-only audit of the
host**. Therefore:

- **Inspection/preview commands are always safe**: `init validate plan diff
  stack-diff fmt lint show graph status history explain doctor inventory
  bench suggest compare compliance policy-coverage test contracts prove
  lock-{info,stats,verify,audit,verify-chain,validate} audit` and any
  `--help` / `--dry-run` / `--preview` / `--diff-only` / `--output-scripts`.
- **A real mutating `apply`/`destroy`/`undo` may ONLY run** against a
  throwaway `mktemp -d` **state dir** + a config whose ONLY resources are
  `type: file`/`type: command` writing under `/tmp`, on the **local** machine
  (`addr: 127.0.0.1` → direct bash, not loopback SSH). **Never** apply
  `package`/`service`/`mount`/`network`/`gpu`/`github_release` resources to
  this host. **Never** run `destroy`/`undo`/`state-rm`/`lock-rotate-keys`
  against a real/default state dir.
- **Do NOT modify repo files.** If a gate finds a bug, file a GitHub issue
  (`gh issue create --repo paiml/forjar`). Clean up all tempdirs at the end.

### Exit-code capture (mandatory)
Never read `$?` after a pipe — it's the pipe's last stage, not the command's.
Use `OUT=$(cmd 2>&1); EC=$?`. (This exact mistake produced retracted false-positive
bugs in the aprender dogfood; see that skill's Pre-Gate note.)

## Context

- forjar source version: !`grep -m1 '^version' Cargo.toml`
- Git commit: !`git rev-parse --short HEAD`
- Installed forjar: !`forjar --version 2>/dev/null || echo "not installed"`
- crates.io published: !`curl -s --max-time 8 https://crates.io/api/v1/crates/forjar | python3 -c 'import json,sys;print(json.load(sys.stdin)["crate"]["newest_version"])' 2>/dev/null || echo "(unreachable)"`
- Subcommands: !`grep -cE '^\s+[A-Z][A-Za-z0-9]+\(' src/cli/commands/mod.rs`
- Contracts: !`ls contracts/*.yaml 2>/dev/null | wc -l`
- Lib tests: !`grep -rc '#\[test\]' src/ 2>/dev/null | paste -sd+ | bc 2>/dev/null || echo '?'`

## Arguments

$ARGUMENTS

If an argument names a config file, exercise that one too (read-only/preview
only unless it is provably a /tmp-file-only local stack). Otherwise use
`examples/*.yaml` and a generated sandbox stack.

## Your Task

Run ALL gates. For each: run the check, report **PASS/FAIL/SKIP** with evidence.
Run independent gates in parallel where safe. End with a **GO / WARN / FAIL**
verdict. **Read-only w.r.t. the host.** File issues for bugs; do not fix code.

---

## Gate 1: Build & Install (proves the binary is this commit)

```bash
cargo install --path . --force 2>&1 | tail -5
forjar --version
git rev-parse --short HEAD
```
PASS if `forjar --version` is the source version and the build is clean.
WARN if the embedded SHA (if any) ≠ HEAD. FAIL on build error.

## Gate 2: Command Grid — every subcommand responds to `--help`, no phantoms

```bash
# Auto-discover every top-level subcommand and assert --help works (no panic,
# no "not implemented"). This is the phantom-subcommand + crash sweep.
forjar --help 2>&1 | awk '/^  [a-z]/{print $1}' | while read -r c; do
  OUT=$(timeout 20 forjar "$c" --help 2>&1); EC=$?
  if [ "$EC" -ne 0 ]; then echo "G2 FAIL $c: --help exit $EC"
  elif echo "$OUT" | grep -qiE 'not.*implemented|todo|unimplemented'; then echo "G2 PHANTOM $c"
  else echo "G2 ok $c"; fi
done | sort | uniq -c | sort -rn | head
```
Then spot-exercise the safe inspection commands on an example config:
```bash
CFG=$(ls examples/*.yaml 2>/dev/null | head -1)
for c in validate plan diff fmt lint show graph explain inventory; do
  OUT=$(timeout 30 forjar "$c" -f "$CFG" 2>&1 || timeout 30 forjar "$c" "$CFG" 2>&1); EC=$?
  echo "G2.$c: exit=$EC $(echo "$OUT" | head -1)"
done
```
PASS if every subcommand's `--help` exits 0 with no phantom/panic. FAIL on any panic.

## Gate 3: Falsifiable Claims C1–C10 (the heart of the forjar dogfood)

Each maps to a README claim. Build a tiny sandbox stack first:
```bash
SBX=$(mktemp -d); ST="$SBX/state"; CFG="$SBX/forjar.yaml"; TGT="$SBX/out.txt"
cat > "$CFG" <<YAML
version: "1.0"
name: dogfood
machines: { local: { hostname: localhost, addr: 127.0.0.1 } }
resources:
  a: { type: file, machine: local, path: $TGT, content: "dogfood" }
  b: { type: command, machine: local, depends_on: [a], run: "true", creates: $SBX/b.done }
YAML
```
- **C1 Deterministic hashing**: two `forjar plan` runs over the same config
  produce identical resource hashes (`plan --output-scripts` / `plan` diff = empty).
- **C2 Deterministic DAG order**: `forjar graph -f "$CFG"` (or `plan`) yields a
  stable topological order across 3 runs (diff = empty); `b` after `a`.
- **C3 Idempotent apply** (#129): `forjar apply -f "$CFG" --state-dir "$ST" --yes`,
  then a **second** apply → summary shows `0 changed` / all unchanged; with
  `--force`, the summary must still distinguish `M actual change(s)` (M=0) from
  forced no-ops. (Mutating, but /tmp-only local — SAFE.)
- **C4 Cycle detection**: a config with `a→b→a` `depends_on` cycle → `validate`
  exits non-zero with a clear cycle error (`OUT=$(...); EC=$?`).
- **C5 Content-addressed state**: after apply, the state dir has BLAKE3-addressed
  entries / `.b3` sidecars; `forjar lock-verify`/`lock-verify-chain` PASS.
- **C6 Atomic state persistence**: state writes are temp+rename (no partial file
  after an interrupted run); `lock-verify` is clean.
- **C7 Recipe input validation**: a recipe with a missing/typed-wrong `{{inputs.*}}`
  → `validate` fails loud non-zero.
- **C8 Heredoc injection safety**: a `command`/`file content` value containing
  `'`, `$()`, backticks is shell-escaped in `plan --output-scripts` output
  (the payload is neutralized, not executed) — `bashrs lint` the emitted script.
- **C9 Minimal dependencies**: `cargo metadata --no-deps --format-version 1 |
  jq '[.packages[0].dependencies[]|select(.kind==null)]|length'` within the
  README-stated bound.
- **C10 Jidoka failure isolation**: a stack where resource `x` fails its
  `pre_apply`/check → apply **fails loud** (non-zero exit, `x` failed,
  dependents cascade-skipped) and does **not** report success. (Recent fix:
  a failing `pre_apply` hook fails apply and is not retried.)

PASS if all 10 hold. FAIL on any claim that falsifies.

## Gate 4: Contract validation

```bash
for c in contracts/*.yaml; do
  python3 -c "import yaml;yaml.safe_load(open('$c'))" 2>&1 && echo "  valid: $c" || echo "  INVALID: $c"
done
# AllImplemented binding policy is enforced at build time:
cargo build 2>&1 | grep -iE 'CONTRACT|binding' | tail -3
# Contract-enforcing tests:
cargo test --lib falsification 2>&1 | grep 'test result' | tail -3
```
PASS if all contracts parse, `cargo build` reports all bindings bound, and the
falsification tests pass.

## Gate 5: Code quality

```bash
cargo test --lib 2>&1 | grep 'test result' | tail -1
cargo clippy --all-targets -- -D warnings 2>&1 | grep -c '^error' 
cargo fmt --all -- --check && echo "fmt clean"
```
PASS if 0 test failures, 0 clippy errors, fmt clean. WARN on clippy warnings.

## Gate 6: Coverage (≥95% line, project gate)

```bash
cargo llvm-cov --lib --summary-only 2>&1 | tail -5
```
PASS if line coverage ≥ 95%. (Slow; may SKIP with a note if llvm-cov absent.)

## Gate 7: Open issues (informational)

```bash
gh issue list --repo paiml/forjar --state open --limit 20
```
Always PASS.

## Gate 8: Safe apply→reapply→check→destroy→undo lifecycle (tempdir + /tmp only)

Using the SBX from Gate 3 (local, /tmp-file-only — SAFE):
```bash
forjar apply -f "$CFG" --state-dir "$ST" --yes 2>&1 | tail -3   # converge
test -f "$TGT" && echo "applied: file created"
forjar apply -f "$CFG" --state-dir "$ST" --yes 2>&1 | grep -iE 'unchanged|0 changed'  # C3
forjar check -f "$CFG" --state-dir "$ST" 2>&1 | tail -2          # no drift
forjar status --state-dir "$ST" 2>&1 | tail -3
forjar destroy -f "$CFG" --state-dir "$ST" --yes 2>&1 | tail -3  # tears down /tmp file
forjar undo --state-dir "$ST" 2>&1 | tail -3                     # destroy-undo roundtrip
```
PASS if converge→idempotent-reapply→no-drift→destroy→undo all behave per contract.
**Never** point this at a non-tempdir state dir.

## Gate 9: Loud-failure injection (must fail non-zero, never silently degrade)

```bash
OUT=$(forjar validate -f /nonexistent.yaml 2>&1); echo "missing file: exit=$?"
printf 'version: "1.0"\nname: x\nresources:\n  a: {type: file, machine: nope, path: /tmp/x}\n' > "$SBX/bad.yaml"
OUT=$(forjar validate -f "$SBX/bad.yaml" 2>&1); echo "unknown machine: exit=$? :: $(echo "$OUT"|tail -1)"
OUT=$(forjar plan -f "$SBX/bad.yaml" 2>&1); echo "plan on bad: exit=$?"
```
PASS if every malformed/invalid input exits non-zero with a clear message.
FAIL if any bad input is silently accepted (exit 0).

## Gate 10: Protocol checks

- **P1 Silent-flag**: `diff <(forjar status --state-dir "$ST") <(forjar status --state-dir "$ST" --json)` must differ (no no-op flag); same for `plan`/`plan --json`.
- **P2 Exit-code honesty**: any command printing `error`/`failed` must exit non-zero (`OUT=$(...);EC=$?`).
- **P3 JSON validity**: `forjar status --json`, `plan --dry-run-json`, `lock-info --json` (where supported) → `jq .` must parse.
- **P4 Idempotency observability (#129)**: `apply --force` on a converged stack prints the `note: --force re-ran N … (0 actual change(s))` line — C3 is observable through `--force`.
- **P5 dist self-verify**: `forjar dist -f dist-forjar.yaml --verify 2>&1` Tier-1 static checks PASS (sh -n, bashrs, snippet/URL structure).

PASS if all protocols hold.

## Gate 11: doctor + dist artifacts

```bash
forjar doctor 2>&1 | tail -10
forjar dist -f dist-forjar.yaml --installer -o "$SBX/install.sh" 2>&1 | tail -2 && sh -n "$SBX/install.sh" && echo "installer parses"
bashrs lint "$SBX/install.sh" 2>&1 | tail -1
```
PASS if doctor reports the toolchain and the generated installer is valid POSIX
sh with 0 bashrs errors.

---

## Verdict

After all gates, provide:
1. **Summary table**: Gate 1–11 | Status | Evidence
2. **Claims**: C1–C10 | PASS/FAIL
3. **Contracts**: 10 contracts | valid + bound + tested
4. **Command grid**: N subcommands | help-ok / phantom / panic counts
5. **GO** if Gates 1–7 pass AND Gates 8–11 have no FAIL
6. **WARN** if only soft issues (clippy warnings, coverage just under, SKIPs)
7. **FAIL** if any panic, exit-code lie, silent acceptance of bad input,
   falsified C-claim, contract violation, or non-idempotent apply

File bugs: `gh issue create --repo paiml/forjar --title "forjar <cmd>: <title>" --body "..."`

## Cleanup

```bash
rm -rf "$SBX" /tmp/forjar-dogfood-* 2>/dev/null
```
**Do not** delete or modify any real state dir, the repo, or `examples/`.
