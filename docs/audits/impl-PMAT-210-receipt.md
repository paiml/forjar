# Implementation receipt — PMAT-210 — triage of issue #485, an S1 filed four hours after the 1.26.0 cut, classified and deferred to 1.27 as PMAT-209 with the local half of the defect measured first

verdict: DONE — the issue is classified S1, disposed to a named release, given a code ticket (PMAT-209), recorded in `docs/audits/triage-1.26.0.md`, and answered on the issue itself; no `src/` change belongs on this branch and none is on it.

## Identity

| field | value |
|---|---|
| ticket | PMAT-210 (kind: triage) |
| branch | PMAT-209-triage-485-drift-false-positive |
| issue | #485 (opened 2026-09-08T16:04:13Z by the operator) |
| code ticket minted | PMAT-209, status planned, release 1.27 |
| base_commit | 19b9c1f1e42ea254b3322e526d451835fb92e480 |

## What this branch claims, and does not claim

It claims the triage. It does **not** claim PMAT-209's acceptance criteria: those describe the fix, the fix is 1.27 work, and the release that carries the defect is already tagged, published and pinned across the fleet. A triage branch that shipped a `src/` change would be the thing to refuse here, not the thing to expect.

The rail is the skill's own: a `kind:triage` branch may change `docs/audits/**` and `docs/roadmaps/roadmap.yaml`, and this one changes exactly those two paths.

## The disposition

| field | value |
|---|---|
| class | S1 — a drift lane containing such a resource can never be green, and the fleet tripwire is that lane |
| disposition | deferred to 1.27 as its first ticket |
| rationale | filed after the tag, the publish and the fleet pin; a published release cannot absorb it |
| recorded in | `docs/audits/triage-1.26.0.md`, new section "Filed after the cut" |
| answered on | https://github.com/paiml/forjar/issues/485#issuecomment-5588292058 |

The line that read "No S1 finding is open at the cut" now reads "No S1 finding was open at the cut. One was filed four hours after it — #485, above — and is deferred to 1.27 as PMAT-209." The claim was true and stayed true; what changed is that it can no longer be misread as "the board is empty".

## The measurement, so the deferral is narrowed rather than guessed

Two probes on 1.26.0 (`forjar 1.26.0`, resolved through `scripts/dogfood/lib/binary.sh`), each preceded by `forjar plan -f` on the same file, each against a temp state dir under the session scratchpad, on `$(hostname)` only, no SSH:

| probe | content | plan | apply | apply again | drift |
|---|---|---|---|---|---|
| A | literal block with `$HOME` | 1 to add | 1 converged, 0 unchanged, 0 failed | 0 converged, 1 unchanged | No drift detected |
| B | `{{params.user}}` and `${LD_LIBRARY_PATH:-}` inside the content, matching the gx10 `bashrc` shape in the report | 1 to add | 1 converged, 0 unchanged, 0 failed | — | No drift detected |
| C — control, **1.25.2** | probe B's content | 1 to add | 1 converged, 0 unchanged, 0 failed | 0 converged, 1 unchanged | No drift detected |

Probe C exists because the teamwork lane refused the narrowing without it, and it was right to. Probes A and B alone prove only that 1.26.0 is clean locally; they cannot distinguish "the local path is immune" from "1.26.0 already fixed what 1.25.2 got wrong", and those two readings send the next person to opposite places. The control settles it: **1.25.2, the version the report is against, also passes locally** — same three commands, same result, `forjar 1.25.2` from the release tarball whose sha256 `ca633235aa95ac3c306ad1d78b6d24ede567c4b8953e722c3da739a44c6d75c4` matches its line in the v1.25.2 `SHA256SUMS`.

So the local path is immune in both versions, the pin to 1.26.0 did not quietly fix the fleet, and the defect lives somewhere the probe does not reach. The remaining candidates named in PMAT-209 are the remote read-back and state written by an earlier version; neither is testable from this session, because SSH is out of scope for it.

One incidental measurement, recorded because it cost a step: `FORJAR_STATE_DIR` is not read. The first probe run set it and forjar still used `state` relative to the cwd, which is the repository's own state directory; the apply refused on that directory's integrity check and wrote nothing, and `drift` reported the host's real resources. `--state-dir` is the flag that works. Nothing was written outside the scratchpad.

## Gaps

- The remote half of #485 is unmeasured here and stays unmeasured until PMAT-209 is picked up on a machine that shows it. The issue comment names the one command that would split write-side from read-side.
- Whether `FORJAR_STATE_DIR` is meant to exist at all is not decided here; it is noted, not ticketed.

IMPL-PMAT-210-RECEIPT-END
