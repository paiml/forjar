# PMAT-232 — quorum lanes (agy, --sandbox, writes=false)

Three lanes at efa49822 with `origin/main` pinned to a138c809. Each was told to re-run all three mutations itself rather than trust the log, to work out by hand which of the three possible `state` strings takes which branch of the shell `case`, to check what happens when `gh` fails or returns nothing, and to finish by reading the diff as a hostile reader.

### lane 1 — verdict FAIL (conv-179049d3, 1 turn(s), 0s)

C1 CONFIRMED, C2 CONFIRMED, C3 CONFIRMED, C4 CONFIRMED, C5 CONFIRMED, C6 CONFIRMED, C7 CONFIRMED, C8 CONFIRMED, C9 CONFIRMED, C10 CONFIRMED, C11 CONFIRMED. All eleven claims hold perfectly, but a hostile reading of the diff revealed a false generalization in the roadmap text regarding the job steps.

### lane 2 — verdict FAIL (conv-803e03ae, 1 turn(s), 0s)

C1 CONFIRMED, C2 CONFIRMED, C3 CONFIRMED, C4 CONFIRMED, C5 CONFIRMED, C6 CONFIRMED, C7 CONFIRMED, C8 CONFIRMED, C9 CONFIRMED, C10 CONFIRMED, C11 CONFIRMED.

Shell evaluation:
(a) `gh ... --jq` outputs raw strings without quotes, and bash strips trailing newlines. Draft emits `prerelease=true draft=true` (matches edit). Published prerelease emits `prerelease=true draft=false` (passes). Published full emits `prerelease=false draft=false` (passes). No trailing whitespace interferes.
(b) If gh fails or outputs an empty string, state falls to `*)` and exits 1, safely failing the job. No loops or incorrect edits occur.
(c) GitHub Actions uses `bash -e {0}` by default (which was verified in the step logs). `set -e` is active. If the first `gh release view` fails, the job immediately fails and exits safely.
(d) `needs.create-release.outputs.created` is completely unused in the new `release.yml`, yet `create-release` still sets it, leaving behind a dead output.

Verdict reason: The 11 claims are all CONFIRMED, but I found three sentences in the diff (commit message, workflow comment, and roadmap) that contain false assertions regarding the number of assets/installer uploaded by the re-dispatch, and the presence of GH_TOKEN in every other step of the homebrew job.

### lane 3 — verdict FAIL (conv-02916c3e, 1 turn(s), 0s)

C1 CONFIRMED, C2 CONFIRMED, C3 CONFIRMED, C4 CONFIRMED, C5 CONFIRMED, C6 CONFIRMED, C7 CONFIRMED, C8 CONFIRMED, C9 REFUTED, C10 CONFIRMED, C11 CONFIRMED. The verdict is FAIL because two claims are false: actionlint has exactly 11 findings, not 22, and only the 'Download release checksums' step logs the GH_TOKEN environment, not 'every other step' as claimed in the roadmap. The shell execution rules are correctly implemented.


## reduction

0 PASS / 3 FAIL. Every one of the eleven claims was confirmed by at least two lanes and nine by all three; the FAIL verdicts come from the hostile-reader half of the brief, which is where the two real defects were. One lane's refutation of the actionlint count does not reproduce and is recorded as a lane error.
