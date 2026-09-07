# Quorum evidence — PMAT-165 (publish poka-yoke) — the claims as put to the refuters

Three claim lanes (lenses: the script; the tests and shim; the Makefile, contract and .gitignore), one agy /teamwork-preview lane and one crux lane read `git diff origin/main...HEAD` at ec67a34d. Their findings forced 02d65ef1 (RED), ea89439b (GREEN) and 0044c116 (contract) — PMAT-184..187. Ids: L1..L3 (findings L<i>-F<j>), T, X, D1..D7, F1..F2.

## Claim lane 1 (verdict PASS)

The script implements the PMAT-165 publish poka-yoke correctly with no defects found.

C1: The script exits 2 before any cargo call if the tag is missing/malformed (scripts/publish-from-tag.sh:61), nonexistent (scripts/publish-from-tag.sh:65), not on main (scripts/publish-from-tag.sh:69), or disagrees with the Cargo.toml version (scripts/publish-from-tag.sh:83).
C2: The execution of `cd "$WT"` (scripts/publish-from-tag.sh:106) into the temporary directory proves the publish runs in the detached worktree and not the ambient repo.
C3: The child cargo environment sees no registry token because it is explicitly stripped using `env -u CARGO_REGISTRY_TOKEN` (scripts/publish-from-tag.sh:167) before execution.
C4: The test `c_dry_run_never_publishes_and_leaves_no_worktree_behind` (tests/falsification_publish_from_tag.rs:214) will fail if the `if [[ "${DRY_RUN:-0}" = "1" ]]; then continue; fi` block (scripts/publish-from-tag.sh:169) is removed.
C5: The script satisfies the `token_free_environment` contract row (contracts/publish-from-tag-v1.yaml:85) by wrapping every publish call with `env -u CARGO_REGISTRY_TOKEN` (scripts/publish-from-tag.sh:167).
C6: The script cannot publish a dirty ignored file from the ambient repo because `git worktree add` (scripts/publish-from-tag.sh:95) checks out a fresh tree without copying ambient untracked files, and a stale index poll that gives up (scripts/publish-from-tag.sh:157) safely aborts the script with exit 2 rather than publishing wrong artifacts.

Findings:
- L1-F1 [asserted] scripts/publish-from-tag.sh:61 — The script exits 2 for a missing or malformed tag before any cargo calls.
- L1-F2 [asserted] scripts/publish-from-tag.sh:65 — The script exits 2 for a nonexistent tag before any cargo calls.
- L1-F3 [asserted] scripts/publish-from-tag.sh:69 — The script exits 2 if the tag is not an ancestor of origin/main before any cargo calls.
- L1-F4 [asserted] scripts/publish-from-tag.sh:83 — The script exits 2 if the Cargo.toml version disagrees with the tag before any cargo calls.
- L1-F5 [asserted] scripts/publish-from-tag.sh:106 — The execution of `cd \"$WT\"` proves the publish runs in the detached worktree and not the ambient repo.
- L1-F6 [asserted] scripts/publish-from-tag.sh:167 — The child cargo sees a token-free environment because the registry token is stripped via `env -u CARGO_REGISTRY_TOKEN`.

## Claim lane 2 (verdict FAIL)

C1: The exact refusal order before the first cargo call is: invalid tag format (exit 2), missing tag (exit 2), tag not ancestor of origin/main (exit 2), Cargo.toml version mismatch (exit 2), and dirty detached worktree (exit 3) at scripts/publish-from-tag.sh:44-88.
C2: The test attempts to prove the publish ran in the worktree by asserting `cwd != repo_path` and `worktree_count == 1`, but the diff has defect FALSIFY-PUBLISH-C-weak-assertion at tests/falsification_publish_from_tag.rs:175 because this would still pass if the script skipped `git worktree add` and just used a directory copy or clone.
C3: The child cargo sees an environment where the registry token is stripped via `env -u CARGO_REGISTRY_TOKEN` (scripts/publish-from-tag.sh:150), which is proven by the shim logging `TOKEN: <unset>`.
C4: Test `e_the_registry_token_never_reaches_the_shim` (tests/falsification_publish_from_tag.rs:198) will fail if `env -u CARGO_REGISTRY_TOKEN` is removed from `cargo publish` at scripts/publish-from-tag.sh:150.
C5: The contract row 'The last package in topological order is never polled' is true of the script because `poll_index` is skipped when the package name matches the last name in the order file (scripts/publish-from-tag.sh:160).
C6: The script cannot publish a dirty ignored file because `git worktree add --detach` (scripts/publish-from-tag.sh:80) creates a completely fresh, isolated tree containing only tracked files, and the dirty check asserts this clean state before any packaging occurs.

