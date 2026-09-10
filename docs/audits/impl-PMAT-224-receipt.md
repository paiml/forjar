# Implementation receipt — PMAT-224 — a shape the committed-quorum gate lets a kind:triage branch satisfy honestly

verdict: DONE — a receipt may declare `kind: triage`; the gate verifies the triage rail from the diff itself, requires the falsification block to say `not_applicable` and prints what it did not verify, and documentation the branch touches anchors its claims for that kind only. Closes forjar#491 on this repository's side; the rail's other half is filed as paiml/paiml-implement#68.

## Identity

| field | value |
|---|---|
| ticket | PMAT-224 (kind: code, `orch:fable`, `orch-basis:M>=3`) |
| issue | forjar#491, filed from the closure of #486 |
| branch | PMAT-224-quorum-gate-triage-shape |
| base | be863153 (the squash of PR #504) |
| commits | 3cc65103 roadmap · 7dad1e47 the RED fixture test · 9da1f848 the fix and the complexity extraction · 15418d23 the spec, CHANGELOG and contract rows · the receipt and evidence · the quorum receipt |
| discover.json sha256 | 453198278ff3bfbc05c32dfe4920681e4b22b9b60ba15c4429cb499b2a5daa3a |
| gate_cmd | `cargo test --workspace` — `gate_cmd_fallback=true`, said in the first status block |
| model gate | `model=fable class=fable decision=admit basis=file`; tier 1 meets tier 1 |
| session | the SECOND ticket in Claude session 94370bba (PMAT-223 was the first). `goal.sh set` REFUSED it — exit 2, `one ticket per session: PMAT-223 was set here — start a new claude session` — and refused again after `goal.sh clear`. I named the rule at the end of PMAT-223; Noah's next message was, verbatim, `continue autonomously using pmat-implement`, and the ticket proceeded on that, exactly as PMAT-222 did on the same words. The R-5 ledger was not touched; the lanes were declared with `goal.sh worker` as usual |
| status-line join | `[U]` — not measured this run; every dispatch was declared with `goal.sh worker` first and every Agent description began with `PMAT-224/ph<i>` |
| k_measured vs global | no `k_measured_at_set` (the set was refused); by the transcript instrument `k_measured` was 45 at PMAT-223's receipt commit, 64 at this ticket's first status block and 75 at this receipt — 30 turns for the whole ticket; my own count is higher because the instrument collapses a multi-tool turn into one id, the gap named here as the doctrine asks |

orch_model: fable [A]   orch_class: code   orch_decision: admit   orch_basis: M>=3
fable_binding: true   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=cross  route=agy-goal  w=1.00  basis=absent  effort=1[U]  (overridden to direct: bash + python + one Rust test; agy write lanes barred by the recorded hazard)
  ph1.grill  class=plan  route=agy-plan  w=1.00  basis=absent  effort=1[U]  (delegate teamwork, read-only)
  ph2  class=mechanical  route=agy-goal  w=1.00  basis=absent  note=fable-binding  effort=1[U]  (overridden to direct: spec, CHANGELOG, contract)
  ph3  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate quorum width 3)
  ph4  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_quorum_gate_has_a_triage_shape (pre-fix tree, 6 cases)"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-224-red.log  sha256=5a7eda1d6f0cb155
  cmd="cargo test --test falsification_quorum_gate_has_a_triage_shape (fixed tree)"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-224-green.log  sha256=b6b10e70e7958b44
  cmd="mutation 1: citation_shape never widens — the triage anchor case"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-224-mutation-1.log  sha256=17de1ab02e995aa3
  cmd="mutation 2: the triage falsification arm removed — the whole-gate case"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-224-mutation-2.log  sha256=987aa8d5414135d4
  cmd="mutation 3: the rail check emptied — the code-diff case"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-224-mutation-3.log  sha256=38a856b4c26f00cb
  cmd="cargo clippy --all-targets -- -D warnings"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-224-clippy.log  sha256=315dffa97429049b
  cmd="bash scripts/dogfood/contracts.sh (gate G)"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-224-gate-G.log  sha256=d09e4037f4df3bd7
  cmd="bashrs lint scripts/quorum-gate.sh (0 errors, 15 warnings; the same on main; exit 1 is bashrs's warning exit)"  claimed_exit=0(lanes)  rerun_exit=1  log_path=docs/audits/logs/PMAT-224-bashrs.log  sha256=1c96f80e3426b112
  cmd="cargo test --workspace --no-fail-fast at 15418d23"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-224-workspace-green.log  sha256=d819f77157befd8e

Each log under docs/audits/logs/ is the reduced form (head, verdict lines, tail) of the full log, whose byte count and sha256 are on its first line.

## The defect and the fix

`scripts/quorum-gate.sh` required a falsification test the branch wrote or changed, and `scripts/quorum_evidence.py` counted a claim as anchored only when it cited a Rust file or a root manifest the branch touched. A `kind:triage` branch — classify and link, no diff — touches only `docs/audits/**` and the roadmap: it had no test to name and nothing citable, so it anchored 0% of its claims by construction and could not satisfy the falsification block at all. PR #486 was pushed `waived`; ten waived receipts sit in `.quorum/` on main. The `CIT_RE` comment already named the mechanism for release commits: a gate that cannot be passed honestly is what teaches a repo to reach for the waiver.

