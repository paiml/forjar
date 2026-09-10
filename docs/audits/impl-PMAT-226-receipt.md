# Implementation receipt — PMAT-226 — release: forjar 1.28.0, the first cut under the two-day cadence, and the triage rail admits the release ledger

verdict: PASS — `make dogfood-release` exit 0 on 568dc169 with all nine gates (A B C D E F G H T) green in one run, line coverage 96.45% inside llvm-cov and gate F's mutation arm a measured zero (no `src/` change); CHANGELOG `[1.28.0]` carries nine behaviour paragraphs and `docs/audits/crux-1.28.0.md` a roster row for each (gate H 9 of 9); the one line of code, the triage rail admitting `docs/roadmaps/releases.yaml` by file, is red against main's gate script (7 passed, 1 failed) and green here (8 passed); three quorum lanes confirmed every code claim 3/3 and refuted two claims on their wording, both corrected and recorded; gate R pre-tag PASS with its four post-tag arms PENDING by name. Not measured: cargo mutants on this host (PMAT-216, nothing to mutate on this branch); the tag, the GitHub release, crates.io and docs.rs are the next step, not this receipt's claim.

## Identity

| field | value |
|---|---|
| ticket | PMAT-226, kind:code, orch:fable (`orch-basis:release`), roadmap row minted with `pmat work add`, patched textually, label `release:v1.28.0` applied at mint time |
| issue | none filed — a release cut; the goal it discharges is `docs/roadmaps/releases.yaml`'s `next: v1.28.0 due 2026-09-10T22:10:01Z` (PMAT-225, forjar#506) |
| branch | `PMAT-226-release-1.28.0` on main `191770fa` |
| commits | `a183f21a` the rail, its two tests, the roadmap row; `97e72ce6` Cargo.toml/Cargo.lock/README bump and CHANGELOG `[1.28.0]`; `568dc169` `docs/audits/crux-1.28.0.md`; then the receipts, evidence and artifact |
| session | the fourth ticket of one Claude session (PMAT-223, PMAT-224, PMAT-225, PMAT-226). `goal.sh set` refused it (R-5, one ticket per session, naming PMAT-223); it proceeded on the user's verbatim instruction — `continue autonomously using pmat-implement: ensure all tickets/work are linked to tagged releases using "pmat goal" style and ensuring frequent releases/dogfood after every 2 days (tickets/pull requests must be tagged/triaged continuously)` — quoted in the roadmap row's notes; `goal.sh worker` declared both lanes |
| turns | k measured from the transcript (distinct assistant message ids): 123 at the PMAT-225 receipt, 174 at the dogfood verdict; rows in `docs/audits/impl-estimates.jsonl`; `estimate.sh` had no rows for this shape (K̂=5, `first-run[U]`) |

orch_model: fable [A]   orch_class: code   orch_decision: admit   orch_basis: release
fable_binding: true   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=agy-goal  w=1.00  basis=absent  note=fable-binding  effort=1[U]  (overridden to direct: a one-line predicate, two test cases and prose)
  ph3.crux  class=research  route=agy-grillme  w=1.00  basis=absent  effort=1[U]  (delegate teamwork width 1, read-only)
  ph4.quorum  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate quorum width 3, read-only)
  ph5  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="make dogfood-release on 568dc169 (A B C D E F G H T, one run)"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-226-dogfood-release.log  sha256=ddd38db8e31c052a
  cmd="cargo test --test falsification_quorum_gate_has_a_triage_shape with main's scripts/quorum-gate.sh checked out (RED: 7 passed, 1 failed), then at HEAD (GREEN: 8 passed)"  claimed_exit=101(lanes)  rerun_exit=101/0  log_path=docs/audits/logs/PMAT-226-gate-tests.log  sha256=2fcc08270c0e06d6
  cmd="bash scripts/dogfood/crux-reconcile.sh (gate H, 9 of 9)"  claimed_exit=0(lanes)  rerun_exit=0  log_path=docs/audits/logs/PMAT-226-gate-H.log  sha256=49f51bf8560ac90b
  cmd="bash scripts/dogfood/tagged.sh (gate T, cut in flight)"  claimed_exit=0(lanes)  rerun_exit=0  log_path=docs/audits/logs/PMAT-226-gate-T.log  sha256=0149f1888d5148b2

## What was measured before a line was written