Findings:
- L2-F1 [asserted] tests/falsification_publish_from_tag.rs:175 — The diff has defect FALSIFY-PUBLISH-C-weak-assertion at tests/falsification_publish_from_tag.rs:175 because test C only checks cwd != repo_path and worktree_count == 1, meaning the script could skip creating a detached worktree entirely (e.g. by using git clone or cp into a temp dir) and still falsely pass the test.

## Claim lane 3 (verdict FAIL)

C1: The exact refusal order before the first cargo call is: empty/malformed tag (exit 2), nonexistent tag (exit 2), tag not on origin/main (exit 2), version mismatch (exit 2), and dirty detached worktree (exit 3) at scripts/publish-from-tag.sh:57-101.
C2: The publish is proven to run in the worktree because the script creates a temporary directory `WT="$(mktemp -d ...)"` outside the repo, checks out the tag there, and runs `cd "$WT"` before any cargo commands at scripts/publish-from-tag.sh:86.
C3: The child cargo environment is stripped of the token because every publish call is strictly wrapped with `env -u CARGO_REGISTRY_TOKEN` at scripts/publish-from-tag.sh:170.
C4: The test `e_the_registry_token_never_reaches_the_shim` will fail if `env -u CARGO_REGISTRY_TOKEN` is removed from the cargo publish commands at tests/falsification_publish_from_tag.rs:267.
C5: The contract row stating "poll_index skipped when pkg = last(topo_order)" is true of the script, enforced by `if [[ "$name" != "$last_name" ]]; then poll_index...` at scripts/publish-from-tag.sh:176.
C6: The script cannot publish a dirty ignored file because `cargo publish` natively excludes ignored files from packaging, and un-ignored untracked files will cause it to safely abort (without `--allow-dirty`). The script cannot publish a tag on a non-main branch because `git merge-base --is-ancestor` explicitly enforces main ancestry. However, the diff has defect: the `.publish-*` metadata files created in the root worktree are un-ignored untracked files that will cause `cargo publish` of the root crate to deterministically fail with an uncommitted files error at scripts/publish-from-tag.sh:111.

Findings:
- L3-F1 [measured] scripts/publish-from-tag.sh:111 — The script creates `.publish-metadata.json`, `.publish-pkgs.tsv`, and `.publish-order.tsv` in `$WT`. Since `$WT` is the root of the workspace crate, and these files are not in `.gitignore` or Cargo.toml `exclude`, `cargo publish` on the root crate will fail with an uncommitted files error.

## Independent lane — agy /teamwork-preview (verdict FAIL)

The independent code review team completed its audit of PMAT-165-publish-from-tag. All constraints were respected (read-only, no real cargo publish, bash/cargo tests passed cleanly). However, the team discovered critical flaws in the script logic, primarily falsifying the author's core claim about `git worktree add --detach` (git automatically detaches on tags, making the claimed mutation guard vacuous). Additionally, they found deadlocks in DRY_RUN multi-crate path dependencies, dev-dependency cycle false positives in the topo sort, and regex substring matching bugs in the index poll. The standard test suite (pv validate, bash lint, cargo test) passed cleanly, proving that a standard claim lane would have falsely approved this PR. The target codebase requires changes.

Findings:
- T-F1 [measured] scripts/publish-from-tag.sh:31 — `git worktree add --detach` mutation guard on tags is necessary (proposed fix: Remove `--detach` to prove it is vacuous; git automatically detaches on tags.)
- T-F2 [asserted] scripts/publish-from-tag.sh:100 — `grep -v '^!! target/'` correctly handles ignored target/ (proposed fix: Remove the grep filter; target/ cannot exist in a fresh /tmp worktree.)
- T-F3 [asserted] scripts/publish-from-tag.sh:142 — `cargo search` correctly parses index version (proposed fix: Use exact parsing instead of `grep -qF "${name} = \"${version}\""` which is vulnerable to substring/prefix collisions and newer-version deadlocks.)
- T-F4 [asserted] scripts/publish-from-tag.sh:167 — `DRY_RUN=1` succeeds with path dependencies (proposed fix: Skip dry run or restructure for multi-crate path dependencies.)
- T-F5 [asserted] scripts/publish-from-tag.sh:120 — Topological sort correctly handles cycles (proposed fix: Filter out dev-dependencies in the jq query to prevent false cycle rejections.)

