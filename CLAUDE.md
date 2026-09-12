# Forjar Development Guidelines

## Code Search

NEVER use grep/glob for code search. ALWAYS prefer `pmat query`.

```bash
# Find functions by intent
pmat query "error handling" --limit 10

# Find with fault patterns (--faults)
pmat query "unwrap" --faults --exclude-tests

# Find coverage gaps
pmat query --coverage-gaps --limit 20 --exclude-tests
```

## Quality Gates

- **No source file over 500 lines** — split into module directories or extract tests
  - Exception: `src/cli/commands/mod.rs`, `apply_args.rs`, `status_args.rs`, `validate_args.rs` (declarative clap structs, no logic)
  - Pre-commit hook enforces complexity; `pmat comply check` enforces file health
- All functions must be TDG grade A (complexity <= 10)
- Cognitive complexity per function <= 25 (pre-commit enforced)
- Minimum 95% line coverage (`cargo llvm-cov`)
- Zero clippy warnings (`cargo clippy -- -D warnings`)
- Never use `cargo tarpaulin` — use `cargo llvm-cov` instead

## Testing

```bash
cargo test                              # Run all tests
cargo llvm-cov --summary-only           # Check coverage
cargo clippy -- -D warnings             # Lint check
```

## Pre-publish gate

**Nothing is published until `make dogfood-release` exits 0.** It is the release
blocker, and it is the human-run half of the pair the clean-room CI lane is the
machine-run half of: the clean room proves the artifact BUILDS from a cold tree,
`dogfood-release` proves the artifact DOES WHAT THIS REPO SAYS IT DOES.

```bash
make dogfood            # B C D G — hermetic, cheap; run it on every commit
make dogfood-release    # + A E T F H — the pre-publish gate
make dogfood-published VERSION=x.y.z   # C and D against what crates.io serves
make release-check      # post-tag: tag, release, crates.io, docs.rs, receipts
```

| gate | asks |
|---|---|
| A `harness.sh` | every PR merged since the newest `v*` tag maps to a ticket whose `docs/audits/impl-<ticket>-receipt.md` exists at HEAD, ends with `IMPL-<ticket>-RECEIPT-END` and has exactly one `verdict:` line; gh unreachable or a PR with no ticket is UNMEASURED |
| B `comply.sh` | `pmat comply` against the committed `.pmat.yaml`, with a stronger instrument in place of each disabled check |
| C `surface.sh` | the CLI/MCP/HTTP surface, measured from the running artifact: declared vs live, diffed against `docs/audits/surface_audit.csv` |
| D `docs.sh` | every fenced `forjar …` block in README.md, run against a fixture in a sandboxed HOME |
| E `quorum.sh` | every PR merged since the newest `v*` tag has an unwaived `.quorum/<branch>.json` at HEAD: >=3 lanes, judges and refuters per claim, >=1 refuted claim, evidence files, no waiver or override key anywhere (the same predicate `release-check.sh` applies after the tag) |
| F `coverage.sh` | the 95% line floor enforced inside llvm-cov, plus `cargo mutants` over this branch's own diff |
| G `contracts.sh` | the contract corpus validates, lints, has depth, and every citation resolves |
| H `crux-reconcile.sh` | every behaviour bullet under CHANGELOG `[Unreleased]` has a `docs/audits/crux-<ver>.md` row naming >=3 world-class systems |
| T `tagged.sh` | every tagged release since the floor has a row in `docs/roadmaps/releases.yaml` whose cut, PRs and tickets are what git and GitHub say; every shipped ticket carries `release:<tag>` and says `status: completed`; every ticket merged since the newest tag carries `release:<next.tag>`; from `cookbook_floor` every release names the paiml/forjar-cookbook commit it was qualified against, whose `Cargo.toml` must admit the version that shipped under Cargo's caret rule (`forjar = "1.2"` is `>=1.2.0, <2.0.0`) **and whose `Cargo.lock` must PIN it** — a requirement is a range, the lock is what cargo builds, and the first commit any release named locked 1.2.1 while the release was 1.29.0. **The cookbook is bumped and pushed as part of the cut, before the tag, and the cut names the resulting commit**; while a cut is in flight the CHANGELOG's own PR and ticket counts must equal what the window measures; and the next cut is not overdue (`cadence_days`, 2); `make release-goal` prints the join as one status line |
| R `release-check.sh` | the tag, the release, crates.io, docs.rs, a committed `.quorum/<branch>.json` receipt (unwaived, >=3 lanes, >=3 judges) per merged PR |

The operator-facing procedure is the **`forjar-dogfood` skill**
(`.claude/skills/forjar-dogfood/SKILL.md`) — that name, never `dogfood`: a
user-scope skill of the same name silently wins, and
`tests/falsification_dogfood_skill_is_named.rs` exists to keep the collision
from coming back.

Every gate prints exactly one `GATE <letter> PASS|FAIL <detail>` line, its exit
code is the verdict, and it carries a trailing `# mutation:` comment naming the
one-line change that turns it RED. That comment is the point: a green shell gate
proves nothing on its own, so each one ships with the address of its own
falsifier, and `tests/falsification_dogfood_scripts_declare_mutations.rs`
asserts the address exists, that the script is strict, and that no `|| true`
swallows a measurement. `contracts/forjar-dogfood-coverage-v1.yaml` records the
whole set.

**Never `--skip`, never `|| true` on a measurement, never lower a floor.** An
UNMEASURED check is a FAILING check: a gate that could not run and a gate that
passed must never print the same thing.

## Architecture

- `src/core/` — Config parsing, planning, execution, state management
- `src/resources/` — Resource handlers (package, file, service, mount)
- `src/transport/` — Local and SSH execution
- `src/tripwire/` — Drift detection, hashing, event logging
- `src/cli/` — CLI commands and argument parsing
