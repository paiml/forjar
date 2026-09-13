# PMAT-549 — adjudicated claims

Two rounds of three sandboxed agy lanes, each followed by three judges under a binding kill rule: one
verified counterexample kills a claim however many lanes confirmed it, an absence is verified only by the
judge's own search over all of src/ and tests/, and a judgment question is answered in the reason rather
than deciding the ruling.

Round one (head 830ff969) killed four of eight claims, 3/3 FAIL. Round two (head 37096656, after the
fixes) confirmed all eight, 3/3 PASS, and named four properties no test covered; the head this receipt
binds adds a test for each. The CRUX was ruled on in round one like any other claim.

## REFUTED

1. [r1-lockless-literal] Round one, claim 3 — "no report is constructed without `DriftReport::new`" (all
   three judges, on lane 1's finding).
   - evidence: the lockless path built its report as a struct literal, so its census never learned which
     resources went unmeasured and `unmeasured_total` was 0 on every lockless run. Fixed at
     src/tripwire/drift/lockless.rs:69, which now calls `DriftReport::new(findings, census)`, and pinned by
     `an_unanswered_assertion_is_unmeasured_not_inspected` at src/tripwire/drift/lockless.rs:183.
   - corrected: the census invariant holds for reports built by `DriftReport::new`; at the round-one head
     the lockless path built its own and skipped the unmeasured marking entirely.

2. [r1-apply-wording] Round one, claim 6 — "apply behaviour is unchanged" (all three judges).
   - evidence: `apply` prints each finding's detail, and this change rewrote that detail. The reader is
     src/tripwire/drift/file.rs:162, which now emits `{path} not measured: {why}` where it emitted `not
     accessible`; apply's pre-apply check prints `d.detail` verbatim, so the operator-visible line changed
     even though the set of findings apply consumes did not.
   - corrected: apply still consumes every finding from `detect_drift_full` and still plans to reconcile an
     unmeasured resource, exactly as it did the old MISSING one; the line it prints for that resource now
     says `not measured` where it said `not accessible`. CHANGELOG.md:41 states it in those words.

3. [r1-detector-tests] Round one, claim 7 — "the tests discriminate" (two judges of three; the third
   ruled the factual sentences true and answered the gap as a judgment).
   - evidence: no test asserted that the image or task detectors emit UNMEASURED. Both arms existed —
     src/tripwire/drift/image.rs:74 returns `DriftFinding::unmeasured` — and nothing would have caught
     their removal. Added: `an_unanswered_completion_check_is_unmeasured_not_a_failed_guard` and
     `an_unanswered_docker_inspect_is_unmeasured_not_an_error` at src/tripwire/drift/tests_unmeasured.rs:189
     and src/tripwire/drift/tests_unmeasured.rs:204, each killed by its own mutation (M9, M10).
   - corrected: the tests discriminated for the file path only; the image and task arms were implemented,
     claimed, and untested until this head.

4. [r1-consumer-blind] Round one, claim 8 — "paiml/infra's drift-tripwire.sh will now SEE unmeasured
   resources outside findings" (all three judges).
   - evidence: that script reads `findings`, `drift_count` and `machines_checked`, and nothing else. With
     unmeasured resources moved out of `findings`, it reads `drift_count` 0 against 0 findings, and
     `forjar drift --json` without `--tripwire` exits 0 (src/cli/drift.rs:386) — so it printed `no
     unexcused drift` for a host that answered nothing. Measured end to end with both binaries before the
     consumer was fixed.
   - corrected: it will not see them; it is blind to them, and silently passes an unreachable machine. The
     CHANGELOG carries the upgrade note, and the consumer was fixed in paiml/infra#561 (merged 72f5895),
     which now fails a machine whose resources went unmeasured.

## CONFIRMED

1. [r2-reader-rule] Round two C1 — `Err`, or exit 255 from an SSH transport, is UNMEASURED; any other exit
   status is the target's answer, including a remote script that exits 255 itself.
   - evidence: src/tripwire/drift/unmeasured.rs:60 is `classify`, and the arm at
     src/tripwire/drift/unmeasured.rs:64 tests `out.exit_code == SSH_OWN_FAILURE && is_ssh_transport`. All
     three judges reproduced it; a local script exiting 255 stays an answer, which
     `a_local_script_exiting_255_is_its_own_answer` pins.

2. [r2-every-query] Round two C2 — every query a drift detector sends goes through `unmeasured::read`, the
   directory listing included: refused is an ERROR finding, unanswered is UNMEASURED, and no drift path
   turns either into "no finding".
   - evidence: the listing is src/tripwire/drift/file.rs:77, `listing_digest(unmeasured::read(...))`. One
     round-one judge dissented on the round-one wording, citing `remote_path_digest` as a transport call
     outside the reader; that function is the executor's baseline writer (its caller is in
     src/core/executor/helpers.rs), not a drift detector, and it is disclosed as a residual rather than
     claimed as covered.

