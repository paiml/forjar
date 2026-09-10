# Quorum evidence — PMAT-230 — the claims as put to the lanes

Three rounds. Each round's claim set is below in full, because the diff changed after each round and the claims changed with it.


## round 1

# PMAT-230 — claims for the quorum lanes

Branch PMAT-230-release-download-clobber, one commit (469cc9a9) on main
(b3e5812d). Judge the diff `main...469cc9a9`.

1. Rule 9 (`rule9_every_release_download_overwrites_what_a_previous_release_left`
   in tests/falsification_release_workflow_shape.rs) is RED against main's
   workflows and GREEN at HEAD: with `git checkout b3e5812d --
   .github/workflows/release.yml .github/workflows/binary-release.yml` the
   binary reports 8 passed / 1 failed, and at HEAD 9 passed / 0 failed.
2. Every `gh release download` in .github/workflows/** passes `--clobber`
   and none passes `--skip-existing`. There are exactly three call sites:
   release.yml's dist-artifacts job, release.yml's homebrew job, and
   binary-release.yml's "Regenerate and upload SHA256SUMS" step.
3. The rule sweeps every `*.yml` under .github/workflows via
   `all_workflow_files()` rather than naming files, and fails if fewer than
   three call sites match — so a rule that stops matching cannot pass for
   the wrong reason, and a fourth call site added elsewhere is covered.
4. The rule refuses `--skip-existing` BY NAME, and the reason is sound: the
   dist-artifacts step's next line is `test -s /tmp/SHA256SUMS`, which
   cannot distinguish this release's checksums from the previous release's,
   and `forjar dist --checksums-file /tmp/SHA256SUMS` then embeds whatever
   is there into install.sh.
5. The defect being fixed is real and was observed, not hypothesised: run
   34500075606's dist-artifacts job failed with `/tmp/SHA256SUMS already
   exists (use --clobber to overwrite file or --skip-existing to skip)`,
   after which publish-release was skipped and the v1.28.0 GitHub release
   remains a DRAFT PRERELEASE with 13 assets and no install.sh.
6. The repository had already learned this hazard one job over: release.yml's
   `checksums` job clears its staging directory first and carries a comment
   naming the v1.18.0 release whose SHA256SUMS carried ten lines, four of
   them belonging to 1.17.0. dist-artifacts and homebrew had no guard.
7. actionlint reports the SAME finding set on this branch as on main — 22
   findings both sides, differing only in two in-script line numbers that
   moved because comments were added — so the change introduces no new lint
   finding. Both edited files still parse as YAML.
8. The only semantic change to either workflow is the added `--clobber`
   flag: no step was added, removed, reordered or re-scoped, and no job's
   `needs`, `if`, `runs-on` or environment changed.
9. The diff touches two workflow files, one test file and one roadmap row.
   No file under src/ changes, so gate F's mutation arm has nothing to
   mutate on this branch.
10. The PMAT-230 roadmap row carries kind: code, orch:fable with an
    `orch-basis:release` token, the label release:v1.29.0 applied at mint
    time, and three acceptance criteria that are measurable statements.


## round 2

# PMAT-230 — round 2 claims (the diff changed after round 1)

Branch PMAT-230-release-download-clobber, now two commits (469cc9a9,
b02d26b4) on main (b3e5812d). Judge the diff `main...b02d26b4`.

Round 1 (three lanes, 0/3 PASS) refuted three things, all acted on:
C3 (the rule matched a literal string and could be walked past), C5 (the
error message was misquoted), and a judgment that `--clobber` on
binary-release.yml is not needed. The claims below are the corrected set.

1. Rule 9 now finds a call site by SUBCOMMAND, not by literal string: it
   joins the command across backslash continuations from any line
   containing `gh release`, and treats it as a download when `download`
   appears as a whole token. So `gh release -R repo download …` and a
   backslash between `release` and `download` are both caught.
2. Each of the three evasions round 1 named turns the rule RED, measured
   one at a time against release.yml's dist-artifacts call site:
   flags-before-subcommand, backslash-split subcommand, and
   `|| true` suppression. With the file restored the rule is green.
   (docs/audits/logs/PMAT-230-rule9-mutations.log)
3. `||` on a `gh release download` is refused BY NAME, because suppressing
   the exit code leaves the previous release's file exactly where it was —
   the outcome `--clobber` exists to prevent.
4. `--skip-existing` is still refused by name, and the rule still requires
   at least three call sites so it cannot pass by matching nothing.
5. The rule's comment now states that it is a TEXT RATCHET and names what
   it cannot catch: a command assembled from a variable, one inside a
   here-doc the line-joiner does not follow, and one in a script the
   workflow calls rather than in the workflow itself.
6. The quotation of gh's error is now exact everywhere it appears in the
   tree: ``(use `--clobber` to overwrite file or `--skip-existing` to skip
   file)``. Round 1 refuted the earlier wording and was right.
7. binary-release.yml's `--clobber` now says in its own comment that it is
   NOT load-bearing there — that job is on `ubuntu-latest` and `mkdir
   assets` precedes the download, so the collision cannot happen on that
   line today — and why it carries the flag anyway (the rule is uniform;
   `runs-on` has moved under these jobs before). No lane's finding was
   silently dropped.
8. Nothing else changed: the diff is still two workflow files, one test
   file, one roadmap row and one audit log. No file under src/.
9. The whole test binary is green at HEAD (9 passed) and rule 9 alone is
   red against main's workflows.
10. actionlint's finding set is unchanged from main (22 findings both
    sides; only in-script line numbers move).


## round 3

# PMAT-230 — round 3 claims (the rule was rewritten after round 2)

Branch PMAT-230-release-download-clobber, three commits on main (b3e5812d).
Judge the diff `main...HEAD`. Round 1 (0/3 PASS) and round 2 (1/3 PASS)
each refuted the rule's matching; every finding was acted on and measured.

1. Rule 9 finds a call site by reading gh's grammar, not by searching text:
   `download_sites` joins from any line carrying a `gh` token and keeps a
   command whose `release_subcommand` is `download`; `release_subcommand`
   returns the first non-flag token after `release`, skipping `-R`/`--repo`
   and their value.
2. Flags are compared as WHOLE TOKENS (`--clobber`, `--skip-existing`), and
   a trailing `#` comment is cut from every line before the command is
   joined.
3. Seven cases are measured one at a time against release.yml's
   dist-artifacts call site (docs/audits/logs/PMAT-230-rule9-mutations.log):
   flags before the subcommand, a backslash splitting `release`/`download`,
   `|| true`, a backslash splitting `gh`/`release`, a trailing
   `# --clobber` comment, and `--pattern "*--clobber*"` each turn rule 9
   RED; an added innocent `gh release upload --title download` leaves it
   GREEN. With the file restored the whole binary is green.
4. The rule's comment names what it still cannot catch — a command
   assembled from a variable, one inside a here-doc, one in a script the
   workflow calls — and `strip_trailing_comment`'s own doc says it also
   cuts a `#` inside a quoted string, which can only make the rule
   stricter, never looser.
5. The three real call sites are unchanged in behaviour and all pass
   `--clobber`; `sites >= 3` still guards against a rule that matches
   nothing.
6. The whole test binary is green at HEAD (9 passed) and rule 9 alone is
   red against main's workflows.
7. The diff is two workflow files, one test file, one roadmap row and one
   audit log. No file under src/.
8. `release_subcommand` cannot panic or index out of bounds on any input:
   it uses `position`/`?` and a bounded while loop, and returns None when
   there is no non-flag token after `release`.
