# Judges — forjar#607: book v1.33.0 with the tool, and run the cut path without python3

The ticket's full text, verbatim from the roadmap: .quorum/evidence/PMAT-607-ticket.md.

Lanes and findings: see `PMAT-607-lanes.md`. Line numbers are at the head.
The tests were measured on a build host at 57432891, the commit that carries
every script and test change; the commits after it change only the ledger,
the roadmap, the crux audit and this evidence.

## CONFIRMED

1. [loader] C1 — The ledger loads with no python3 on PATH, every form the ledger uses gives the old JSON, and an unknown shape is refused by line.
   - evidence: `tests/falsification_607_release_path_runs_no_python.rs:23` puts a python3 and a python that exit 97 first on PATH. `tests/falsification_607_release_path_runs_no_python.rs:174` loads comments, flow lists with and without spaces, an empty list, a sha with a leading 0 and a comment line inside a row, and compares the JSON exactly. `tests/falsification_607_release_path_runs_no_python.rs:198` feeds five shapes outside the subset and expects exit 1, "does not parse" and the line number. The fixture copies the loader at `tests/release_goal_fixture/mod.rs:204`.
2. [edits] C2 — Labelling, unlabelling and booking run as awk, and keep the old messages and exit codes.
   - evidence: `tests/falsification_607_release_path_runs_no_python.rs:80` cuts with python refused, opens `releases: []`, empties a label list to `[]`, appends to a list and to `[]`, and then runs gate T green on the booked fixture. `tests/falsification_607_release_path_runs_no_python.rs:138` labels once, reports "already" the second time, and refuses an unknown row with exit 3 and the old message.
3. [tags] C3 — A pre-release tag is not a release: it gets no row and opens no window.
   - evidence: `tests/falsification_607_release_path_runs_no_python.rs:219` tags an rc below its release and an rc at HEAD; gate T is green and says the merged PRs carry release:v0.0.2, and `window v0.0.1` prints `prs: [10]`. Every fixture that runs a tag-picking script copies the rule: `tests/release_check_fixture/mod.rs:230`, `tests/dogfood_gates_harness/mod.rs:353`, `tests/falsification_crux_gate_reads_the_release_section.rs:49`.
4. [falsify] C4 — The falsifier goes red when the change is reverted, for each part.
   - evidence: with both ports reverted to ce9e4906, 4 of the 5 tests in `tests/falsification_607_release_path_runs_no_python.rs:80` and below fail; with only `scripts/release-goal.sh` reverted, the two edit tests at `tests/falsification_607_release_path_runs_no_python.rs:80` and `tests/falsification_607_release_path_runs_no_python.rs:138` fail; with the tag rule in `scripts/dogfood/lib/tags.sh` put back to every v* tag, `tests/falsification_607_release_path_runs_no_python.rs:219` fails. Restored: 5 passed, and 19 related test targets pass.
5. [cut] C5 — The v1.33.0 row and the v1.34.0 goal came from the tool, after one alias the tool also wrote.
   - evidence: `release-goal.sh alias PMAT-1055 PMAT-629` first, because #630 closes #629 but names only an id from another repo; then `release-goal.sh cut v1.33.0 --next v1.34.0`. The row's cookbook sha is the cookbook's head today, the same sha as v1.32.0's row, and e43f0743 says so. The tool's own edits are the ones `tests/falsification_607_release_path_runs_no_python.rs:80` checks.
6. [disclosure] C6 — Gate T is not green on this head, and the ticket says so instead of claiming it.
   - evidence: `scripts/dogfood/tagged.sh` at the head prints T3 FAIL naming #646 and #643, which name only a GitHub issue. After that rule, T5 would ask for `docs/audits/dogfood-1.33.0-receipt.md`, which was not measured and is not written. Gate T runs on main and before a tag, not in PR CI. The acceptance criterion added in ffff31c9 states all of this.
7. [crux] C7 — The 1.33.0 crux audit was measured with the tag's own gate, and dates itself after the cut.
   - evidence: `git show v1.33.0:scripts/dogfood/crux-reconcile.sh` run in a detached tree of v1.33.0 with the audit added printed `GATE H PASS 1 of 1`; with the PMAT-607 tag rule the same tree takes the PENDING arm. The audit's method section records both lines and the date, and it lists the rc.1 behaviours gate H never read.

## REFUTED

1. [scope] R1 — The crux audit's claim that the #611 fix appends `$CARGO_HOME/bin` to `$GITHUB_PATH` is not backed, because no such change is in this diff. Raised by gemini-3.1-pro-high in round 1.
   - corrected: the change shipped in v1.33.0 and is not this branch's. It is `git show v1.33.0:.github/workflows/release.yml` line 275, added by 49b33fbd (#654), and ffff31c9 makes the audit cite that. Round 2 ran with the citation committed and was 3/3 PASS.