The fix gives the receipt a `kind`. For `kind: triage` the gate verifies the rail FROM THE DIFF — `docs/audits/**`, `docs/roadmaps/roadmap.yaml`, `.quorum/**`, anything else refused by name — requires `falsification.not_applicable` with a reason, refuses a receipt that also names a test, and prints NOT APPLICABLE with the reason so an unmeasured check never reads like a passed one; the evidence pass still runs. `quorum_evidence.py` anchors a citation into documentation the branch touches only for that kind, under the unchanged at-base, as-added and must-be-touched rules, so a code branch cannot anchor its claims on the receipt it wrote itself. A receipt with no kind is a code receipt and its path is what main had.

## What review changed

- **The plan widened the citation shape for every receipt.** The grill lane refused that: a code branch always touches the receipt and ledger it writes. Documentation anchors for `kind: triage` only, and the rail refusal makes declaring the kind useless for a code diff.
- **The plan's triage arm bypassed too little.** The falsification block's first loop requires `test`, `reverted` and `observed_failure`; the whole block is replaced by the `not_applicable` shape for triage.
- **The hook refused the fix on debt already on main.** `check_manifest` and `check_claims` measured 90 and 83 cognitive against a limit of 25 at be863153; the hook measures the whole touched file. Seven flat helpers were extracted, proven behaviour-preserving by the fifteen cases across the three gate binaries. Lane 2 refuted the word "strictly": `receipt_identity` also carries the new guard for an unknown `kind`, a message main could not print on a field it did not read. The digest says so.
- **Three review lanes: 3/3 PASS, lane-reduce agreed.** Lane 3's refutation attempts (the `docs/audits.txt` edge of the rail, `sys.exit(0)` inside the python block, the order of operations in the helpers) are the useful ones; each held.

## Verification, all my own runs

| command | claimed | mine |
|---|---|---|
| the six fixture cases on the pre-fix tree | — | **3 failed** (`only 0/4`; `falsification is missing 'test'` ×2), 3 passed (the controls) |
| the six cases on the fixed tree; the two sibling gate binaries | — | 6 passed; 4 + 5 passed |
| three mutations by hand | lanes: confirmed by reading the logs | each named case FAILED under its mutation, 6/6 after restore |
| `bashrs lint scripts/quorum-gate.sh`, main and branch | lanes: measured | 0 errors, 15 warnings, both |
| `pmat` pre-commit complexity | — | refused the first fix commit (90 / 83 / 27 > 25); accepted the extracted file |
| `cargo clippy --all-targets -- -D warnings` | — | exit 0 |
| `rustup run stable cargo fmt --all -- --check` | — | exit 0 |
| `scripts/dogfood/contracts.sh` (gate G) | — | PASS: 40 contracts validate, citations resolve |
| `cargo test --workspace --no-fail-fast` at 15418d23 | — | **exit 0, 317 binaries, 19,624 passed, 0 failed, 58 ignored** |
| the lanes' grep claims (no reader of `falsification.*` outside the gate; contract citations exist by name) | lanes: confirmed | agree |

Mutations observed RED: the three by hand above, one per hunk, each with its reduced log committed. Gate F's mutation arm cannot run on this host (PMAT-216); no `cargo mutants` figure is claimed.

## Dispatch ledger

| dispatch | mode | agent | lane | width | turns | maxTurns hit | resumed | conversations |
|---|---|---|---|---|---|---|---|---|
| PMAT-224/ph1.grill | delegate | ab5b21d53 | teamwork | 1 | 24 | no | no | conv-048ea2e6 (children unknown) |
| PMAT-224/ph3.review | delegate | a92f7d2b5 | quorum | 3 | 21 | no | no | conv-e96ec28b, conv-0bf1d1c3, conv-fe39eeb3 |

Slots used: at most 1 of 3 at any instant. Denials: 0. I-3: PASS attempted=7 denied=0 running_peak=1 slots=3 (five delegate dispatches on PMAT-223 and two on PMAT-224 in this session; no worker subagent, no resume, no Workflow). Route lines above are `route.sh`'s output verbatim; `q=?` because `quota.json` is absent.

## Jidoka

One row appended to docs/audits/jidoka.jsonl: the pre-commit hook's refusal of the fix on cognitive complexity that was already on main, paid down by extraction in the same commit. Blocking, same repo, no bypass.

## Estimates

`K̂=4 basis=first-run[U]` (`estimate.sh` matched no rows under this repo key, ROWS=0); `K=80` declared against docs/audits/impl-estimates.jsonl:L15-L19 (PMAT-223's rows) — declared in the refused `goal.sh set` line, so the status line never carried it. Actual by the transcript instrument: 45 at PMAT-223's receipt, 64 at this ticket's first status block, 75 at this receipt — 30 turns. Rows appended.

## Gaps

- **The triage rail's other half** lives in the paiml-implement skill (`kind-gate.sh` admits only `docs/audits/**` and the roadmap) — filed as paiml/paiml-implement#68 with the one-line change. Until it lands, a triage branch that satisfies this gate fails the skill's DoD rail; the two then name the same three paths.
- **A scripts-only branch still cannot anchor on its scripts.** `CIT_RE` names `.rs` files and root manifests; this very branch changed `scripts/quorum-gate.sh` and `scripts/quorum_evidence.py` and anchors its digest on the Rust fixture test and the CHANGELOG instead. The same shape as #491 one file-type over; not widened here, named for the next pass at the citation rule.
- **pv lane: NotRun as a separate lane;** the corpus is exercised by gate G in the same PR.
- **Gate F mutation arm: not measured on this host (PMAT-216);** closed for this diff by the three hand mutations.
- **Status-line join `[U]`;** the R-5 refusal also means no goal record for this ticket.

IMPL-PMAT-224-RECEIPT-END