## Crux lane — competitive survey (verdict FAIL)

A competitive survey comparing forjar's publish-from-tag.sh script with 5 other systems was conducted and detailed in the artifact. The final verdict is FAIL because forjar relies on long-lived local credentials, violating the superior security pattern established by crates.io Trusted Publishing which uses short-lived OIDC tokens.

Findings:
- X-F1 [asserted] <home-path>:? — cargo publish --workspace handles dependency ordering natively and uses cargo's polling.
- X-F2 [asserted] <home-path>:? — cargo-release uses internal topo-sort and relies on cargo's polling.
- X-F3 [asserted] <home-path>:? — release-plz handles dependencies internally and uses its own index polling.
- X-F4 [asserted] <home-path>:? — cargo-workspaces (cargo ws publish) uses a fixed sleep interval.
- X-F5 [asserted] <home-path>:? — crates.io Trusted Publishing uses short-lived OIDC tokens.

## Orchestrator dispositions (each is itself a claim to refute)

- D1 — the script wrote `.publish-*` scratch files into the tag worktree after its clean-tree gate (L3, measured; delegate-corroborated): CONFIRMED. FIXED in ea89439b (PMAT-184): scratch lives in a sibling `mktemp -d` under the same trap, and the clean-tree assertion is re-run before every `cargo publish`; case f pins that no scratch file ever lands in the worktree.
- D2 — the index poll used `grep -qF` on `cargo search`, colliding on prefixes and never terminating once a newer version exists (T): CONFIRMED. FIXED in ea89439b (PMAT-185): `cargo info NAME@VERSION --registry crates-io` with an anchored `cargo search` fallback and a bounded exit-2; cases g, h, i.
- D3 — the topological sort took dev-dependencies, so a dev back-edge read as a cycle (T): CONFIRMED. FIXED in ea89439b (PMAT-186): `select(.kind == null or .kind == "build")`; case j.
- D4 — test C did not pin the detached worktree and the header's `--detach` guard claim was vacuous (L2, T), and L1-F5's `cd "$WT"` sentence was the same vacuity from the claim side: CONFIRMED. FIXED in ea89439b (PMAT-187): case c asserts the shim's `git rev-parse --git-common-dir` resolves to the sandbox repo and that two worktrees were live; the header names the real guard (git clone in place of git worktree add turns case c red — measured by the worker, 9 passed / 1 failed).
- D5 — `grep -v '^!! target/'` was dead in a fresh worktree (T): CONFIRMED. FIXED in ea89439b (the assertion is strict).
- D6 — local long-lived credentials versus crates.io Trusted Publishing (X): REJECTED as a defect — the manual publish from a detached worktree with the local credentials file is the operator's sanctioned path (run brief §0.1); no workflow runs cargo publish, no registry token enters GitHub secrets.
- D7 — `DRY_RUN=1` cannot dry-run a dependent whose dependency is not yet on the index (T): CONFIRMED as an inherent cargo limit, not a defect — `cargo publish --dry-run` resolves registry dependencies; the script skips crates already on the index and, in the real run, publishes in order and polls between dependents.

## Orchestrator's own measured claims

- F1 — At 0044c116: `cargo test --test falsification_publish_from_tag` 10 passed; `pv validate contracts/publish-from-tag-v1.yaml` valid; `bashrs lint scripts/publish-from-tag.sh` 0 errors; fmt and clippy clean (worker-reported; the test and lint re-run by the orchestrator are recorded in the receipt).
- F2 — In scripts/publish-from-tag.sh every occurrence of `allow-dirty` is in a comment saying the flag is never used, `grep -n 'sleep [0-9]'` matches nothing, and every line naming CARGO_REGISTRY_TOKEN is an `env -u CARGO_REGISTRY_TOKEN` invocation that strips it (the earlier form of this sentence quoted a grep that matched comments and could not match `env -u`; the refuters corrected it).