- Gate T on merged main after PR #507: `GATE T PASS … 9 of 9 PR(s) merged since v1.27.0 carry release:v1.28.0; due 2026-09-10T22:10:01Z, 8h left`. `release-goal.sh show`: `v1.28.0 ████████░░ 39h/48h left=8h · 9 merged, 9 tagged`. `sync --check`: every ticket `already`.
- The ledger row for a tag cannot be written before the tag exists: its cut instant is the tagger date and gate T's T2 is red for a row naming a tag origin lacks. So every release is two PRs — the cut and the booking — and the booking is classify-and-link with no code in it.
- The booking had no honest shape under the quorum gate: `docs/roadmaps/releases.yaml` was outside the `kind: triage` rail from #491 (`docs/audits/**`, `docs/roadmaps/roadmap.yaml`, `.quorum/**`), and for a `kind: code` receipt `CIT_RE` anchors only `.rs` files and root manifests, none of which a booking touches. PMAT-224 predates PMAT-225's ledger; this is the gap between them.
- Gate F's mutation arm counts the mutable set (`src/`), not the changed set (`scripts/dogfood/coverage.sh:154-171`): a release branch that touches no `src/` file passes that arm by measurement. PMAT-216 (cargo mutants dies on this host) therefore does not block this cut and stays open.
- Four merged PRs had no CHANGELOG paragraph: #493 (PMAT-215), #496 (PMAT-219), #498 (PMAT-220), #500 (PMAT-221); #494 (PMAT-217) is test-only. Written here from the PR bodies and the code; compared to both by all three review lanes.
- A coverage warm-up started on main before the branch existed went red under the bump (`tests/falsification_version_matches_manifest.rs` compared a binary built at 1.27.0 with a manifest at 1.28.0). The real run happened once, on the committed tree, with the tracked tree clean. Recorded in `docs/audits/jidoka.jsonl`.

## The shape

