# Quorum evidence — PMAT-163 — the claims as put to the refuters

Three claim lanes read `git diff origin/main...HEAD` at cf9ac6f7 (lenses: the gate scripts; Make, skill, CLAUDE.md, contract, guards, ledger, fixtures; the forjar-contracts change for #452), one agy /teamwork-preview lane hunted for theater, and one crux lane surveyed the field. Their findings forced f7d97024 and 3c740863 (the release-check and coverage fixes) and 24129273 (the ci.yml wiring, orchestrator). Line numbers are at cf9ac6f7; a refuter must re-locate a cited line. Ids: L1..L3 (claim lanes, findings L<i>-F<j>), T (teamwork), X (crux), D1..D7 (dispositions), F1..F4 (measured).

## Claim lane 1 (verdict PASS)

I have inspected the forjar-dogfood gate scripts and their exact conditions. The scripts strictly avoid vacuous checks by implementing solid floors (`MIN_CLI`, `MIN_INVOCATIONS`, `KERNELS >= 1`) and ceilings, verifying mutations explicitly (e.g. `BASHRS_ERROR_CEILING=0` or `MIN_CLI=9999` tests), executing tests without deleting them (via cfg_attr ignores with bounded counts), and explicitly testing features in multiple permutations (`make dogfood` vs `make dogfood-release`). The architecture and gate validations are completely sound and ready for release.

Findings:
- L1-F1 [asserted] scripts/dogfood/comply.sh:144 — Gate B (comply.sh) passes if `pmat comply check` exits 0, an active branch ruleset exists, gate scripts have 0 bashrs errors, and legacy shell errors don't exceed the ceiling; mutating `BASHRS_ERROR_CEILING=0` turns it red by making the known SEC011 finding a regression.
- L1-F2 [asserted] Makefile:148 — The `make dogfood` target runs `comply.sh`, `surface.sh`, `docs.sh`, and `contracts.sh`, whereas `make dogfood-release` additionally runs `coverage.sh` and `crux-reconcile.sh`.
- L1-F3 [asserted] scripts/dogfood/coverage.sh:45 — The `aprender-corpus` tests are gated behind a `not(feature = "aprender-corpus")` ignore attribute rather than deleted, ensuring they compile on every run, print their ignore reason, and are bounded by the `IGNORED_CEILING` and `APRENDER_ANNOTATIONS` ratchets.
- L1-F4 [asserted] scripts/dogfood/contracts.sh:226 — The test `falsification_contract_citations_resolve` will fail if a source change introduces a citation where a falsifier names an item that is not in the `.rs` file it names.
- L1-F5 [asserted] scripts/dogfood/surface.sh:230 — The `surface_audit.csv` header strictly matches `transport,kind,name,declared,live`, proven by the python script explicitly writing this string to the output file.
- L1-F6 [asserted] scripts/dogfood/surface.sh:239 — None of the gate scripts can pass vacuously, as proved by explicit minimum floors such as `MIN_CLI` which fails the gate if the CLI surface shrinks below the floor.

## Claim lane 2 (verdict PASS)

C1: Gate H (crux-reconcile.sh:53) fails if any behaviour bullet under [Unreleased] lacks a row naming >=3 systems; mutating `MIN_SYSTEMS=4` turns it RED by reporting every row as thin since they were written to the original floor of 3.
C2: `Makefile:154` shows `make dogfood` runs the B, C, D, and G gates, while `make dogfood-release` depends on `dogfood` and adds the F (coverage) and H (crux reconciliation) release-blocking gates.
C3: `scripts/dogfood/coverage.sh:105` proves `aprender-corpus` tests are excluded with a `cfg_attr` ignore, ensuring they are compiled and explicitly reported as ignored (tracked by `IGNORED_CEILING`) rather than being silently deleted.
C4: The `its_first_frontmatter_line_declares_the_unshadowable_name()` test at `tests/falsification_dogfood_skill_is_named.rs:92` fails if the first frontmatter line of the skill file is changed, preserving the collision-free `forjar-dogfood` name.
C5: `docs/audits/surface_audit.csv:1` declares the header `transport,kind,name,declared,live`, which exactly matches the column layout written by `scripts/dogfood/surface.sh:230`.
C6: `scripts/dogfood/surface.sh:237` asserts explicit vacuity floors (`MIN_CLI`, `MIN_MCP`, `MIN_HTTP`) to ensure the parity and agreement checks cannot pass vacuously on an empty binary output.

Findings:
- L2-F1 [asserted] scripts/dogfood/crux-reconcile.sh:53 — Gate H fails if any behaviour bullet under [Unreleased] lacks a row naming >=3 systems; mutating MIN_SYSTEMS=4 turns it RED by reporting every row as thin since they were written to the original floor of 3.
- L2-F2 [asserted] Makefile:154 — make dogfood runs the B, C, D, and G gates, while make dogfood-release depends on dogfood and adds the F and H release-blocking gates.
- L2-F3 [asserted] scripts/dogfood/coverage.sh:105 — aprender-corpus tests are excluded with a cfg_attr ignore, ensuring they are compiled and reported as ignored (tracked by IGNORED_CEILING) rather than being silently deleted or hidden.
- L2-F4 [asserted] tests/falsification_dogfood_skill_is_named.rs:92 — Test its_first_frontmatter_line_declares_the_unshadowable_name() fails if the first frontmatter line of the skill file is changed, preserving the collision-free forjar-dogfood name.
- L2-F5 [cited] docs/audits/surface_audit.csv:1 — The surface_audit.csv declares the header `transport,kind,name,declared,live`, which is the exact column layout written by the surface.sh generator to guarantee the committed ledger format.
- L2-F6 [asserted] scripts/dogfood/surface.sh:237 — surface.sh asserts explicit vacuity floors (MIN_CLI, MIN_MCP, MIN_HTTP) against the measured surface to ensure the parity checks cannot pass vacuously on an empty binary output.

## Claim lane 3 (verdict PASS)

C1: Gate B's exact PASS/FAIL condition is 'pmat comply against the committed .pmat.yaml, with each disabled check replaced by a stronger instrument' and the mutation that turns it RED is 'set BASHRS_ERROR_CEILING=0' at contracts/forjar-dogfood-coverage-v1.yaml:866.
C2: `make dogfood` runs gates B, C, D, G, and `make dogfood-release` adds gates F and H at Makefile:636.
C3: The 38 aprender-corpus tests are gated using `#[cfg_attr(not(feature = \"aprender-corpus\"), ignore = \"<reason>\")]` which avoids deletion by ensuring the bodies still compile and prevent API rot at crates/forjar-contracts/VENDORED.md:1046.
C4: The test `tests/test_fj129_force_distinguishability.rs::fj129_force_distinguishability_four_shapes` is failed by the source change where `--force` collapses 'ran and changed nothing' into 'changed something' at contracts/apply-summary-distinguishability-v1.yaml:716.
C5: The document sentence 'Every gate prints exactly one `GATE <letter> PASS|FAIL <detail>` line, its exit code is the verdict, and it carries a trailing `# mutation:` comment' is true of the code at CLAUDE.md:575.
C6: Evidence that no gate can pass vacuously is provided by the completeness obligation test `the_expected_gates_are_all_present` which pins the expected script set, and the diff has defect FALSIFY-DF-006 maps rule 'Exactly one skill claims the name' to the wrong test `nothing_in_the_tree_is_still_called_dogfood` at contracts/forjar-dogfood-coverage-v1.yaml:941.

Findings:
- L3-F1 [cited] contracts/forjar-dogfood-coverage-v1.yaml:941 — FALSIFY-DF-006 maps the rule 'Exactly one skill claims the name' to the test `nothing_in_the_tree_is_still_called_dogfood` instead of `exactly_one_skill_claims_the_name`, creating a mismatch between the rule text and the test function named. (proposed fix: Change the test reference to `tests/falsification_dogfood_skill_is_named.rs::exactly_one_skill_claims_the_name` or update the rule text to match `nothing_in_the_tree_is_still_called_dogfood`.)

## Independent lane — agy /teamwork-preview (verdict FAIL)

The branch introduces several unsound gating mechanisms that can silently mask failures or pass over empty sets. Notably, release-check.sh ignores squash-merged PRs and masks remote failures if tags aren't fetched locally, and coverage.sh fails to ratchet down ignored test counts, allowing new failures to be hidden in the slack.

Findings:
- T-F1 [asserted] scripts/dogfood/docs.sh:197 — Gate theater: docs.sh reads the surface list from the committed CSV rather than the built artifact, allowing dead verbs to pass the presence check if the CSV is stale.
- T-F2 [asserted] scripts/dogfood/release-check.sh:140 — Exit 0 on failure: release-check.sh uses git log --merges to find PRs. If PRs are squash-merged, the pipeline returns empty but exits 0, bypassing the loop and silently passing the quorum check.
- T-F3 [asserted] scripts/dogfood/surface.sh:41 — Vacuity floors: The floors are hardcoded in the bash scripts (e.g., MIN_CLI=211). Shrinking the ledger does not lower the floor; a developer must manually edit the script to lower the hardcoded value.
- T-F4 [asserted] scripts/dogfood/comply.sh:144 — Mutation comments: They are genuine one-line changes that turn their respective gates red, proving the gate is falsifiable.
- T-F5 [asserted] scripts/dogfood/docs.sh:250 — docs.sh executes blocks, so dead verbs fail execution. However, a dead verb listed in KNOWN_BROKEN (like forjar make clean) bypasses the exit code check and silently passes.
- T-F6 [asserted] scripts/dogfood/release-check.sh:158 — release-check.sh exits 0 (PASS) with PENDING if the tag is missing locally. If the release failed on GitHub but the tag isn't fetched locally, this script masks the failure by passing.
- T-F7 [asserted] scripts/dogfood/coverage.sh:98 — #452 gating: coverage.sh checks if ignored > IGNORED_CEILING, but lacks a check to enforce shrinkage if ignored < IGNORED_CEILING. Slack can be used to silently ignore new failures.

## Crux lane — competitive survey (verdict PASS)

### Competitive Survey of Release Gating

| System | Artifact vs Source | Docs Executed? | Coverage Floor & Anti-Gaming | Go/No-Go Owner |
|---|---|---|---|---|
| **Rust** (`cargo-mutants`, `cargo-semver-checks`) | Mostly source (Rustdoc JSON, mutants on source). | Yes, `cargo test` executes doctests. | Uses `cargo-mutants` to catch vacuous tests, but a strict 95% line floor isn't universally enforced. | CI/CD pipeline or package owner via `cargo publish`. |
| **Kubernetes** | Conformance tests run against real artifacts/clusters; `verify-*` checks source. | Generally no (docs aren't typically part of e2e conformance). | Historically struggles with strict global floors; relies heavily on code review (OWNERS). | SIG Release / Release Managers (human committee) via dashboards. |
| **Debian** (`autopkgtest`, reproducible builds) | Tests built binary packages (`.deb`); reproducible builds verify source-to-artifact mapping. | No systematic doc execution. | No strict coverage floor for packages. | Release Team (human) + automated migration (britney). |
| **Terraform** (Acceptance Tests) | `TF_ACC=1` tests compile the provider and execute against real cloud APIs. | No, docs are often generated from schema but not executed. | Relies on acceptance test CRUD coverage; no mathematical floor. | Provider maintainer reviewing CI. |
| **forjar (Dogfood Gate)** | Measures transport surface (Gate C) from the *running artifact* (stdio, HTTP, `--help`), not just source declarations. | Yes (Gate D), extracts and runs all fenced `forjar` blocks in README against a sandboxed `HOME`. | 95% line floor (Gate F) + `cargo mutants` specifically run over the branch's diff to prevent gaming. | Human operator running `make dogfood-release`, generating a deterministic receipt (Gate A). |

**Verdict**: PASS
`forjar`'s pre-publish dogfood gate is extremely robust and is at least as sound as the surveyed systems. It combines their strongest attributes (mutation testing from Rust, built-artifact testing from Debian/K8s) while introducing exceptionally rigorous requirements—such as end-to-end executable README blocks and competitive crux-reconciliation—that exceed typical industry baselines.

Findings:
- X-F1 [cited] /tmp/lanes-163q1-1355373/scripts/dogfood/coverage.sh:11 — Like Rust's cargo-mutants convention, forjar uses mutation testing to prevent coverage gaming, but explicitly applies it in-diff alongside a strict 95% floor.
- X-F2 [cited] /tmp/lanes-163q1-1355373/scripts/dogfood/surface.sh:15 — Like Kubernetes conformance tests, forjar verifies the live surface of the running artifact rather than trusting static source declarations.
- X-F3 [cited] /tmp/lanes-163q1-1355373/.claude/skills/forjar-dogfood/SKILL.md:128 — Like Debian's reproducible builds, forjar enforces strict determinism by ensuring the generated dogfood receipt is bit-for-bit identical across runs.
- X-F4 [cited] /tmp/lanes-163q1-1355373/scripts/dogfood/docs.sh:11 — Unlike Terraform where docs are often generated but not run, forjar extracts and executes all fenced commands in the README inside a sandboxed environment.

## Orchestrator dispositions (each is itself a claim to refute)

- D1 — release-check enumerated merged PRs with `git log --merges`, invisible to squash merges (T): CONFIRMED (PR #472 was a squash). FIXED in f7d97024 and 3c740863 (PMAT-178): `gh pr list --state merged --base main --search merged:>=<previous tag date>` with each merge commit checked as an ancestor of HEAD; a gh that cannot answer is FAIL; an empty set over commits that landed is FAIL; and the per-PR receipt is the COMMITTED `.quorum/<slug>.json` (slug = head branch with `/` → `-`, the file scripts/quorum-gate.sh enforces on every push, which survives a squash) read at the merge commit or HEAD — JSON, no `waived` key, ≥ 3 lanes and ≥ 3 judges. The brief's `docs/audits/quorum-<pr>.md` matched nothing in this repository.
- D2 — `coverage.sh` compared the ignored-test count with `-gt` only (T): CONFIRMED. FIXED in f7d97024 (PMAT-179): the count must EQUAL the committed figure (43), RED in both directions.
- D3 — release-check reported PENDING from the local tag list, masking a missing release for a remote tag (T): CONFIRMED. FIXED in f7d97024 (PMAT-180): PENDING only when `git ls-remote --tags origin` has no tag for Cargo.toml's version; a remote tag this checkout lacks is FAIL naming `git fetch --tags`; the crux arm (Arm 6, gate H) is PENDING until Cargo.toml's version differs from the last cut tag (3c740863).
- D4 — FALSIFY-DF-006 named `nothing_in_the_tree_is_still_called_dogfood` for the exactly-one-skill rule (L3): CONFIRMED. FIXED in f7d97024 (PMAT-181): DF-006 cites `exactly_one_skill_claims_the_name`; DF-007 carries the other test; DF-010..012 register the new falsifiers.
- D5 — `docs.sh` reads the surface set from the committed CSV for its presence cross-check (T): REJECTED as theater — gate C regenerates `docs/audits/surface_audit.csv` from the built binary earlier in the same `make dogfood` run and fails if it differs, and docs.sh EXECUTES the README blocks; the CSV feeds only the presence cross-check.
- D6 — a documented verb pinned as KNOWN_BROKEN passes by design (T): CONFIRMED as a SKIP row with a named blocker, which the DF verdict rules allow (a SKIP stays in the receipt with its blocker; a vanished gate reads as passed). The one pinned row (`forjar make clean` in README.md) is minted as PMAT-189 and fixed in the release-cut PR, after which the pin is removed.
- D8 — a refuter noted the `aprender-corpus` feature is enabled in no workflow, so the 43 corpus tests never run in this repository's CI: CONFIRMED as an accepted, documented limit, not a hidden failure — the corpus is aprender's, the ignore reason names it, the exact ratchet fails if the count moves in either direction, and the vendored crate's corpus tests run in aprender's own CI; `cargo test -p forjar-contracts --lib --features aprender-corpus` runs them here when the corpus is present.
- D7 — L3's four citations are past EOF (yaml:941 in a 210-line file, Makefile:636 in 178, CLAUDE.md:575 in 86, VENDORED.md:1046 in 64): the delegate re-grepped the substantive defect to contracts/forjar-dogfood-coverage-v1.yaml:187-190, where it was real; the line numbers are lane artifacts.

## Orchestrator's own measured claims

- F1 — `make dogfood` at cf9ac6f7, run by the orchestrator: GATE B PASS (comply clean; ruleset 13878864 requires [gate]; 8 gate scripts at 0 bashrs errors), GATE C PASS (211 CLI names, 12 MCP tools, 12 HTTP verbs, every declared name live and every live name declared, matching the committed ledger), GATE D PASS (18 README invocations run or parse against the fixtures, 1 pinned as known-broken), GATE G PASS (35 contracts validate, pv lint 0 errors, 4 kernels with equations and kani harnesses, every contract names a falsifier); the guard tests 4 + 4 passed; `cargo test -p forjar-contracts --lib` 1333 passed, 0 failed, 43 ignored; `pv validate` of the new contract valid.
- F2 — Coverage on origin/main, `cargo llvm-cov --locked --ignore-run-fail --summary-only` on a clean clone: lines 96.41% workspace, 96.54% forjar crate (functions 92.76% / 92.67%, regions 95.64% / 95.79%) — above the 95-line floor gate F enforces; no gap tickets were needed.
- F3 — `bash scripts/dogfood/release-check.sh` at 3c740863 on this tree (Cargo.toml 1.25.2 = the last tag, so the window is v1.25.1..HEAD): Arms 1–4 evaluate for real and pass (tag on main, GitHub release published, crates.io 1.25.2, docs.rs true); Arm 5 reports receipt=ok for #472, #465, #464 and receipt=waived for #461, #460, #459, #458, #457, #430 — GATE R FAIL, the honest verdict: six pre-run PRs shipped under waivers. For 1.26.0 the window becomes v1.25.2..HEAD, which holds #472 and this run's PRs, every one with a real four-lane receipt.
- F4 — The ci.yml wiring (24129273, orchestrator): a `dogfood` job builds the release binary and runs gates C and D plus the guard tests on every PR, and `gate` requires it; gates B, F, G and H need pmat and pv (pv is `aprender-contracts-cli`, a path crate not on crates.io), so they run in the clean-room through `make dogfood-release` with the tools provisioned by forjar.yaml (the IP ticket).