3. [r2-census-partition] Round two C3 — the census puts every in-scope resource in exactly one of
   inspected, skipped or unmeasured, for every report including the lockless one.
   - evidence: src/tripwire/drift/census.rs:103 is `is_inspected`, `self.skipped.is_none() &&
     !self.unmeasured`, so the three states partition by construction; and every report is now built by
     `DriftReport::new`, which is what round one's refutation forced.

4. [r2-json-and-text] Round two C4 — CLI JSON keeps `drift_count == findings.len()` and adds
   `unmeasured_count`; text never prints `No drift detected.` while anything is unmeasured; with drift and
   unmeasured together the tripwire exit is the drift one.
   - evidence: src/cli/drift.rs:107 guards the clean line on both lists being empty, the summary line at
     src/cli/drift_report.rs:72 is guarded the same way, and src/cli/drift.rs:373 returns the drift error
     before the unmeasured one at src/cli/drift.rs:379.

5. [r2-mcp-surface] Round two C5 — the MCP verb puts unmeasured resources in `unmeasured` and names them in
   `unchecked`, never in `findings`, and `drifted` is false when only unmeasured remain.
   - evidence: src/mcp/handlers_drift.rs:111 branches on `f.is_unmeasured()`, and `drifted` is
     `!self.findings.is_empty()` at src/mcp/handlers_drift.rs:142. The E05 contract already tells an agent
     to read `drifted` together with `unchecked`, in the doc comment at src/mcp/types.rs:119.

6. [r2-apply-unchanged] Round two C6 — apply still treats an unmeasured resource as observed drift and
   plans to reconcile it; `--alert-cmd`, `policy.notify.on_drift` and `--auto-remediate` act on drift only.
   - evidence: lane 1 of round two refuted this, and all three judges rejected the refutation: apply's
     pre-apply drift check prints each finding's `detail`, which comes from the detectors, not from
     `remote_path_digest`. The alert and remediate arms are gated on `total_drift > 0` in src/cli/drift.rs.

7. [r2-tests-discriminate] Round two C7 — every new or changed test fails if the behaviour it covers is
   reverted, and the round was asked to name what no test would catch.
   - evidence: fourteen mutations, each run on a committed tree and restored by name, each killing the
     named test and nothing else; the control run is green every time. The four properties round two named
     are covered at src/tripwire/drift/tests_unmeasured.rs:214, src/tripwire/drift/tests_unmeasured.rs:222,
     tests/falsification_drift_unmeasured_is_not_drift.rs:215 and
     tests/falsification_drift_unmeasured_is_not_drift.rs:241.

8. [r2-changelog-true] Round two C8 — every sentence of the CHANGELOG entry and of FALSIFY-VE-025 is true
   of the code, including the upgrade note for `--json` consumers.
   - evidence: the apply sentence is CHANGELOG.md:41 and it is the one round one corrected; the upgrade
     note names paiml/infra#560 as a measured consumer, exit 0 with `no unexcused drift` over a host that
     answered nothing. Two judges re-read both texts against the code line by line.

9. [r1-crux-prior-art] Round one CRUX — the prior-art survey's account of forjar and of Nagios, Prometheus
   and Terraform is accurate, including the two points on which forjar does NOT match.
   - evidence: the survey's own citations reproduce — src/core/error.rs:278 is the legacy classifier
     looking for `drift detected`, which the tripwire's `N drift finding(s)` does not contain, so drift
     exits 1 rather than the drift class 10. That string is on the base commit and this branch keeps it;
     the crux digest records it as filed rather than fixed here.