- **The cut.** Cargo.toml `1.28.0` (`Cargo.toml:3`), Cargo.lock's forjar entry (`Cargo.lock:1171`, `cargo metadata --locked --offline` exit 0), README's two caret requirements `1.28` (`README.md:96`, `:98`; gate D's version-claim arm green). CHANGELOG: `[Unreleased]` left empty at the top; `[1.28.0] - 2026-09-10` with a non-bold introduction naming the census (nine PRs, nine tickets, every one labelled) and nine bold behaviour paragraphs — the four `[Unreleased]` held (#506, #491, #501, #499) and five new (#497, #485's write side, #487's second half, #495, this rail). `docs/audits/crux-1.28.0.md`: one roster row per paragraph, Method naming the lane and the three corrections, a Gate H keys table, a limit section.
- **The one line of code.** `scripts/quorum-gate.sh:384-386`: `on_rail` admits `docs/roadmaps/releases.yaml` beside `docs/roadmaps/roadmap.yaml`, by file and not by directory; the refusal text names the ledger. `scripts/quorum_evidence.py` changes in a comment only. `tests/falsification_quorum_gate_has_a_triage_shape.rs`: the fixture takes `also: Option<(&str, &str)>` in place of `touch_code: bool` (`:173`, `:201`; `CODE_EDIT` at `:46` keeps the code-edit shape byte-identical), plus `a_triage_receipt_over_the_release_ledger_is_on_the_rail` (`:450`) and `another_file_under_roadmaps_is_still_off_the_rail` (`:465`).
- **What follows.** After the squash-merge: annotated tag `v1.28.0` on the squash commit (message drafted from this receipt), `binary-release.yml`, `make publish-from-tag TAG=v1.28.0`, `make release-check`; then PMAT-227 (`kind: triage`) books the row with `scripts/release-goal.sh cut v1.28.0 --next v1.29.0` and labels itself `release:v1.29.0`, and gate T is green on main again.

## Falsification

- `docs/audits/logs/PMAT-226-gate-tests.log`: with main's `scripts/quorum-gate.sh` (191770fa) checked out over this branch's test file, `cargo test --test falsification_quorum_gate_has_a_triage_shape` → 7 passed, 1 failed, the failing test the new positive case, the gate naming `docs/roadmaps/releases.yaml` as off the rail; with the rail restored → 8 passed. The six pre-existing cases and the negative case are green in both runs, which is what proves the fixture refactor changed no shape and the rail is by file.
- Gate H is red without the crux document (`GATE H FAIL no docs/audits/crux-1.28.0.md: 9 behaviour(s) …`, measured before it was written) and green with it (`docs/audits/logs/PMAT-226-gate-H.log`).
- Every gate script's registered `# mutation:` comment is asserted present by `tests/falsification_dogfood_scripts_declare_mutations.rs`; none is changed here.

## Review record

- CRUX survey: one agy teamwork lane (`conv-ba0f8bcf`) on the nine paragraphs at 97e72ce6, verdict PASS, nine rows. The orchestrator re-resolved every `[V]` citation against HEAD (9 of 9 in range), re-pinned four to the exact line the claim needs, re-cast rows 1, 2 and 9 onto roster systems, and corrected three descriptions of forjar the lane got wrong: the receipt is hash-bound, not "cryptographically signed"; #495 did not change `machine_is_local`; the rail is verified from the diff. The Method section of the crux document says so.
- Quorum: three sandboxed `--mode plan` lanes on the diff at 568dc169, base pinned at 191770fa (`conv-8a855795`, `conv-9f491252`, `conv-8a38ece1`): 1/3 PASS, 2/3 FAIL. C1–C4, C6–C10, C12 confirmed 3/3, each re-run by the orchestrator. C5 refuted by lane 3 on its count (the non-bold introduction is a sixth new paragraph) and C11 by lanes 2 and 3 on its wording (`an orch-basis: token` read as the literal); both recorded REFUTED with `- corrected:` in `.quorum/evidence/release-1.28.0-judges.md`. Neither touches the diff; no second round was run on an unchanged diff, and this receipt says why rather than hiding a 2/3 FAIL behind a re-run.
- Both delegate dispatches hit their 30-turn cap after every lane had written its file; the lane files were read directly and `lane-reduce.sh` run by the orchestrator (dissent 2, as above).
- Evidence: `.quorum/evidence/release-1.28.0-{claims,lanes,judges,agy,crux,pmat}.md`; artifact `.quorum/PMAT-226-release-1.28.0.json` (kind code, 4 lanes named, judges 3, refuters 3, confirmed 10, refuted 2, vacuous tests in touched paths 0).

## Gates measured

| gate | command | result |
|---|---|---|
| A B C D E F G H T | `make dogfood-release` on 568dc169, tracked tree clean, one run | exit 0; every line in `docs/audits/logs/PMAT-226-dogfood-release.log` and quoted in `docs/audits/dogfood-1.28.0-receipt.md` |
| F, stated | `cargo llvm-cov --workspace --locked --fail-under-lines 95` inside the gate | 96.45%; mutants: 1 `.rs` file changed, none under `src/` — nothing to mutate |
| T | `scripts/dogfood/tagged.sh` | `cut in flight: Cargo.toml is at 1.28.0` (`docs/audits/logs/PMAT-226-gate-T.log`) |
| R, pre-tag | `scripts/dogfood/release-check.sh` | PASS pre-tag, 10 PRs since v1.27.0 with receipts; tag, release, crates.io, docs.rs PENDING |
| rail test | `cargo test --test falsification_quorum_gate_has_a_triage_shape` | 8 passed at HEAD; 7 passed / 1 failed against main's gate script |
| lock | `cargo metadata --format-version 1 --offline --locked --no-deps` | exit 0, forjar 1.28.0 |
| bashrs | `bashrs lint scripts/quorum-gate.sh` | 0 errors (12 warnings, all pre-existing) |
| vacuous | `pmat analyze vacuous-tests --path tests --format json` | 43 of 4713 tree-wide, 0 in the touched test file |
| hooks | `pmat hooks install --strict --force`; pre-commit on every commit | format, complexity, clippy, SATD green; every commit carries `Pmat-Ticket: PMAT-226` |
| I-3 | `transcript-gate.sh` on the session directory | `PASS transcript-gate: attempted=13 denied=0 running_peak=1 slots=3` (session-wide; two dispatches for this ticket) |

Not measured on this host: cargo mutants (PMAT-216) — and on this branch there is nothing for it to mutate. `cargo test --workspace` was not re-run as a separate step: gate F's llvm-cov run executes the whole workspace suite (`--workspace --locked`) and exited 0 with `forjar-contracts 0 failed / 44 ignored`; the one changed test binary was run on its own twice.

## Routing and dispatch

| phase | class | `route.sh` (verbatim) | executed by |
|---|---|---|---|
| 0 frame | orchestration | `route=self w=100.00 basis=absent` | self |
| 1 rail, tests, bump, CHANGELOG | impl | `route=agy-goal w=1.00 basis=absent note=fable-binding effort=1[U]` | self — a one-line predicate, two test cases and prose; the deviation is named here, smaller than a lane brief |
| 3 CRUX | research | `route=agy-grillme w=1.00 basis=absent effort=1[U]` | delegate, one teamwork lane (the nearest agy surface; `/grillme` is absent headless) |
| 4 review | review | `route=agy-quorum w=1.00 basis=absent effort=1[U]` | delegate, three lanes |
| 5 dogfood, receipts, push, PR | orchestration | `route=self w=100.00 basis=absent` | self |

Dispatch ledger: 2 Agent calls (`PMAT-226/ph3.crux`, `PMAT-226/ph4.quorum`), 0 denied, 0 resumes, running peak 1 of 3 slots; agy conversations as above (4). Both dispatches declared with `goal.sh worker` before the call.

## Gaps, named

- The booking PR (PMAT-227) is what makes gate T green again after the tag; between the tag and its merge, T2 and T6 are red on main by design. If that is still true at 05:00 UTC the daily workflow opens its issue — correct behaviour, but a window worth closing fast.
- `scripts/release-goal.sh cut` still does not verify the receipts it books exist (PMAT-225's named gap); gate T refuses downstream.
- The paiml-implement skill's `kind-gate.sh` rail (paiml/paiml-implement#68) does not yet name the ledger; the repo-side rail does as of this branch. Comment left on #68.
- cargo mutants is unmeasured on this host (PMAT-216); this branch could not have exercised it.
- `estimate.sh` returned `first-run[U]` with 0 rows although `docs/audits/impl-estimates.jsonl` carries rows for this repository — its selector does not match this repo's shape; named, not fixed.

IMPL-PMAT-226-RECEIPT-END
