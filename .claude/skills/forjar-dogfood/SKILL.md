---
name: forjar-dogfood
description: The forjar pre-publish go/no-go. Runs the mechanical gates (make dogfood-release) and adds the three that need judgement, then writes one deterministic receipt. Read-only.
allowed-tools: Bash(cargo:*), Bash(make:*), Bash(pmat:*), Bash(pv:*), Bash(gh:*), Bash(git:*), Bash(bash:*), Bash(find:*), Bash(head:*), Bash(tail:*), Bash(wc:*), Bash(sort:*), Bash(diff:*), Bash(timeout:*), Bash(jq:*), Bash(python3:*), Bash(echo:*), Bash(cat:*), Bash(mktemp:*), Bash(mkdir:*), Bash(cmp:*), Read, Glob, Grep, Agent
effort: high
---

# forjar-dogfood — the pre-publish go/no-go

`forjar` is Infrastructure as Code. This skill is a **read-only audit**: it
never applies to a real host, never edits the tree except the one receipt it
writes, and never closes an issue. Its whole output is
`docs/audits/dogfood-<ver>-receipt.md`.

## Why the name is `forjar-dogfood` and not `dogfood`

Claude Code resolves a skill by its frontmatter `name:`, and a user-scope skill
of the same name wins over the repo's. `~/.claude/skills/dogfood/` exists on the
machines that run this. The old repo skill was `name: dogfood`, so a contributor
asking for "the dogfood" got a different document, with no error and no diff.
`tests/falsification_dogfood_skill_is_named.rs` fails if the name reverts.

## The eight gates

| Gate | What it asserts | Where it runs |
|------|-----------------|---------------|
| **A** | The frame: HEAD is the tree under test, the release binary is this version, the receipt is reproducible | this skill |
| **B** | `pmat comply check` passes against the committed config | `scripts/dogfood/comply.sh` |
| **C** | The declared transport surface equals the surface the built binary exposes | `scripts/dogfood/surface.sh` |
| **D** | Every documented `forjar …` invocation runs, and every count README claims is the derived one | `scripts/dogfood/docs.sh` |
| **E** | One quorum receipt per PR merged since the last tag | this skill |
| **F** | Coverage ≥ 95% line, and 0 mutant survivors in the diff | `scripts/dogfood/coverage.sh` |
| **G** | Every contract validates, lints, and resolves its falsifiers to tests that run in `ci / gate` | `scripts/dogfood/contracts.sh` |
| **H** | Every `[Unreleased]` behaviour bullet has a crux row with ≥3 systems | `scripts/dogfood/crux-reconcile.sh` + judgement here |

B, C, D, F, G, H are **mechanical**: shell, no agent, `make dogfood-release`
runs all six and fails on the first RED. This skill does not re-implement them
and does not paraphrase their output — it runs the Make target and quotes the
`GATE <letter> …` line each script printed.

A, E and the second half of H need judgement and run here.

## Frame — do this first, in this order

```bash
VER=$(python3 -c "import re,io;print(re.search(r'^version = \"(.*)\"', io.open('Cargo.toml').read(), re.M).group(1))")
bash ~/.claude/skills/paiml-implement/scripts/goal.sh set --ticket "DF-$VER"
git rev-parse HEAD && git status --porcelain
```

**A fails** if the tree is dirty (a gate measured against uncommitted edits is
not measuring the release), if `git rev-parse HEAD` is not the commit the
release will be cut from, or if the release binary's `--version` is not the
`Cargo.toml` version. The scripts assert the last one themselves; A is where you
say which commit the receipt is about.

Never resolve `forjar` from `PATH`. The binary under test is
`$(cargo metadata --format-version 1 --no-deps | jq -r .target_directory)/release/forjar`
after `cargo build --release`, which is what `make dogfood` builds.

## Gate B, C, D, F, G, H — run them, do not reimplement them

```bash
make dogfood-release
```

Record, per gate, the single `GATE <letter> PASS|FAIL <detail>` line the script
printed and the exit code of the target. If the target stops at the first RED,
that is the design: the remaining gates are **NOT MEASURED**, and the receipt
must say `not measured` for them rather than carrying a stale verdict from a
previous run. An unmeasured gate is not a passing gate.

