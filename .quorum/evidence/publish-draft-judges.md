# Quorum evidence — PMAT-232 — adjudicated claims

One round of three sandboxed lanes, base pinned at a138c809, the diff at efa49822: 0/3 PASS. All eleven claims confirmed — nine by all three lanes, two by two of three — and the FAIL verdicts come from the hostile-reader half of the brief, which found two sentences a reader could check and find false. Both are corrected. One lane's refutation does not reproduce and is recorded rather than dropped.

Citations resolve at the merge base: `tests/falsification_release_workflow_shape.rs:184` is rule 3's test function, which pins the `created` output this change stops consuming, and `:241` is rule 5, the rule this change rewrites.

## CONFIRMED

1. [observed] THE DEFECT WAS OBSERVED, NOT REASONED ABOUT — in run 34530301814 every asset job succeeded, `publish-release` reported `success`, and the v1.28.0 release stayed `draft=true`, because both the edit and the assertion sat behind `needs.create-release.outputs.created` and that run did not create the release.
- evidence: all three lanes read the run and the release. The job that was supposed to catch it is `tests/falsification_release_workflow_shape.rs:241`'s subject, and its old assertion was inside the same guard, which is why no rule went red either.

2. [state-not-provenance] THE FIX DECIDES FROM THE RELEASE'S STATE, NOT FROM WHO CREATED IT — it reads `isDraft`/`isPrerelease`, edits only when the state ends `draft=true`, re-reads, and exits non-zero with an `::error::` if the release is still a draft.
- evidence: 3/3, each lane working out the three possible state strings by hand. A draft emits `prerelease=true draft=true` and takes the edit; a published prerelease emits `prerelease=true draft=false` and a published full release `prerelease=false draft=false`, and both pass without being touched.

3. [unconditional] THE ASSERTION IS UNCONDITIONAL AND ASSERTS ONLY `draft=false` — `prerelease=true` is deliberately not asserted, because a re-dispatch over a release an operator has promoted would then fail on a correct state.
- evidence: 3/3. Rule 4 still asserts `prerelease=true draft=true` for a run that creates a release, so the property is not lost, only moved to the job that can actually know it; `tests/falsification_release_workflow_shape.rs:184` is its neighbour.

4. [no-demotion] A RE-DISPATCH CANNOT DEMOTE A PUBLISHED RELEASE — the edit is conditional on `draft=true`, so an operator's promotion to a full release survives any number of re-dispatches.
- evidence: 3/3, including the mutation that makes the edit unconditional and turns rule 5 red.

5. [red-green] RULE 5 IS RED AGAINST MAIN AND GREEN HERE — 7 passed / 1 failed with main's release.yml checked out over this branch's tests, 8 passed at HEAD.
- evidence: `docs/audits/logs/PMAT-232-gate-tests.log`, re-run by all three lanes in their clones; the failing rule is `tests/falsification_release_workflow_shape.rs:241` and the seven that stay green are the PMAT-166 rules around it.

6. [battery] THREE MUTATIONS, EACH RED — the `created` guard restored, the end-state assertion removed, the edit made unconditional.
- evidence: `docs/audits/logs/PMAT-232-rule5-mutations.log`; the brief told each lane to apply all three itself rather than trust the log, and all three did and agreed. Each mutation edits the job `tests/falsification_release_workflow_shape.rs:241` reads.

7. [rule-was-weak-first] THE FIRST VERSION OF THE RULE WAS TOO WEAK AND IS RECORDED AS SUCH — it asserted only that `draft=false` appears in the run block, which the `gh release edit --draft=false` command satisfies by itself, so two of the three mutations stayed green until the rule was tightened.
- evidence: the battery log's first run, and `tests/falsification_release_workflow_shape.rs:241`'s comment, which says so in the file rather than only in a receipt.

8. [shell] THE SHELL IS SAFE IN EVERY STATE THE LANES COULD CONSTRUCT — `gh --jq` emits a raw string with no quotes, bash strips the trailing newline, an empty or unexpected `state` falls to the catch-all and exits 1, and the step runs under `bash -e` so a failing `gh release view` fails the job rather than continuing on a stale value.
- evidence: 3/3, each lane reasoning about the three strings and the failure paths. This is the part of the change no text rule can cover — `tests/falsification_release_workflow_shape.rs:241` reads the job's text and cannot execute it — and it is why the brief asked for it by name.

9. [actionlint] THE FINDING SET IS UNCHANGED FROM MAIN — 22 findings on both sides, differing only in two shifted in-script line numbers.
- evidence: measured on both trees by the orchestrator, counting lines that carry a `file:line:col:` prefix. Two lanes confirmed; the third refuted it as 11 and is answered below.

10. [triage] PMAT-233 IS CANCELLED AS A DUPLICATE AND PMAT-200 ABSORBS ITS MEASUREMENT — no other ticket's status changed.
- evidence: 3/3 against the roadmap. PMAT-200 already carried the missing `HOMEBREW_TAP_TOKEN` as an operator decision; what PMAT-233 added was the first sight of that decision from inside the job.

11. [scope] NOTHING UNDER src/ CHANGES, SO THE CHANGE IS EXACTLY AS WIDE AS IT CLAIMS — the diff is `.github/workflows/release.yml`, `tests/falsification_release_workflow_shape.rs` and `docs/roadmaps/roadmap.yaml`, which also means gate F's mutation arm has nothing to mutate on this branch and passes by measuring zero rather than by being skipped.
- evidence: 3/3 from `git diff --name-only main...HEAD`. The one test file is `tests/falsification_release_workflow_shape.rs:241`'s, and the rules either side of the rewritten one are untouched.

## REFUTED

12. [two-false-sentences] THE HOSTILE READING FOUND TWO SENTENCES THAT ARE FALSE AS WRITTEN — the new workflow comment said the re-dispatch left "thirteen assets and no installer", and PMAT-200's notes said the tap step's empty token stands out "while every other step in the same job shows `GH_TOKEN: ***`".
- evidence: `dist-artifacts` succeeded at 23:49:16Z and `publish-release` ran at 23:55:15Z, so the release held fourteen assets including `install.sh` by then — thirteen-and-no-installer was the FIRST run's state, not the re-dispatch's. And only one sibling step in the homebrew job declares a token at all: the checksums download, from `secrets.GITHUB_TOKEN`. Found by two lanes independently.
- corrected: the comment now says the release was complete at fourteen assets and still invisible because a draft is, and the note now names both declarations — `secrets.HOMEBREW_TAP_TOKEN`, unprovisioned, against `secrets.GITHUB_TOKEN`, which is — which is the sharper statement of the same defect.

## A lane error, recorded rather than dropped

One lane refuted claim 9, reporting that actionlint yields 11 findings rather than 22. Re-run by the orchestrator over the whole workflow directory on both trees: 22 lines carry a `file:line:col:` prefix on each side, and the sets are identical but for two shifted in-script line numbers. A count of unique messages, or a run over a subset of the files, would give a smaller number; the claim is about the finding set and it stands.

## Named and not acted on

`create-release` still exposes `created` as a job output, and nothing consumes it now that `publish-release` does not. It is kept deliberately: `tests/falsification_release_workflow_shape.rs:184` requires it, and it remains the only record in the run of which run created the release. Removing it would trade a dead output for a weaker rule.
