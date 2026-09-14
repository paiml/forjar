---
name: forjar-dogfood
description: The forjar pre-publish go/no-go. Runs the mechanical gates (make dogfood-release), reads their output, and writes one deterministic receipt. Read-only, no subagents.
allowed-tools: Bash(cargo:*), Bash(make:*), Bash(pmat:*), Bash(pv:*), Bash(gh:*), Bash(git:*), Bash(bash:*), Bash(find:*), Bash(head:*), Bash(tail:*), Bash(wc:*), Bash(sort:*), Bash(diff:*), Bash(timeout:*), Bash(jq:*), Bash(python3:*), Bash(echo:*), Bash(cat:*), Bash(mktemp:*), Bash(mkdir:*), Bash(cmp:*), Read, Glob, Grep
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

## The eight gates, and T

| Gate | What it asserts | Where it runs |
|------|-----------------|---------------|
| **A** | Every PR merged since the last tag carries `docs/audits/impl-<ticket>-receipt.md`, ending in its END marker, with exactly one verdict — and, from `TRAILER_FLOOR`, is filed under a ticket its own merge commit does not CONTRADICT: if the commit claims any `Pmat-Ticket:`, the PR's ticket must be among them (PMAT-540) | `scripts/dogfood/harness.sh` |
| **B** | `pmat comply check` passes against the committed config | `scripts/dogfood/comply.sh` |
| **C** | The declared transport surface equals the surface the built binary exposes | `scripts/dogfood/surface.sh` |
| **D** | Every documented `forjar …` invocation runs, and every count README claims is the derived one | `scripts/dogfood/docs.sh` |
| **E** | Every PR merged since the last tag carries `.quorum/<slug>.json` at the quorum floor, with no waiver anywhere in it | `scripts/dogfood/quorum.sh` |
| **F** | Coverage ≥ 95% line, and 0 mutant survivors in the diff | `scripts/dogfood/coverage.sh` |
| **G** | Every contract validates, lints, and resolves its falsifiers to tests that run in `ci / gate` | `scripts/dogfood/contracts.sh` |
| **H** | Every `[Unreleased]` behaviour bullet has a crux row with ≥3 systems | `scripts/dogfood/crux-reconcile.sh` |
| **T** | Every tagged release since the floor has a row in `docs/roadmaps/releases.yaml` whose cut, PRs and tickets are what git and GitHub say; every shipped ticket carries `release:<tag>` and says `status: completed`, every ticket merged since the newest tag carries `release:<next.tag>`, the next cut is not overdue, from `cookbook_floor` every release names a paiml/forjar-cookbook commit whose `Cargo.toml` admits AND whose `Cargo.lock` pins the released version, and while a cut is in flight the CHANGELOG's own PR and ticket counts equal what the window measures (PMAT-225, PMAT-236, PMAT-241, PMAT-520, PMAT-537) | `scripts/dogfood/tagged.sh` |

All nine are **mechanical**: shell, no agent. `make dogfood-release` runs
every one of them and fails on the first RED. T is the release-goal gate: the
declared side is `docs/roadmaps/releases.yaml` and the `release:<tag>` labels
on the roadmap rows, the measured side is git and GitHub, and
`make release-goal` (`scripts/release-goal.sh show`) prints the join as one
status line — `<next tag> <bar> <elapsed>h/<cadence>h left=<h> · <merged>
merged, <tagged> tagged · due <instant> basis=…`. After a tag,
`scripts/release-goal.sh cut <tag> --next <next>` writes the row and moves the
labels; before one, `scripts/release-goal.sh sync` labels the open window.

**The cookbook is part of the release, not a downstream of it (PMAT-241).**
paiml/forjar-cookbook is where forjar is USED rather than described: gate D
validates every one of its configs against the built artifact, and `make
dogfood-published VERSION=x.y.z` does it against what crates.io actually
serves. Nothing recorded which cookbook that was, and the cookbook's master had
not moved since 2026-08-29 while ten tags went out claiming to be dogfooded
against it. From `cookbook_floor` every ledger row names the cookbook commit
the release was qualified against — `release-goal.sh window <tag>` takes it
from `git ls-remote … refs/heads/master` at the moment of the cut, so `cut`
books it — and gate T refuses a row that names none, names a branch instead of
a commit, names a commit the cookbook does not carry, or names one whose
`Cargo.toml` cannot admit the version that shipped — read as CARGO reads it:
`forjar = "1.2"` is a caret, `>=1.2.0, <2.0.0`, so a plain `>=` would pass
2.0.0 against a cookbook that cannot build with it.

**And the requirement is not the measurement — the LOCK is (PMAT-537).** A
requirement is a range; `cargo` compiles what `Cargo.lock` pins. Measured on
the first commit any release ever named: the manifest said `1.2` and the lock
said **1.2.1**, so the gate blessed a cookbook compiled twenty-seven minors
behind the release it was recorded as qualifying. Gate T reads both now and
refuses a row whose cookbook lock is not exactly the released version; a lock
it cannot read, or one with no forjar in it, is UNMEASURED and red.

**So the cookbook is bumped and pushed as PART OF THE CUT, before the tag**,
and the cut names the resulting commit. That is an obligation on a second
repository and it is stated here rather than implied. It was done the other way
round exactly once — for 1.29.0, where the cookbook was bumped after the tag
and the row corrected — and the receipt for that says so. This skill does not re-implement
a gate, does not decide a gate, and does not paraphrase a gate's output — it
runs the Make target and quotes the `GATE <letter> …` line each script printed.

