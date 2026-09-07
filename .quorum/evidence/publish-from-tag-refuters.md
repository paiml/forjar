# Quorum evidence — PMAT-165 — refuter rulings

Three refuter lanes (conv-7dc7e061, conv-f4c91dfc, conv-ad143174; 287–366 s; per-lane standalone clones; lane 3's wrapper timed out after its structured output was complete) ruled on all 27 dossier ids at 0f9a19b5. Every disposition D1–D7, every crux row and F1 survived; the two round-1 defect findings and all five teamwork claims were refuted as stale (fixed in ea89439b/0044c116); L1-F5 (`cd "$WT"` proves the worktree) was refuted 2-1 as the same vacuity D4 records; F2's grep was corrected. Five named attacks (merge-base on a later-merged branch, SIGINT and resume, `git clone --shared` against the git-common-dir guard, the poll bound's message) were not reported on and are handed to the judges. Lanes 2 and 3 ran the shim-backed test and built 5.7 GB each; swept.

## Refuter 1 (verdict PASS)

L1-F1: NARROWED\nL1-F2: NARROWED\nL1-F3: NARROWED\nL1-F4: NARROWED\nL1-F5: NARROWED\nL1-F6: NARROWED\nL2-F1: REFUTED\nL3-F1: REFUTED\nT-F1: REFUTED\nT-F2: REFUTED\nT-F3: REFUTED\nT-F4: REFUTED\nT-F5: REFUTED\nX-F1: SURVIVES\nX-F2: SURVIVES\nX-F3: SURVIVES\nX-F4: SURVIVES\nX-F5: SURVIVES\nD1: SURVIVES\nD2: SURVIVES\nD3: SURVIVES\nD4: SURVIVES\nD5: SURVIVES\nD6: SURVIVES\nD7: SURVIVES\nF1: SURVIVES\nF2: NARROWED

Findings:
- R1-F1 [asserted] scripts/publish-from-tag.sh:69 — L1-F1 (proposed fix: Line changed; substance holds.)
- R1-F2 [asserted] scripts/publish-from-tag.sh:73 — L1-F2 (proposed fix: Line changed; substance holds.)
- R1-F3 [asserted] scripts/publish-from-tag.sh:77 — L1-F3 (proposed fix: Line changed; substance holds.)
- R1-F4 [asserted] scripts/publish-from-tag.sh:91 — L1-F4 (proposed fix: Line changed; substance holds.)
- R1-F5 [asserted] scripts/publish-from-tag.sh:145 — L1-F5 (proposed fix: Line changed; substance holds.)
- R1-F6 [asserted] scripts/publish-from-tag.sh:246 — L1-F6 (proposed fix: Line changed; substance holds.)
- R1-F7 [asserted] tests/falsification_publish_from_tag.rs:89 — L2-F1 (proposed fix: Defect no longer exists; test C now asserts WORKTREES == 2 and git-common-dir.)
- R1-F8 [measured] scripts/publish-from-tag.sh:103 — L3-F1 (proposed fix: Defect no longer exists; scratch files are written to a separate SCRATCH dir outside WT.)
- R1-F9 [measured] scripts/publish-from-tag.sh:35 — T-F1 (proposed fix: Script no longer claims --detach is the guard; correctly identifies WORKTREE OF THIS REPO as the guard.)
- R1-F10 [asserted] scripts/publish-from-tag.sh:122 — T-F2 (proposed fix: The grep filter was removed; assert_worktree_pristine uses --ignored natively.)
- R1-F11 [asserted] scripts/publish-from-tag.sh:198 — T-F3 (proposed fix: grep -qF was removed and replaced by anchored exact match in awk.)
- R1-F12 [asserted] scripts/publish-from-tag.sh:246 — T-F4 (proposed fix: DRY_RUN=1 does not succeed with path dependencies, accepted as inherent cargo limit.)
- R1-F13 [asserted] scripts/publish-from-tag.sh:166 — T-F5 (proposed fix: dev-dependencies are now filtered out, so topological sort correctly handles cycles.)
- R1-F14 [measured] scripts/publish-from-tag.sh:4 — F2 (proposed fix: Grep matched comments and missed env -u due to =; substance holds, but regex must be corrected.)

## Refuter 2 (verdict FAIL)

L1-F1: SURVIVES
L1-F2: SURVIVES
L1-F3: SURVIVES
L1-F4: SURVIVES
L1-F5: REFUTED
L1-F6: SURVIVES
L2-F1: REFUTED
L3-F1: REFUTED
T-F1: REFUTED
T-F2: REFUTED
T-F3: REFUTED
T-F4: REFUTED
T-F5: REFUTED
X-F1: SURVIVES
X-F2: SURVIVES
X-F3: SURVIVES
X-F4: SURVIVES
X-F5: SURVIVES
D1: SURVIVES
D2: SURVIVES
D3: SURVIVES
D4: SURVIVES
D5: SURVIVES
D6: SURVIVES
D7: SURVIVES
F1: SURVIVES
F2: REFUTED

Findings:
- R2-F1 [asserted] scripts/publish-from-tag.sh:145 — L1-F5 (proposed fix: REFUTED: `cd` does not prove it is a detached worktree (a clone would pass this check); D4 explicitly refutes this mechanism.)
- R2-F2 [asserted] tests/falsification_publish_from_tag.rs:65 — L2-F1 (proposed fix: REFUTED: Test C now checks GITCOMMONDIR and asserts worktree_count is 2, successfully pinning the detached worktree.)
- R2-F3 [measured] scripts/publish-from-tag.sh:149 — L3-F1 (proposed fix: REFUTED: The scratch files are written to a sibling `$SCRATCH` directory created by mktemp -d, not into the `$WT` worktree.)
- R2-F4 [measured] scripts/publish-from-tag.sh:114 — T-F1 (proposed fix: REFUTED: The script header explicitly acknowledges that `--detach` is NOT the mutation guard, as git automatically detaches on tags. The true guard is that cargo runs inside a worktree of this repository.)
- R2-F5 [asserted] scripts/publish-from-tag.sh:122 — T-F2 (proposed fix: REFUTED: The `grep -v '^!! target/'` filter was entirely removed from `assert_worktree_pristine` because target/ cannot exist in a fresh /tmp worktree.)
- R2-F6 [asserted] scripts/publish-from-tag.sh:208 — T-F3 (proposed fix: REFUTED: `already_published` no longer uses `grep -qF`. It uses `cargo info` with an anchored `awk` fallback to prevent prefix collisions.)
- R2-F7 [asserted] scripts/publish-from-tag.sh:246 — T-F4 (proposed fix: REFUTED: `DRY_RUN=1` does not succeed with path dependencies if they aren't on the index. D7 confirmed this is an inherent cargo limit, and the script's behavior remains that dry-run fails.)
- R2-F8 [asserted] scripts/publish-from-tag.sh:165 — T-F5 (proposed fix: REFUTED: The jq query now explicitly filters out dev-dependencies (`select(.kind == null or .kind == \"build\")`), so the topological sort correctly handles cycles without false positives.)
- R2-F9 [measured] scripts/publish-from-tag.sh:246 — F2 (proposed fix: REFUTED: The grep command matches the three comment lines explaining `--allow-dirty`, but it does NOT match the `env -u` lines because they lack an `=` character.)

## Refuter 3 (verdict PASS)

L1-F1: NARROWED (relocated to 69)
L1-F2: NARROWED (relocated to 73)
L1-F3: NARROWED (relocated to 77)
L1-F4: NARROWED (relocated to 91)
L1-F5: REFUTED (cd does not prove detached worktree)
L1-F6: NARROWED (relocated to 246)
L2-F1: REFUTED (stale complaint, test C now checks GITCOMMONDIR)
L3-F1: REFUTED (stale complaint, scratch files are in a sibling dir)
T-F1: REFUTED (stale complaint, --detach is vacuous and acknowledged)
T-F2: REFUTED (stale complaint, grep filter removed)
T-F3: REFUTED (stale complaint, uses exact awk parsing now)
T-F4: REFUTED (stale complaint, DRY_RUN=1 still fails with path deps if not on index)
T-F5: REFUTED (stale complaint, dev-deps are now filtered)
X-F1: SURVIVES
X-F2: SURVIVES
X-F3: SURVIVES
X-F4: SURVIVES
X-F5: SURVIVES
D1: SURVIVES
D2: SURVIVES
D3: SURVIVES
D4: SURVIVES
D5: SURVIVES
D6: SURVIVES
D7: SURVIVES
F1: SURVIVES
F2: NARROWED (grep regex also matches comment lines now)

Findings:
- R3-F1 [asserted] scripts/publish-from-tag.sh:69 — The script exits 2 for a missing or malformed tag before any cargo calls.
- R3-F2 [asserted] scripts/publish-from-tag.sh:73 — The script exits 2 for a nonexistent tag before any cargo calls.
- R3-F3 [asserted] scripts/publish-from-tag.sh:77 — The script exits 2 if the tag is not an ancestor of origin/main before any cargo calls.
- R3-F4 [asserted] scripts/publish-from-tag.sh:91 — The script exits 2 if the Cargo.toml version disagrees with the tag before any cargo calls.
- R3-F5 [asserted] scripts/publish-from-tag.sh:145 — The execution of `cd "$WT"` proves the publish runs in the detached worktree and not the ambient repo.
- R3-F6 [asserted] scripts/publish-from-tag.sh:246 — The child cargo sees a token-free environment because the registry token is stripped via `env -u CARGO_REGISTRY_TOKEN`.
- R3-F7 [asserted] tests/falsification_publish_from_tag.rs:89 — The diff has defect FALSIFY-PUBLISH-C-weak-assertion at tests/falsification_publish_from_tag.rs:175 because test C only checks cwd != repo_path and worktree_count == 1, meaning the script could skip creating a detached worktree entirely (e.g. by using git clone or cp into a temp dir) and still falsely pass the test.
- R3-F8 [measured] scripts/publish-from-tag.sh:149 — The script creates `.publish-metadata.json`, `.publish-pkgs.tsv`, and `.publish-order.tsv` in `$WT`. Since `$WT` is the root of the workspace crate, and these files are not in `.gitignore` or Cargo.toml `exclude`, `cargo publish` on the root crate will fail with an uncommitted files error.
- R3-F9 [measured] scripts/publish-from-tag.sh:35 — `git worktree add --detach` mutation guard on tags is necessary (proposed fix: Remove `--detach` to prove it is vacuous; git automatically detaches on tags.)
- R3-F10 [asserted] scripts/publish-from-tag.sh:122 — `grep -v '^!! target/'` correctly handles ignored target/ (proposed fix: Remove the grep filter; target/ cannot exist in a fresh /tmp worktree.)
- R3-F11 [asserted] scripts/publish-from-tag.sh:208 — `cargo search` correctly parses index version (proposed fix: Use exact parsing instead of `grep -qF "${name} = \"${version}\""` which is vulnerable to substring/prefix collisions and newer-version deadlocks.)
- R3-F12 [asserted] scripts/publish-from-tag.sh:248 — `DRY_RUN=1` succeeds with path dependencies (proposed fix: Skip dry run or restructure for multi-crate path dependencies.)
- R3-F13 [asserted] scripts/publish-from-tag.sh:165 — Topological sort correctly handles cycles (proposed fix: Filter out dev-dependencies in the jq query to prevent false cycle rejections.)
- R3-F14 [measured] scripts/publish-from-tag.sh:4 — No `--allow-dirty`, no token echo/export, no fixed sleep in scripts/publish-from-tag.sh.

