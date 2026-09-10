# PMAT-227 — quorum lanes (agy, --sandbox, writes=false)

Each lane cloned the branch with `git clone --shared` at a47ca249 with `origin/main` pinned to cdcc0e80 (which is also the commit `v1.28.0` names), re-ran the command named beside each claim, and returned one structured verdict. No lane built anything: nothing in this diff is code. Conversation ids are shortened to `conv-<8hex>`.

## round 1 (a47ca249) — 3 lane(s), 3 FAIL

### lane 1 — verdict FAIL (conv-4372a570, 1 turn(s), 0s, exit ?)

Quorum evaluated all 9 claims against branch PMAT-227-book-v1.28.0. Claims C1, C2, C3, C5, C6, C7, and C9 are CONFIRMED. Claim C4 is REFUTED because PMAT-227 does not carry the release:v1.29.0 label (it only has kind:triage). Claim C8 is REFUTED because release-goal.sh show fails on the branch due to unmerged commits, and on base it prints basis=docs/roadmaps/releases.yaml:L61 instead of L62. The verdict is FAIL due to refuted claims C4 and C8.

| claim | verdict | grounding | what the lane ran or read |
|---|---|---|---|
| C1 | CONFIRMED | measured | `bash scripts/release-goal.sh window v1.28.0 (exit 0)` — The v1.28.0 row in docs/roadmaps/releases.yaml is byte-identical to scripts/release-goal.sh window v1.28.0 with the note lines dropped. |
| C2 | CONFIRMED | measured | `git for-each-ref --format='%(objecttype) %(creatordate:iso-strict) %(*objectname)' refs/tags/v1.28.0 (exit 0)` — The row's cut is the tag's creatordate (2026-09-10T18:07:14+02:00, i.e. 16:07:14Z); the tag is annotated and points at cdcc0e80, the squash commit of PR #508, and origin carries it. |
| C3 | CONFIRMED | measured | `docs/roadmaps/releases.yaml` — next.tag is v1.29.0 and next.due 2026-09-12T16:07:14Z is the cut plus exactly 172800 seconds. |
| C4 | REFUTED | measured | `docs/roadmaps/roadmap.yaml` — PMAT-227 does not carry release:v1.29.0; its labels block only contains kind:triage. The other ten window tickets carry release:v1.28.0. |
| C5 | CONFIRMED | measured | `DOGFOOD_RELEASES_REF=worktree DOGFOOD_ROADMAP_REF=worktree bash scripts/dogfood/tagged.sh` — Gate T passes with this ledger over main's HEAD (0 of 0 PR(s) merged since v1.28.0 ... due 2026-09-12T16:07:14Z), and on the branch itself the gate is red on ONE arm only — its own unmerged commits since the tag, which is the bypassed-revie |
| C6 | CONFIRMED | measured | `git diff --name-only cdcc0e80 a47ca249 (exit 0)` — The diff cdcc0e80...a47ca249 touches only docs/roadmaps/releases.yaml and docs/roadmaps/roadmap.yaml. |
| C7 | CONFIRMED | measured | `grep -n 'not counted' docs/roadmaps/releases.yaml (exit 1)` — grep -n 'not counted' docs/roadmaps/releases.yaml found nothing, and PMAT-228 exists with kind: code, status: planned and acceptance criteria naming the fix. |
| C8 | REFUTED | measured | `bash scripts/release-goal.sh show (exit 2)` — scripts/release-goal.sh show fails with exit code 2 on the branch due to unmerged commits. When run on base, it prints basis=docs/roadmaps/releases.yaml:L61, not L62. scripts/release-goal.sh sync --check also fails with exit code 2 on the b |
| C9 | CONFIRMED | measured | `cargo search forjar --limit 1 (exit 0)` — forjar 1.28.0 is published on crates.io. |

### lane 2 — verdict FAIL (conv-86493b36, 1 turn(s), 0s, exit ?)

C8 is refuted: scripts/release-goal.sh show fails on the branch, and on base-2 it prints basis=docs/roadmaps/releases.yaml:L61, not L62.

