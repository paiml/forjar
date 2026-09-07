# Quorum evidence — PMAT-165 — adjudicated claims (majority of three judges)

27 claim ids (three claim lanes' findings, the teamwork review, the crux rows, seven dispositions and two measured claims) were put to three refuters at 0f9a19b5 and then to three judges at 619723dd (conv-aaea92ae, conv-298d2014, conv-8d744e78; 98–137 s). All three judges PASS; 21 of 27 rows unanimous; the split on D1–D5 is labelling (one lane ruled the complaint half REFUTED-as-stale, two the disposition CONFIRMED — same commit, same code). The judges also answered the five attacks the refuters skipped, each 3-0 with the citation re-located by the delegate: merge-base accepts a tag whose commit reached main through a later merge and that is correct; the EXIT trap removes scratch and worktree and already_published() skips crate A on a re-run; a shared clone cannot satisfy the git-common-dir guard; the exhausted poll prints its refusal and exits 2 through die(); kind == null selects normal dependencies. Every item names the shim-backed test case in this diff that pins it. One caveat recorded: the trap spec is EXIT only, and bash runs the EXIT trap on SIGINT-induced exit.

## REFUTED — 8 claims killed

1. [contract] L3-F1 — scripts/publish-from-tag.sh:111 — The script creates `.publish-metadata.json`, `.publish-pkgs.tsv`, and `.publish-order.tsv` in `$WT`. Since `$WT` is the root of the workspace crate, and these files are not in `.gitignore` or Cargo.toml `exclude`, `cargo publish` on the root crate will fail with an uncommitted files error.
   - evidence: REFUTED — stale: fixed in ea89439b (scratch in a sibling mktemp -d; clean tree re-asserted before every publish); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_publish_from_tag.rs:147.

2. [script] L1-F5 — scripts/publish-from-tag.sh:106 — The execution of `cd \"$WT\"` proves the publish runs in the detached worktree and not the ambient repo.
   - evidence: REFUTED — refuted 3-0: `cd "$WT"` does not prove a detached worktree (a clone passes it); the guard that does is case c, git-common-dir plus two live worktrees (D4); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_publish_from_tag.rs:147.

3. [teamwork] T-F1 — scripts/publish-from-tag.sh:31 — `git worktree add --detach` mutation guard on tags is necessary (proposed fix: Remove `--detach` to prove it is vacuous; git automatically detaches on tags.)
   - evidence: REFUTED — stale: fixed in ea89439b (the header names the real guard); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_publish_from_tag.rs:147.

4. [teamwork] T-F2 — scripts/publish-from-tag.sh:100 — `grep -v '^!! target/'` correctly handles ignored target/ (proposed fix: Remove the grep filter; target/ cannot exist in a fresh /tmp worktree.)
   - evidence: REFUTED — stale: fixed in ea89439b (the dead grep removed; --ignored strict); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_publish_from_tag.rs:147.

5. [teamwork] T-F3 — scripts/publish-from-tag.sh:142 — `cargo search` correctly parses index version (proposed fix: Use exact parsing instead of `grep -qF "${name} = \"${version}\""` which is vulnerable to substring/prefix collisions and newer-version deadlocks.)
   - evidence: REFUTED — stale: fixed in ea89439b (cargo info NAME@VERSION poll, anchored fallback, bounded exit 2); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_publish_from_tag.rs:147.

6. [teamwork] T-F4 — scripts/publish-from-tag.sh:167 — `DRY_RUN=1` succeeds with path dependencies (proposed fix: Skip dry run or restructure for multi-crate path dependencies.)
   - evidence: REFUTED — accepted limit, not a defect (D7): cargo publish --dry-run resolves registry dependencies; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_publish_from_tag.rs:147.

7. [teamwork] T-F5 — scripts/publish-from-tag.sh:120 — Topological sort correctly handles cycles (proposed fix: Filter out dev-dependencies in the jq query to prevent false cycle rejections.)
   - evidence: REFUTED — stale: fixed in ea89439b (dev-dependencies filtered from the topological sort); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_publish_from_tag.rs:147.

8. [tests] L2-F1 — tests/falsification_publish_from_tag.rs:175 — The diff has defect FALSIFY-PUBLISH-C-weak-assertion at tests/falsification_publish_from_tag.rs:175 because test C only checks cwd != repo_path and worktree_count == 1, meaning the script could skip creating a detached worktree entirely (e.g. by using git clone or cp into a temp dir) and still falsely pass the test.
   - evidence: REFUTED — stale: fixed in ea89439b (case c pins git-common-dir and the live worktree count); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_publish_from_tag.rs:147.

## CONFIRMED — 19 claims survived refutation (13 as written, 6 as narrowed)

1. [crux] X-F1 — <home-path>:? — cargo publish --workspace handles dependency ordering natively and uses cargo's polling.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:205.

2. [crux] X-F2 — <home-path>:? — cargo-release uses internal topo-sort and relies on cargo's polling.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:205.

3. [crux] X-F3 — <home-path>:? — release-plz handles dependencies internally and uses its own index polling.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:174.

4. [crux] X-F4 — <home-path>:? — cargo-workspaces (cargo ws publish) uses a fixed sleep interval.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:42.

5. [crux] X-F5 — <home-path>:? — crates.io Trusted Publishing uses short-lived OIDC tokens.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:65.

6. [disposition] D1 — — the script wrote `.publish-*` scratch files into the tag worktree after its clean-tree gate (L3, measured; delegate-corroborated): CONFIRMED. FIXED in ea89439b (PMAT-184): scratch lives in a sibling `mktemp -d` under the same trap, and the clean-tree assertion is re-run before every `cargo publish`; case f pins that no scratch file ever lands in the worktree.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:147.

7. [disposition] D2 — — the index poll used `grep -qF` on `cargo search`, colliding on prefixes and never terminating once a newer version exists (T): CONFIRMED. FIXED in ea89439b (PMAT-185): `cargo info NAME@VERSION --registry crates-io` with an anchored `cargo search` fallback and a bounded exit-2; cases g, h, i.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:174.

8. [disposition] D3 — — the topological sort took dev-dependencies, so a dev back-edge read as a cycle (T): CONFIRMED. FIXED in ea89439b (PMAT-186): `select(.kind == null or .kind == "build")`; case j.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:259.

9. [disposition] D4 — — test C did not pin the detached worktree and the header's `--detach` guard claim was vacuous (L2, T), and L1-F5's `cd "$WT"` sentence was the same vacuity from the claim side: CONFIRMED. FIXED in ea89439b (PMAT-187): case c asserts the shim's `git rev-parse --git-common-dir` resolves to the sandbox repo and that two worktrees were live; the header names the real guard (git clone in place of git worktree add turns case c red — measured by the worker, 9 passed / 1 failed).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:65.

10. [disposition] D5 — — `grep -v '^!! target/'` was dead in a fresh worktree (T): CONFIRMED. FIXED in ea89439b (the assertion is strict).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:65.

11. [disposition] D6 — — local long-lived credentials versus crates.io Trusted Publishing (X): REJECTED as a defect — the manual publish from a detached worktree with the local credentials file is the operator's sanctioned path (run brief §0.1); no workflow runs cargo publish, no registry token enters GitHub secrets.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:65.

12. [disposition] D7 — — `DRY_RUN=1` cannot dry-run a dependent whose dependency is not yet on the index (T): CONFIRMED as an inherent cargo limit, not a defect — `cargo publish --dry-run` resolves registry dependencies; the script skips crates already on the index and, in the real run, publishes in order and polls between dependents.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:174.

13. [measured] F1 — — At 0044c116: `cargo test --test falsification_publish_from_tag` 10 passed; `pv validate contracts/publish-from-tag-v1.yaml` valid; `bashrs lint scripts/publish-from-tag.sh` 0 errors; fmt and clippy clean (worker-reported; the test and lint re-run by the orchestrator are recorded in the receipt).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_publish_from_tag.rs:147.

14. [measured] F2 — — In scripts/publish-from-tag.sh every occurrence of `allow-dirty` is in a comment saying the flag is never used, `grep -n 'sleep [0-9]'` matches nothing, and every line naming CARGO_REGISTRY_TOKEN is an `env -u CARGO_REGISTRY_TOKEN` invocation that strips it (the earlier form of this sentence quoted a grep that matched comments and could not match `env -u`; the refuters corrected it).
   - corrected: every cargo publish invocation strips the registry token via env -u CARGO_REGISTRY_TOKEN (lines 246 and 253), every occurrence of allow-dirty is in a comment saying the flag is never used, and no fixed sleep exists
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_publish_from_tag.rs:147.

15. [script] L1-F1 — scripts/publish-from-tag.sh:61 — The script exits 2 for a missing or malformed tag before any cargo calls.
   - corrected: relocated: the refusal chain now begins at scripts/publish-from-tag.sh line 69
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_publish_from_tag.rs:147.

16. [script] L1-F2 — scripts/publish-from-tag.sh:65 — The script exits 2 for a nonexistent tag before any cargo calls.
   - corrected: relocated to line 73
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_publish_from_tag.rs:147.

17. [script] L1-F3 — scripts/publish-from-tag.sh:69 — The script exits 2 if the tag is not an ancestor of origin/main before any cargo calls.
   - corrected: relocated to line 77
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_publish_from_tag.rs:147.

18. [script] L1-F4 — scripts/publish-from-tag.sh:83 — The script exits 2 if the Cargo.toml version disagrees with the tag before any cargo calls.
   - corrected: relocated to line 91
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_publish_from_tag.rs:147.

19. [script] L1-F6 — scripts/publish-from-tag.sh:167 — The child cargo sees a token-free environment because the registry token is stripped via `env -u CARGO_REGISTRY_TOKEN`.
   - corrected: relocated to line 246
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_publish_from_tag.rs:147.

## Tables as returned

### Judge 1 (verdict PASS)

# Refuter Split Decision

- **L1-F5**: Split between Refuter 1 (who narrowed it, accepting `cd` as proof) and Refuters 2 & 3 (who correctly refuted it because `cd` does not prove a detached worktree, a clone would also pass).
- **F2**: Split was over the grep regex matching comments. Refuter 2 outright refuted the claim because of it, while Refuters 1 and 3 narrowed it to correct the regex text.
- **Line-Relocation Family (L1-F1..F4, L1-F6)**: Split between Refuter 2 (who accepted them as-is/SURVIVES without changing the lines) and Refuters 1 & 3 (who narrowed them by updating the line numbers to match HEAD).

# Complete Adjudication Table

| ID | Status |
|---|---|
| L1-F1 | CONFIRMED-AS-NARROWED |
| L1-F2 | CONFIRMED-AS-NARROWED |
| L1-F3 | CONFIRMED-AS-NARROWED |
| L1-F4 | CONFIRMED-AS-NARROWED |
| L1-F5 | REFUTED |
| L1-F6 | CONFIRMED-AS-NARROWED |
| L2-F1 | REFUTED |
| L3-F1 | REFUTED |
| T-F1 | REFUTED |
| T-F2 | REFUTED |
| T-F3 | REFUTED |
| T-F4 | REFUTED |
| T-F5 | REFUTED |
| X-F1 | CONFIRMED |
| X-F2 | CONFIRMED |
| X-F3 | CONFIRMED |
| X-F4 | CONFIRMED |
| X-F5 | CONFIRMED |
| D1 | REFUTED |
| D2 | REFUTED |
| D3 | REFUTED |
| D4 | REFUTED |
| D5 | REFUTED |
| D6 | CONFIRMED |
| D7 | CONFIRMED |
| F1 | CONFIRMED |
| F2 | CONFIRMED |

**Totals:**
- CONFIRMED: 9
- CONFIRMED-AS-NARROWED: 5
- REFUTED: 13

### Judge 2 (verdict PASS)

Adjudication Table:
1. L1-F1: CONFIRMED-AS-NARROWED
2. L1-F2: CONFIRMED-AS-NARROWED
3. L1-F3: CONFIRMED-AS-NARROWED
4. L1-F4: CONFIRMED-AS-NARROWED
5. L1-F5: REFUTED
6. L1-F6: CONFIRMED-AS-NARROWED
7. L2-F1: REFUTED
8. L3-F1: REFUTED
9. T-F1: REFUTED
10. T-F2: REFUTED
11. T-F3: REFUTED
12. T-F4: REFUTED
13. T-F5: REFUTED
14. X-F1: CONFIRMED
15. X-F2: CONFIRMED
16. X-F3: CONFIRMED
17. X-F4: CONFIRMED
18. X-F5: CONFIRMED
19. D1: CONFIRMED
20. D2: CONFIRMED
21. D3: CONFIRMED
22. D4: CONFIRMED
23. D5: CONFIRMED
24. D6: CONFIRMED
25. D7: CONFIRMED
26. F1: CONFIRMED
27. F2: CONFIRMED

Split Decision:
- L1-F5: Refuter 1 narrowed it, Refuters 2 and 3 refuted it. I side with Refuters 2/3: REFUTED (a clone passes `cd`).
- F2: Refuter 2 refuted the old regex; but F2 was restated after the refuters to fix the regex, making the current dossier sentence true. CONFIRMED.
- The line-relocation family (L1-F1..F4, F6): The lines moved, but substance holds. CONFIRMED-AS-NARROWED.

Totals:
- CONFIRMED: 14
- CONFIRMED-AS-NARROWED: 5
- REFUTED: 8

### Judge 3 (verdict PASS)

L1-F1: CONFIRMED-AS-NARROWED
L1-F2: CONFIRMED-AS-NARROWED
L1-F3: CONFIRMED-AS-NARROWED
L1-F4: CONFIRMED-AS-NARROWED
L1-F5: REFUTED
L1-F6: CONFIRMED-AS-NARROWED
L2-F1: REFUTED
L3-F1: REFUTED
T-F1: REFUTED
T-F2: REFUTED
T-F3: REFUTED
T-F4: REFUTED
T-F5: REFUTED
X-F1: CONFIRMED
X-F2: CONFIRMED
X-F3: CONFIRMED
X-F4: CONFIRMED
X-F5: CONFIRMED
D1: CONFIRMED
D2: CONFIRMED
D3: CONFIRMED
D4: CONFIRMED
D5: CONFIRMED
D6: CONFIRMED
D7: CONFIRMED
F1: CONFIRMED
F2: CONFIRMED-AS-NARROWED

Totals:
CONFIRMED: 13
CONFIRMED-AS-NARROWED: 6
REFUTED: 8