`make dogfood` is the shorter tier (B, C, D, G) — no coverage, no crux. It is
what `ci / gate` runs per commit. `make dogfood-release` is the release gate.

## Gate E — one quorum receipt per merged PR since the last tag

```bash
LAST=$(git describe --tags --abbrev=0)
WHEN=$(git log -1 --format=%cs "$LAST")
gh pr list --repo paiml/forjar --state merged --base main \
   --search "merged:>=$WHEN" --json number,title,mergedAt
ls docs/audits/quorum-*.md .quorum/ 2>/dev/null
```

**E fails** if a PR merged since `$LAST` has no receipt, or if a receipt exists
whose PR is not in the list (a receipt for a PR that never merged is not
evidence). Name every unpaired number in the receipt. Do not create the missing
receipts — that is the orchestrator's budget, not yours.

## Gate H — the judgement half

`scripts/dogfood/crux-reconcile.sh` proves the *shape*: a row exists for each
`[Unreleased]` behaviour bullet and each row cites ≥3 systems. It cannot judge
whether the row is *about* the bullet. Read `docs/audits/crux-<ver>.md` against
`CHANGELOG.md [Unreleased]` and say so.

**H fails** if the script failed, or if a row is topically unrelated to the
bullet it claims to reconcile.

## Subagents

At most **three**, all read-only, `Agent` descriptions exactly `DF/ph1`,
`DF/ph2`, `DF/ph3`. Suggested split: `DF/ph1` = E (GitHub archaeology),
`DF/ph2` = H (crux reading), `DF/ph3` = reading the `make dogfood-release`
output back against the scripts. A subagent **returns text**. The orchestrator
is the **sole writer** of the receipt; a subagent that writes to
`docs/audits/` has broken the determinism guarantee below.

## The receipt

Write exactly one file, `docs/audits/dogfood-<ver>-receipt.md`, where `<ver>` is
the `Cargo.toml` version.

Hard requirements, because the receipt is checked by re-running:

1. **No timestamps.** No dates, no durations, no elapsed-seconds, no
   `--since`-relative phrasing. A receipt that differs between two runs of the
   same tree cannot be used as evidence about that tree.
2. **Exactly one line matching `^verdict:`** — `verdict: GO`, `verdict: NO-GO`,
   or `verdict: NOT-MEASURED`. Not one per gate; one for the run. Per-gate
   status goes in the table as `PASS` / `FAIL` / `not measured`.
3. **The last line is exactly `DOGFOOD-<ver>-RECEIPT-END`.** Truncation is then
   detectable rather than silent.
4. Quote each gate's `GATE <letter> …` line verbatim. Do not summarise a number
   you did not read.

Determinism is asserted by running twice. If `--twice` is in `$ARGUMENTS`:
generate the receipt, copy it aside, run the whole gate set and regeneration a
second time, and `cmp` the two files. Report the `cmp` result. A non-identical
pair is a **NO-GO** on gate A regardless of B–H, because it means the receipt is
reporting something other than the tree.

## Verdict

- `verdict: GO` — A–H all PASS.
- `verdict: NO-GO` — any gate FAIL. Name the gate letter and the script line.
- `verdict: NOT-MEASURED` — the run could not measure a gate at all (toolchain
  absent, `make dogfood-release` did not reach it). Unmeasured is not passing,
  and it is not failing either; it is its own answer and it blocks a release
  exactly as hard.

## What this skill must not do

- No `forjar apply` / `destroy` / `undo` against any real host or state dir.
- No writes to GitHub: no branch push, no pull-request creation, no issue close,
  no destructive `gh api` call. Findings are text in the receipt.
- No edits to source, contracts, workflows, or `Cargo.toml`. File findings; the
  orchestrator budgets the tickets.
- No surface list written into this document. The surface is derived from the
  built binary by `scripts/dogfood/surface.sh` into
  `docs/audits/surface_audit.csv`; a list copied into prose is a second source
  of truth that cannot fail.

## Arguments

$ARGUMENTS
