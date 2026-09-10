# PMAT-225 — the claims the quorum judged

```
# PMAT-225 (forjar#506) — claims for the quorum, each with the command that re-derives it

Branch PMAT-225-release-goals on main 8e8e79e4. Judge the DIFF (`git diff 8e8e79e4..HEAD`), re-run every command, cite file:line.

C1  `bash scripts/dogfood/tagged.sh` exits 0 on HEAD and prints `GATE T PASS 5 tagged release(s) since v1.25.0 reconcile with git and GitHub and 16 ticket(s) carry their tag; 8 of 8 PR(s) merged since v1.27.0 carry release:v1.28.0; due 2026-09-10T22:10:01Z, …h left`.
C2  The registered mutation (`86400` -> `86401` in scripts/dogfood/tagged.sh) turns the gate red on the real ledger with `the declared goal and the derived one disagree`; reverting restores PASS.
C3  `cargo test --test falsification_release_goals_are_measured` passes 12 cases; each red case asserts the FAIL line names its subject (a PR, a ticket, a tag, a path, `OVERDUE by 3h`, `UNMEASURED`); a gate that exits 1 without a `GATE T FAIL` line fails the case (a death is not a verdict).
C4  `cargo test --test falsification_release_goal_cut_books_the_tag` passes: a tag with no row is red naming `release-goal.sh cut`; `cut v0.0.1 --next v0.0.2` writes the measured row, next.due = cut + 2 days exactly, labels the shipped ticket, moves a free-rider's `release:v0.0.1` to `release:v0.0.2`; `sync --check` exits 1 naming the unlabelled open ticket and `sync` labels it; committed, the gate is green; `show` prints `1h/48h left=47h · 1 merged, 1 tagged … ledger=worktree`.
C5  `cargo test --test falsification_dogfood_harness_and_quorum` passes 18; the two new gate A cases FAIL against main's scripts/dogfood/harness.sh + lib/window.sh (`git stash push -- <those two>`; run; `git stash pop`).
C6  Gate A's rule (lib/window.sh `dogfood_pr_tickets`): the receipt address is the FIRST id in branch, then title, then body, resolved against the registry at HEAD; an id that is no row resolves only through a row labelled `alias:<id>`; a stray first id leaves DOGFOOD_TICKET empty and gate A fails naming it — it is NEVER skipped for the next id (so a forgotten roadmap row cannot fall through to an older ticket in the body).
C7  Every `releases:` row in docs/roadmaps/releases.yaml equals `bash scripts/release-goal.sh window <tag>` on HEAD (prs by ancestry, tickets by the one rule, cut = `git for-each-ref --format=%(creatordate:…)` in UTC — the tagger date; forjar's tags are annotated). None was typed.
C8  `git diff 8e8e79e4..HEAD -- docs/roadmaps/roadmap.yaml` changes ONLY: `- release:<tag>` and `- alias:PMAT-218` label lines, `updated:` stamps, `labels: []` -> `labels:` conversions on rows that gained a label, and the new PMAT-225 row. No YAML round-trip (no re-quoting, no timestamp rewrite elsewhere).
C9  `bash scripts/release-goal.sh window v1.25.1` prints a row with `tickets: []` and exits 0 — an empty ticket census does not kill the tool (`awk NF`, not `grep -v '^$'` under pipefail).
C10 Gate T and the two libraries have no `|| true`, and no `>/dev/null` on a call that can `fail` (lib/window.sh header: fail() prints to stdout; a redirected call is a death). `cargo test --test falsification_dogfood_scripts_declare_mutations` passes over 10 scripts.
C11 `bash scripts/dogfood/comply.sh` -> `GATE B PASS … 14 gate script(s) … at 0 bashrs errors`; `bash scripts/dogfood/contracts.sh` -> `GATE G PASS` with FALSIFY-DF-016..020 resolving to existing tests.
C12 `.github/workflows/release-goal.yml`: `actionlint` clean; `cargo test --test falsification_release_goal_workflow_shape` 5/5 (daily cron, workflow_dispatch, GH_TOKEN on the gate step, fetch-depth 0 + fetch-tags, gate T before the build, one labelled issue upserted on failure() and closed on success(), issues: write, no continue-on-error, no `|| true` outside comments).
C13 The cadence arm: not overdue -> PASS whether Cargo.toml is at the newest tag or at next.tag; overdue (now > next.due, PRs merged, Cargo.toml == newest) -> FAIL `OVERDUE by Nh`; Cargo.toml == next.tag -> `cut in flight`, PASS; any other version -> FAIL `no goal names`. The clock is the tag's creation instant (`creatordate`), not the commit date; `DOGFOOD_NOW` pins it for fixtures only.
C14 Gate T reads the ledger, the registry, Cargo.toml and the receipts AT HEAD (`git show HEAD:…`); `scripts/release-goal.sh` reads the working tree and prints `ledger=worktree` or `ledger=worktree(dirty)`.
C15 Registration is consistent: Makefile `dogfood-release` runs tagged.sh after quorum.sh and before coverage.sh, `make release-goal` exists, the mutations guard test's REQUIRED list has 10 entries including tagged.sh, the forjar-dogfood skill table and CLAUDE.md carry a T row, the contract's qa_gate description names T, CHANGELOG [Unreleased] has one paragraph for #506.

Refutation hunts (name the file:line and the command):
R1 Can a PR's ticket be linked to the wrong release, or a real defect gate A caught on main be hidden now?
R2 Is there any moment of a real release where gate T is red for a wrong reason: on main right after the tag (before `cut` books it — expected red, is the message actionable?), on the release branch with the bump, on a feature branch during an overdue window, on main with an empty window three days later?
R3 Any way the gate becomes a printer: a grep exit 1 under pipefail inside `$(...)`, a jq over a non-array, an empty loop that passes, a `>/dev/null` on a failing function, `local x="$(cmd)"` masking an exit code (bashrs SC2155-class).
R4 Does the daily workflow's issue upsert misbehave: two issues, an issue never closed, a token that cannot write, a `set -e` death before the verdict is captured?
R5 Does `release-goal.sh cut` edit anything outside the one ledger block and the touched rows? Does `label_row` ever corrupt a row (a `labels:` key followed by non-list, a row without `updated:`)?
R6 PMAT-218 -> PMAT-219 alias: is `alias:` on the OWNER row the right place, and can two rows claim the same alias (the code refuses; is the refusal reachable and tested)?

```