| claim | verdict | grounding | what the lane ran or read |
|---|---|---|---|
| C1 | CONFIRMED | measured | `docs/roadmaps/releases.yaml` — bash scripts/release-goal.sh window v1.28.0 | grep -v '^[[:space:]]*#' > /tmp/c1_window.txt (exit 0) is byte-identical to the extracted row from docs/roadmaps/releases.yaml, and the audit files exist. |
| C2 | CONFIRMED | measured | `refs/tags/v1.28.0` — git for-each-ref --format='%(objecttype) %(creatordate:iso-strict) %(*objectname)' refs/tags/v1.28.0 (exit 0) outputs tag 2026-09-10T18:07:14+02:00 cdcc0e80a17a95158fbf9693556df81149020401, and git ls-remote --tags origin (exit 0) shows it  |
| C3 | CONFIRMED | measured | `docs/roadmaps/releases.yaml` — The arithmetic on the two dates outputs exactly 172800 (exit 0). |
| C4 | CONFIRMED | measured | `docs/roadmaps/roadmap.yaml` — awk for release:v1.28.0 and release:v1.29.0 on docs/roadmaps/roadmap.yaml confirms all 10 tickets for v1.28.0 exclusively hold release:v1.28.0, and PMAT-227 holds release:v1.29.0 (exit 0). |
| C5 | CONFIRMED | measured | `scripts/dogfood/tagged.sh` — On base-2, DOGFOOD_RELEASES_REF=worktree DOGFOOD_ROADMAP_REF=worktree bash scripts/dogfood/tagged.sh passes (exit 0). On the branch, bash scripts/dogfood/tagged.sh fails (exit 1) only on the bypassed-review arm. |
| C6 | CONFIRMED | measured | `docs/roadmaps/releases.yaml` — git diff --name-only cdcc0e80 a47ca249 (exit 0) shows only docs/roadmaps/releases.yaml and docs/roadmaps/roadmap.yaml were modified. |
| C7 | CONFIRMED | measured | `docs/roadmaps/releases.yaml` — grep -n 'not counted' docs/roadmaps/releases.yaml exits with 1 (no matches). PMAT-228 is kind: code, status: planned, and names the fix. |
| C8 | REFUTED | measured | `scripts/release-goal.sh` — bash scripts/release-goal.sh show on the branch fails (exit 2) due to bypassed review. On base-2, it passes (exit 0) but prints basis=docs/roadmaps/releases.yaml:L61, not L62. |
| C9 | CONFIRMED | measured | `crates.io` — cargo search forjar --limit 1 (exit 0) confirms forjar 1.28.0 is published on crates.io. |

### lane 3 — verdict FAIL (conv-71645ff3, 1 turn(s), 0s, exit ?)

C8 is refuted. On the branch, `scripts/release-goal.sh show` fails with exit code 2 ("work bypassed review"). When run on base-3 (cdcc0e80), it prints the basis as L61, not L62. All other claims are confirmed.

| claim | verdict | grounding | what the lane ran or read |
|---|---|---|---|
| C1 | CONFIRMED | measured | `docs/roadmaps/releases.yaml` — The v1.28.0 row in docs/roadmaps/releases.yaml is byte-identical to scripts/release-goal.sh window v1.28.0 with the note lines dropped. |
| C2 | CONFIRMED | measured | `refs/tags/v1.28.0` — The row's cut is the tag's creatordate (2026-09-10T16:07:14Z); the tag is annotated, points at cdcc0e80, and origin carries it. |
| C3 | CONFIRMED | cited | `docs/roadmaps/releases.yaml` — next.tag is v1.29.0 and next.due 2026-09-12T16:07:14Z is exactly 172800 seconds after the cut. |
| C4 | CONFIRMED | measured | `docs/roadmaps/roadmap.yaml` — PMAT-227 carries release:v1.29.0; every one of the ten window tickets carries release:v1.28.0 and no other roadmap row does. |
| C5 | CONFIRMED | measured | `scripts/dogfood/tagged.sh` — Gate T passes over main's HEAD with the branch's YAMLs copied; on the branch itself it fails exactly on the bypassed-review arm. |
| C6 | CONFIRMED | measured | `docs/roadmaps/releases.yaml` — The diff cdcc0e80...a47ca249 touches only docs/roadmaps/releases.yaml and docs/roadmaps/roadmap.yaml. |
| C7 | CONFIRMED | measured | `docs/roadmaps/roadmap.yaml` — 'not counted' is gone from releases.yaml, and PMAT-228 exists with kind: code, status planned and acceptance criteria naming the fix and its test. |
| C8 | REFUTED | measured | `scripts/release-goal.sh` — scripts/release-goal.sh show fails on the branch with exit code 2 (work bypassed review) rather than printing the goal. If run on a clone at cdcc0e80, it prints basis=docs/roadmaps/releases.yaml:L61, not L62. |
| C9 | CONFIRMED | measured | `crates.io` — forjar 1.28.0 is on crates.io (cargo search forjar --limit 1 from outside the checkout reports 1.28.0). |

## reduction

`lane-reduce.sh <out_dir> --width 3 --not-before <dispatch epoch>`: 0 PASS, 3 FAIL, dissent 3. Every lane refuted C8 and one lane also refuted C4. The orchestrator re-ran both: C8 is a real error in the claim TEXT and is corrected below; C4's refutation does not reproduce and is recorded as a lane error with the measurement that settles it. Neither changes a byte of the diff, and the second one changes nothing at all — so no second round was run.
