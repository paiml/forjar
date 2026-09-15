# Implementation receipt — PMAT-565 — the lock names the binary that wrote it

verdict: PASS — every write of a per-machine lock stamps `generator` with the writing binary (what `forjar --version` prints) and keeps the value it replaces as `created_by`, once; `lock-repair` and `lock-migrate`, which wrote with a bare `fs::write` and left the `.b3` sidecar stale, go through the same writer; `forjar lock --restamp` converges every `<machine>/state.lock.yaml` under a state dir in one run. Six cases through the writer and the binary; four mutations run, each killing what it names.

## Identity

- ticket PMAT-565 (forjar#565, milestone 1.31.0); kind: code
- branch `PMAT-565-lock-writer-provenance`, stacked on `PMAT-564-drift-declines-on-empty-scope`, to be rebased onto `main` in order after #563 and the #564 PR merge
- discover.json: sha256 6f86fa8d3580dad9 (discovery ran once in the main checkout; the worktree inherits it) — `gate_cmd_fallback=true`
- status-line join: not measured on this ticket (one goal declaration per session)

## What was measured

    state/yoga/state.lock.yaml   generator: forjar 1.1.1    generated_at: 2026-09-15T09:19:01Z
    state/gx10/state.lock.yaml   generator: forjar 1.13.1   generated_at: 2026-09-15T09:18:56Z

Two files rewritten in the same minute by one 1.30.0 binary, naming two
different writers (paiml/infra#605, third signature). `state::new_lock`
stamped `generator` once; `executor::finalize_machine` refreshed
`generated_at` on every apply; nothing refreshed `generator`.

## Plan and routing

| phase | route (route.sh, verbatim) | executor |
|---|---|---|
| 1 RED test + field | `route=agy-goal w=1.00 basis=absent note=fable-binding effort=1[U] bucket_collision=true` | direct |
| 2 stamp, restamp, contract, docs | same | direct |
| 3 review | `route=agy-quorum w=1.00 basis=absent effort=1[U]` | delegate, quorum ×3 |
| 4 receipt, push, PR | `route=self w=100.00 basis=absent` | self |

## Dispatch ledger

| dispatch | mode | agent | turns | maxTurns | resumed | lanes / conversations |
|---|---|---|---|---|---|---|
| PMAT-565/ph3.delegate (1st) | paiml-agy-delegate (opus) | — | 0 | — | no | interrupted by the operator before any lane launched |
| PMAT-565/ph3.delegate | paiml-agy-delegate (opus) | a600e3d4 | 20 | no | no | quorum ×3: conv-f90c4be0 FAIL, conv-b2b2e525 PASS, conv-9c9d7a67 FAIL; child_conversations=3 |

slots used: 1 of 3 at peak · denials: 0 · I-3: attempted=1 (the interrupted dispatch never started a subagent) denied=0 running_peak=1 slots=3.

## Verification (claimed vs re-run)

| check | claimed | re-run here |
|---|---|---|
| acceptance suites | lane 1: pass; lanes 2, 3: silent | 6/6 in the suite; lib `core::state`, `cli::lock_restamp` green |
| any StateLock write bypasses save_lock | lanes 1, 3: yes (repair, migrate); lane 2: no | lanes 1 and 3 right — `lock_repair.rs` ×2, `lock_audit.rs` migrate; fixed |
| restamp scope | lane 1: gap vs the CHANGELOG sentence | the sentence was wrong; scope now stated |
| `cargo test --workspace --no-fail-fast` | — | 341 targets, 19,796 passed, 0 failed (after the repair/migrate change) |
| clippy / fmt / check | — | clean / clean / clean |
| gate G | — | PASS after the contract stopped using a token the ratchet reads as a verb |

## Falsification

`tests/falsification_lock_names_its_writer.rs`: a write stamps the real
writer and keeps the creator; the creator is set once and the writer every
time; a legacy lock migrates its stale generator into `created_by`; apply
rewrites the writer on a forged legacy lock; `lock --restamp` converges three
locks, `--dry-run` writes nothing, a second run restamps 0, sidecars present;
`lock-repair` writes through the writer, sidecar included. RED 5/5 before the
stamp existed; the sixth case was added with the fix the review forced.

| mutation | kills |
|---|---|
| M1 serialise the raw lock in save_lock | 6 of 6 |
| M2 roll `created_by` on every write | the set-once case |
| M3 write on `--dry-run` | the restamp case |
| M4 bare `fs::write` in lock-repair | the repair case |

Each ran against the committed tree and was restored with `git checkout HEAD --`.

## Jidoka

- RED (review): two StateLock writers bypassed `save_lock`. Five-whys: a
  repaired lock named `forjar-repair` forever ← `lock-repair` wrote with
  `fs::write` ← the lock subcommands were written one file each, each owning
  its own IO ← `save_lock` was the atomic-write contract's function, not a
  project rule ← nothing enforced "one writer". Fixed at the three sites; the
  contract enumerates every writer and names the two byte-copying exceptions.
  Not filed: a lint for `fs::write` of a lock path would be the mechanism,
  and is a ticket of its own if it recurs.

## Estimates

K̂=8 [U], K=16; actual: 14 orchestrator turns from mint to this receipt. `basis=first-run[U]`.

## Gaps, named

- **`lock-restore` and `lock-tag` copy bytes**, so they do not stamp; the
  next stamping write moves whatever they left into `created_by`.
- **`lock --restamp` walks one level.** A nested state dir is restamped by
  naming it; `forjar.lock.yaml` is restamped by every apply; `.yaml.age`
  locks are not touched.
- **The fleet is not restamped by this PR.** That is one command per state
  dir after the release reaches the boxes — part of the post-publish apply,
  not of this branch.
- **Gate B is main's red** (CB-2112, CB-2115); this branch is equal or better.

IMPL-PMAT-565-RECEIPT-END
