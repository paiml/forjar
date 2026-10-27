# Claims digest: epic/forjar-697-v1-35-0-lookahead at d6e7e542

Two rounds are adjudicated. Round A is the 3-lane round at 64517a4f (agreed=false: two
FAIL, one PASS). Round B is the 3-lane full-diff round at d6e7e542 (agreed=true, three
PASS). Lane files are attached beside this digest, byte for byte. Citations are line
numbers at the head d6e7e542 and were re-read for this digest.

## CONFIRMED

1. [fix] A1 - src/core/executor/mod.rs grew from 499 lines at base to 509 at 64517a4f, over the 500-line file-health ceiling. True when Round A ran; fixed in d6e7e542.
   - evidence: Round A lane 1 and lane 2 both measured 509. At head the file is 499 lines again, because the call is now `lockless_check::record_for_apply(cfg, scope, &mut locks)` at `src/core/executor/mod.rs:406` and the selector plumbing moved to `src/core/executor/lockless_check.rs:190`.
2. [test] A2 - the unit file only called record with default filters, so the -r/-g branches of add_passing had no --lib coverage at 64517a4f. True; fixed in d6e7e542.
   - evidence: at 64517a4f the unit file was 155 lines with no `RecordFilters {` literal. At head `src/core/executor/tests_lockless_check.rs:161` adds a test that records with a matching -r, and asserts a non-matching -r and a non-matching -g are excluded.
3. [fix] B1 - PMAT-721: lint.yml and msrv.yml drop X64 from runs-on for clippy, file-health and msrv, and carry runner.arch in the cache keys (Round B lane 2).
   - evidence: Round B lane 2 cites .github/workflows/lint.yml line 108 for the key. The guard test is `tests/falsification_722_cache_key_arch.rs:50`, and the committed receipt for that ticket is .quorum/ci-arch-neutral-lint.json.
4. [fix] B2 - PMAT-613: a versioned github_release refuses to downgrade a live binary newer than the declared pin (Round B lane 1 and lane 2).
   - evidence: the guard is the shell emitted by `downgrade_guard` at `src/resources/github_release.rs:180`, called from the apply script at `src/resources/github_release.rs:164`.
5. [fix] B3 - PMAT-615: plan -r and apply -r select the same resource set, closed over depends_on with resolver::goal_closure.
   - evidence: the plan side states that it uses `resolver::goal_closure`, the same function the apply resolver calls, at `src/cli/plan_selector.rs:89` (the call itself is at line 130) and the apply side at `src/cli/apply_selection/closure.rs:109`; one function, two callers.
6. [fix] B4 - PMAT-618: the user check asserts declared attributes, and drops the declared primary group from groups: before comparing with the live supplementary set.
   - evidence: `src/resources/user.rs:60` carries the comment and the strip; the groups assertion is built at `src/resources/user.rs:66`.
7. [fix] B5 - PMAT-622: --exclude is repeatable and an exclude pattern that matches no resource is an error, not a silent no-op.
   - evidence: the error text "matches no resource in this config" is at `src/cli/apply_selection/closure.rs:213`; the glob matcher it uses is `simple_glob_match` at `src/cli/helpers_state.rs:106`.
8. [fix] B6 - PMAT-627: the disk_budget sweep guard accepts depth-2 candidates only under /tmp and requires descent from a declared root otherwise.
   - evidence: the shell guard branches on `fb_depth` at `src/resources/disk_budget/detect.rs:145`.
9. [fix] B7 - PMAT-727: the reaper deletes nothing when its open-path scan did not complete (fail safe, forjar#727).
   - evidence: `src/resources/disk_budget/reaper.rs:82` states the fail-safe, and `src/resources/disk_budget/reaper.rs:86` logs that nothing was proven dead and nothing is deleted.
10. [fix] B8 - PMAT-740: docker `state: pulled` requires a well-formed @sha256 digest and verifies it after the pull, without starting a container.
   - evidence: the digest parser is `src/resources/docker.rs:11`, and the "pulled" script arm is `src/resources/docker.rs:61`, which runs docker pull and then a digest check.
11. [fix] B9 - the lockless record honours the reviewed plan scope and the -r/-g selectors on the plan-file apply path, so a filtered-out resource is neither checked on the host nor recorded.
   - evidence: the filter test is `src/core/executor/lockless_check.rs:95` and the plan-scope test is `src/core/executor/lockless_check.rs:107`. The falsification test is `tests/falsification_plan_file_lockless_record_in_scope.rs:155`, whose marker file proves the check never ran.

## REFUTED

1. [lane] R1 - Round A lane 2: seeded() hardcodes no scope and default filters, so the interactive preflight still runs host completion checks on resources the operator excluded with -r/-g.
   - corrected: false. `seeded` takes the apply's already pruned config, and the apply path runs `resolve_selection(&mut config, &selectors, verbose)` at `src/cli/apply.rs:96` before the preflight calls `seeded` at `src/cli/apply_preflight.rs:321`. A resource that -r/-g excluded is no longer in that config, so the loop in `add_passing` at `src/core/executor/lockless_check.rs:90` never visits it. The doc comment above `add_passing` says the same.
2. [lane] R2 - Round A lane 2: executor changes under the PMAT-721 label are outside the scope of any of the eight tickets, since PMAT-721 is a CI task.
   - corrected: false as to scope. The lockless check and its recording are the acceptance criteria of PMAT-615 in docs/roadmaps/roadmap.yaml (the default apply path "RECORDS it converged"; -r/-g select one set), and PMAT-615 is among the eight tickets. The code under review is that feature, at `src/core/executor/lockless_check.rs:95` and `src/core/executor/lockless_check.rs:107`. The part that stands is the label: the commits carry the PMAT-721 trailer for review-finding fixes. That is a labelling note, not an unrequested change.
