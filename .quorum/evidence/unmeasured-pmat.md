# PMAT-549 — instruments, and what each said

    cargo fmt --all -- --check                     clean at every commit on the branch
    cargo clippy --locked --all-targets -D warnings clean at every commit
    cargo test --lib tripwire::drift               495 passed at the head (490 at 97bd9161, 493 at 37096656)
    cargo test --test falsification_drift_unmeasured_is_not_drift   4 passed
    cargo test --test falsification_e05_verb_drift_contacts_the_host 5 passed
    cargo test --lib mcp::                         70 passed
    pmat analyze vacuous-tests -f json             432 of 19736 cannot fail; none in a touched path
    bash scripts/quorum-gate.sh (PRINT_HASH)       recorded as diff_sha256 in this receipt
    forjar drift over a host that answers nothing  1.28.0 against this branch (FALSIFY-VE-025)
    paiml/infra drift-tripwire.sh                  end to end, both binaries, before and after infra#561

THE KILL MATRIX. Every mutation was applied to a COMMITTED tree, run against the named suites, then
restored from HEAD by name. Every control run was green.

    M1   the ssh-255 arm disabled                  both integration tests + ssh_exit_255_is_unmeasured_and_names_the_host
    M2   the tripwire's unmeasured exit disabled   an_unreachable_resource_is_unmeasured_not_drifted
    M3   the census is never told                  the two census/report unit tests
    M4   the CLI does not split findings           both integration tests
    M5   content_verdict is clean for a non-digest a_directory_listing_that_never_came_back_is_unmeasured_not_clean + the failed-listing test
    M6   a refused listing read as unmeasured      a_directory_listing_that_failed_is_neither_clean_nor_unmeasured
    M7   an answered listing digests nothing       a_directory_listing_that_answered_is_compared_like_any_digest
    M8   lockless builds its report as a literal   an_unanswered_assertion_is_unmeasured_not_inspected
    M9   the task detector bypasses the reader     an_unanswered_completion_check_is_unmeasured_not_a_failed_guard (+ the lockless test)
    M10  the image detector bypasses the reader    an_unanswered_docker_inspect_is_unmeasured_not_an_error
    M11  only an ssh-WORDED 255 is unmeasured      a_remote_script_that_exits_255_over_ssh_is_read_as_unmeasured
    M12  the state-query detector bypasses it      an_unanswered_state_query_is_unmeasured_not_an_error
    M13  unmeasured takes the exit code from drift drift_takes_the_exit_code_when_a_run_has_drift_and_unmeasured
    M14  the drift alert fires on unmeasured       an_unmeasured_only_run_fires_no_drift_alert

REVERTED, which is the other half of the claim. origin/main's `src/` checked out over the head in a
scratch worktree, the branch's tests kept, both suites run:

    falsification_drift_unmeasured_is_not_drift   0 passed, 4 FAILED
    falsification_e05_verb_drift_contacts_the_host 4 passed, 1 FAILED
      drift_over_an_unreachable_machine_must_not_answer_clean:
      "an unanswered query was reported as drift: {"drifted":true,"findings":[{...,"actual_hash":"MISSING"}]}"
      an_unmeasured_only_run_fires_no_drift_alert: exit Some(1), expected Some(4); stderr "error: 1 drift finding(s)"

VACUOUS SCAN. `pmat analyze vacuous-tests` at the head: 432 of 19736 `#[test]` fns cannot fail (2.2%)
across 2181 parsed files, plus 3 that skip silently. NONE is in a file this branch touches — the nearest
is `test_fj016_detect_drift_full_codegen_error_skips` in src/tripwire/drift/tests_full.rs, which the branch
does not touch. Necessary and not sufficient, and this row is the reason to say so: round one killed a
claim because two detector arms had no test at all, and a scan that asks whether a test CAN fail cannot
see an arm no test reaches. The kill matrix is what answers that question.

DOGFOOD, against a host that answers nothing (203.0.113.9, TEST-NET-3):

    forjar 1.28.0   exit 1   "drift_count": 1, "actual_hash": "MISSING", error: 1 drift finding(s)
    this branch     exit 4   UNMEASURED, error: drift unmeasured — the target did not answer: 1 resource(s)

CONSUMER, measured end to end before and after paiml/infra#561: the old drift-tripwire.sh with this
branch's binary printed `no unexcused drift` and exited 0 over that host — the false clean round one
predicted under claim 8. The fixed script fails the machine with `web/conf was not measured`, and the old
binary still fails it as unexcused drift, so neither half of the pair passes a host nothing answered.

TOOL DEFECTS FOUND. agy-lane's isolation check reported "LANE ISOLATION VIOLATED — a shared ref changed"
against the round-two teamwork lane. The ref was moved by the orchestrator's own commit in another
worktree while the lane ran; the reflog names that commit and its author. The check credits every shared
ref change to the lane, and cannot tell a lane's write from a concurrent commit by the party that
dispatched it. Recorded, and the lane's own clone was removed byte-identical.