A and E were prose here until PMAT-201, discharged by an agent that read GitHub
and formed a view. An agent that forms a view is not a gate: it has no exit
code, it does not run in CI, and it cannot be shown to go red. They are now
`scripts/dogfood/harness.sh` and `scripts/dogfood/quorum.sh`, sharing one
window in `scripts/dogfood/lib/window.sh`, with falsifiers in
`tests/falsification_dogfood_harness_and_quorum.rs`.

**The promotion to a full release is a STEP, and gate R checks it (PMAT-534).**
A release is born a prerelease so no consumer sees a half-uploaded asset set,
and GitHub never makes a prerelease `latest`. Clearing the flag does NOT move
the pointer — `make_latest` is fixed when the flag is written — so the promotion
must say `--latest`:

```bash
gh release edit v<ver> --repo paiml/forjar --prerelease=false --latest
```

Measured 2026-09-12: `repos/paiml/forjar/releases/latest` resolved to v1.25.2
while v1.26.0, v1.27.0 and v1.28.0 were all full releases days newer, because
that step existed only as one line of a workflow's output. `make release-check`
now refuses a full release the URL does not point at, and reports a prerelease
with the command that ends it.

## Frame — do this first, in this order (this is not a gate)

```bash
VER=$(python3 -c "import re,io;print(re.search(r'^version = \"(.*)\"', io.open('Cargo.toml').read(), re.M).group(1))")
bash ~/.claude/skills/paiml-implement/scripts/goal.sh set --ticket "DF-$VER"
git rev-parse HEAD && git status --porcelain
```

**The whole run is `NOT-MEASURED`** if the tree is dirty (a gate measured
against uncommitted edits is not measuring the release), if `git rev-parse HEAD`
is not the commit the release will be cut from, or if the release binary's
`--version` is not the `Cargo.toml` version. The scripts assert the last one
themselves. The frame is where you say which commit the receipt is about; it
decides nothing else.

Never resolve `forjar` from `PATH`. The binary under test is
`$(cargo metadata --format-version 1 --no-deps | jq -r .target_directory)/release/forjar`
after `cargo build --release`, which is what `make dogfood` builds.

## Gates A–H — run them, do not reimplement them and do not re-decide them

```bash
make dogfood-release
```

Record, per gate, the single `GATE <letter> PASS|FAIL <detail>` line the script
printed and the exit code of the target. If the target stops at the first RED,
that is the design: the remaining gates are **NOT MEASURED**, and the receipt
must say `not measured` for them rather than carrying a stale verdict from a
previous run. An unmeasured gate is not a passing gate.

`make dogfood` is the shorter tier (B, C, D, G) — no coverage, no crux, no
GitHub window. `ci / gate` does NOT run it: its `dogfood` job runs gates C and
D and the guard tests directly (B and G need pmat, gh, bashrs and pv, which the
release host and the clean room have and a hosted runner does not).
`make dogfood-release` is the release gate: it adds A and E (seconds, so they
run first), then F and H.

## Gates A and E — what the two window gates read, so you can read their output

Both ask GitHub for the PRs merged into `main` since the newest `v*` tag
reachable from HEAD, keep the ones whose merge commit is an ancestor of this
HEAD, and check one file per PR AT HEAD:

- **A**, `scripts/dogfood/harness.sh`: the ticket id is the first `PMAT-<n>` in
  the branch, then the title, then the body; the receipt is
  `docs/audits/impl-<ticket>-receipt.md`; its last line must be
  `IMPL-<ticket>-RECEIPT-END` and exactly one line must match `^verdict:`.
- **E**, `scripts/dogfood/quorum.sh`: the receipt is the COMMITTED
  `.quorum/<slug>.json`, where `<slug>` is the PR's head branch with every `/`
  replaced by `-` — the same file `scripts/quorum-gate.sh` refuses a push
  without. `docs/audits/quorum-<pr>.md` names nothing real; do not look for it.

A GitHub client that cannot answer, or a page that fills the `--limit`, is
UNMEASURED and both gates FAIL saying so — that is not a window with no PRs.

## Gate H — the script is the verdict

`scripts/dogfood/crux-reconcile.sh` decides H: a row exists for each
`[Unreleased]` behaviour bullet and each row cites ≥3 systems. Quote its line.
If you believe a row is topically unrelated to the bullet it claims to
reconcile, that is a FINDING for the receipt and a ticket for the orchestrator
— it is not a verdict you may substitute for the script's, in either
direction.

## No subagents, no re-deciding

Every gate is a script with an exit code, so there is nothing here to delegate:
this skill spawns no subagent and is the sole writer of the receipt. Do not
create a missing receipt for a PR, and do not argue a FAIL down — quote the
line, name the PR, and let the orchestrator budget the ticket.

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
pair is a **NO-GO** regardless of A–H, because it means the receipt is
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
- No subagents. Every gate is a script with an exit code; there is nothing here
  for a subagent to decide, and a delegated verdict is a verdict with no
  falsifier.
- No surface list written into this document. The surface is derived from the
  built binary by `scripts/dogfood/surface.sh` into
  `docs/audits/surface_audit.csv`; a list copied into prose is a second source
  of truth that cannot fail.

## Arguments

$ARGUMENTS
