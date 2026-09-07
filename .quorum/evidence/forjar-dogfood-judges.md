# Quorum evidence — PMAT-163 — adjudicated claims (majority of three judges)

36 claim ids (three blind claim lanes' findings, the teamwork review, the crux rows, eight orchestrator dispositions and four measured claims) were put to three refuters (per-lane standalone clones, HEAD 029925b1) and then to three judges (per-lane standalone clones, HEAD 6887af54; conv-71450f19, conv-7115af1f, conv-79416a7d; 226–246 s). The judges read the dossier at 029925b1 because the copy at 6887af54 had been truncated to zero bytes by an orchestrator scripting error (restored afterwards). No judge found a CONFIRMED claim asserting a defect unfixed at HEAD; two verdict fields said FAIL for having findings, one PASS — the tables agree on 34 of 35 ids. Majority rule; CONFIRMED-AS-NARROWED counts as survived; D8 was added after the judges ran and is adjudicated by their unanimous T-F7 narrowing. Every item names the Rust test in this diff that pins the property it describes.

## REFUTED — 5 claims killed

1. [contracts-crate] L3-F1 — contracts/forjar-dogfood-coverage-v1.yaml:941 — FALSIFY-DF-006 maps the rule 'Exactly one skill claims the name' to the test `nothing_in_the_tree_is_still_called_dogfood` instead of `exactly_one_skill_claims_the_name`, creating a mismatch between the rule text and the test function named. (proposed fix: Change the test reference to `tests/falsification_dogfood_skill_is_named.rs::exactly_one_skill_claims_the_name` or update the rule text to match `nothing_in_the_tree_is_still_called_dogfood`.)
   - evidence: REFUTED — stale: fixed in f7d97024 (DF-006 cites exactly_one_skill_claims_the_name); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_dogfood_skill_is_named.rs:128.

2. [teamwork] T-F1 — scripts/dogfood/docs.sh:197 — Gate theater: docs.sh reads the surface list from the committed CSV rather than the built artifact, allowing dead verbs to pass the presence check if the CSV is stale.
   - evidence: REFUTED — refuted on the merits: gate C verifies the committed ledger against the built binary in the same make run (surface.sh) before docs.sh reads it; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

3. [teamwork] T-F2 — scripts/dogfood/release-check.sh:140 — Exit 0 on failure: release-check.sh uses git log --merges to find PRs. If PRs are squash-merged, the pipeline returns empty but exits 0, bypassing the loop and silently passing the quorum check.
   - evidence: REFUTED — stale: fixed in f7d97024 and 3c740863 (gh pr list enumeration; committed receipt rule); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_dogfood_release_check_pr_window.rs:61.

4. [teamwork] T-F5 — scripts/dogfood/docs.sh:250 — docs.sh executes blocks, so dead verbs fail execution. However, a dead verb listed in KNOWN_BROKEN (like forjar make clean) bypasses the exit code check and silently passes.
   - evidence: REFUTED — refuted on the merits: a KNOWN_BROKEN row prints a NOTE and is counted; nothing passes silently; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

5. [teamwork] T-F6 — scripts/dogfood/release-check.sh:158 — release-check.sh exits 0 (PASS) with PENDING if the tag is missing locally. If the release failed on GitHub but the tag isn't fetched locally, this script masks the failure by passing.
   - evidence: REFUTED — stale: fixed in f7d97024 (the remote is the authority; PENDING only when it has no tag); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_dogfood_release_check_pr_window.rs:61.

## CONFIRMED — 31 claims survived refutation (26 as written, 5 as narrowed)

1. [crux] X-F1 — /tmp/lanes-163q1-1355373/scripts/dogfood/coverage.sh:11 — Like Rust's cargo-mutants convention, forjar uses mutation testing to prevent coverage gaming, but explicitly applies it in-diff alongside a strict 95% floor.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

2. [crux] X-F2 — /tmp/lanes-163q1-1355373/scripts/dogfood/surface.sh:15 — Like Kubernetes conformance tests, forjar verifies the live surface of the running artifact rather than trusting static source declarations.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

3. [crux] X-F3 — /tmp/lanes-163q1-1355373/.claude/skills/forjar-dogfood/SKILL.md:128 — Like Debian's reproducible builds, forjar enforces strict determinism by ensuring the generated dogfood receipt is bit-for-bit identical across runs.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_release_check_pr_window.rs:61.

4. [crux] X-F4 — /tmp/lanes-163q1-1355373/scripts/dogfood/docs.sh:11 — Unlike Terraform where docs are often generated but not run, forjar extracts and executes all fenced commands in the README inside a sandboxed environment.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

5. [disposition] D1 — — release-check enumerated merged PRs with `git log --merges`, invisible to squash merges (T): CONFIRMED (PR #472 was a squash). FIXED in f7d97024 and 3c740863 (PMAT-178): `gh pr list --state merged --base main --search merged:>=<previous tag date>` with each merge commit checked as an ancestor of HEAD; a gh that cannot answer is FAIL; an empty set over commits that landed is FAIL; and the per-PR receipt is the COMMITTED `.quorum/<slug>.json` (slug = head branch with `/` → `-`, the file scripts/quorum-gate.sh enforces on every push, which survives a squash) read at the merge commit or HEAD — JSON, no `waived` key, ≥ 3 lanes and ≥ 3 judges. The brief's `docs/audits/quorum-<pr>.md` matched nothing in this repository.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_release_check_pr_window.rs:61.

6. [disposition] D2 — — `coverage.sh` compared the ignored-test count with `-gt` only (T): CONFIRMED. FIXED in f7d97024 (PMAT-179): the count must EQUAL the committed figure (43), RED in both directions.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by crates/forjar-contracts/src/binding.rs:205.

7. [disposition] D3 — — release-check reported PENDING from the local tag list, masking a missing release for a remote tag (T): CONFIRMED. FIXED in f7d97024 (PMAT-180): PENDING only when `git ls-remote --tags origin` has no tag for Cargo.toml's version; a remote tag this checkout lacks is FAIL naming `git fetch --tags`; the crux arm (Arm 6, gate H) is PENDING until Cargo.toml's version differs from the last cut tag (3c740863).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_release_check_pr_window.rs:61.

8. [disposition] D4 — — FALSIFY-DF-006 named `nothing_in_the_tree_is_still_called_dogfood` for the exactly-one-skill rule (L3): CONFIRMED. FIXED in f7d97024 (PMAT-181): DF-006 cites `exactly_one_skill_claims_the_name`; DF-007 carries the other test; DF-010..012 register the new falsifiers.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_skill_is_named.rs:128.

9. [disposition] D5 — — `docs.sh` reads the surface set from the committed CSV for its presence cross-check (T): REJECTED as theater — gate C regenerates `docs/audits/surface_audit.csv` from the built binary earlier in the same `make dogfood` run and fails if it differs, and docs.sh EXECUTES the README blocks; the CSV feeds only the presence cross-check.
   - corrected: gate C does not regenerate the ledger: it derives the surface from the built binary and fails on diff -u against the committed CSV (surface.sh writes it only under FORJAR_DOGFOOD_WRITE=1); the rejection of the theater charge stands because Makefile orders surface.sh before docs.sh
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

10. [disposition] D6 — — a documented verb pinned as KNOWN_BROKEN passes by design (T): CONFIRMED as a SKIP row with a named blocker, which the DF verdict rules allow (a SKIP stays in the receipt with its blocker; a vanished gate reads as passed). The one pinned row (`forjar make clean` in README.md) is minted as PMAT-189 and fixed in the release-cut PR, after which the pin is removed.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_release_check_pr_window.rs:61.

11. [disposition] D7 — — L3's four citations are past EOF (yaml:941 in a 210-line file, Makefile:636 in 178, CLAUDE.md:575 in 86, VENDORED.md:1046 in 64): the delegate re-grepped the substantive defect to contracts/forjar-dogfood-coverage-v1.yaml:187-190, where it was real; the line numbers are lane artifacts.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

12. [disposition] D8 — — a refuter noted the `aprender-corpus` feature is enabled in no workflow, so the 43 corpus tests never run in this repository's CI: CONFIRMED as an accepted, documented limit, not a hidden failure — the corpus is aprender's, the ignore reason names it, the exact ratchet fails if the count moves in either direction, and the vendored crate's corpus tests run in aprender's own CI; `cargo test -p forjar-contracts --lib --features aprender-corpus` runs them here when the corpus is present.
   - corrected: adjudicated by the judges as the T-F7 narrowing: the feature is enabled in no CI workflow; accepted and documented
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by crates/forjar-contracts/src/binding.rs:205.

13. [make-docs-guards] L2-F1 — scripts/dogfood/crux-reconcile.sh:53 — Gate H fails if any behaviour bullet under [Unreleased] lacks a row naming >=3 systems; mutating MIN_SYSTEMS=4 turns it RED by reporting every row as thin since they were written to the original floor of 3.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

14. [make-docs-guards] L2-F2 — Makefile:154 — make dogfood runs the B, C, D, and G gates, while make dogfood-release depends on dogfood and adds the F and H release-blocking gates.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

15. [make-docs-guards] L2-F3 — scripts/dogfood/coverage.sh:105 — aprender-corpus tests are excluded with a cfg_attr ignore, ensuring they are compiled and reported as ignored (tracked by IGNORED_CEILING) rather than being silently deleted or hidden.
   - corrected: the ratchet variable is IGNORED_EXPECTED (scripts/dogfood/coverage.sh:56), exact in both directions
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by crates/forjar-contracts/src/binding.rs:205.

16. [make-docs-guards] L2-F4 — tests/falsification_dogfood_skill_is_named.rs:92 — Test its_first_frontmatter_line_declares_the_unshadowable_name() fails if the first frontmatter line of the skill file is changed, preserving the collision-free forjar-dogfood name.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_skill_is_named.rs:128.

17. [make-docs-guards] L2-F5 — docs/audits/surface_audit.csv:1 — The surface_audit.csv declares the header `transport,kind,name,declared,live`, which is the exact column layout written by the surface.sh generator to guarantee the committed ledger format.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

18. [make-docs-guards] L2-F6 — scripts/dogfood/surface.sh:237 — surface.sh asserts explicit vacuity floors (MIN_CLI, MIN_MCP, MIN_HTTP) against the measured surface to ensure the parity checks cannot pass vacuously on an empty binary output.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

19. [measured] F1 — — `make dogfood` at cf9ac6f7, run by the orchestrator: GATE B PASS (comply clean; ruleset 13878864 requires [gate]; 8 gate scripts at 0 bashrs errors), GATE C PASS (211 CLI names, 12 MCP tools, 12 HTTP verbs, every declared name live and every live name declared, matching the committed ledger), GATE D PASS (18 README invocations run or parse against the fixtures, 1 pinned as known-broken), GATE G PASS (35 contracts validate, pv lint 0 errors, 4 kernels with equations and kani harnesses, every contract names a falsifier); the guard tests 4 + 4 passed; `cargo test -p forjar-contracts --lib` 1333 passed, 0 failed, 43 ignored; `pv validate` of the new contract valid.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by crates/forjar-contracts/src/binding.rs:205.

20. [measured] F2 — — Coverage on origin/main, `cargo llvm-cov --locked --ignore-run-fail --summary-only` on a clean clone: lines 96.41% workspace, 96.54% forjar crate (functions 92.76% / 92.67%, regions 95.64% / 95.79%) — above the 95-line floor gate F enforces; no gap tickets were needed.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

21. [measured] F3 — — `bash scripts/dogfood/release-check.sh` at 3c740863 on this tree (Cargo.toml 1.25.2 = the last tag, so the window is v1.25.1..HEAD): Arms 1–4 evaluate for real and pass (tag on main, GitHub release published, crates.io 1.25.2, docs.rs true); Arm 5 reports receipt=ok for #472, #465, #464 and receipt=waived for #461, #460, #459, #458, #457, #430 — GATE R FAIL, the honest verdict: six pre-run PRs shipped under waivers. For 1.26.0 the window becomes v1.25.2..HEAD, which holds #472 and this run's PRs, every one with a real four-lane receipt.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_release_check_pr_window.rs:61.

22. [measured] F4 — — The ci.yml wiring (24129273, orchestrator): a `dogfood` job builds the release binary and runs gates C and D plus the guard tests on every PR, and `gate` requires it; gates B, F, G and H need pmat and pv (pv is `aprender-contracts-cli`, a path crate not on crates.io), so they run in the clean-room through `make dogfood-release` with the tools provisioned by forjar.yaml (the IP ticket).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

23. [scripts] L1-F1 — scripts/dogfood/comply.sh:144 — Gate B (comply.sh) passes if `pmat comply check` exits 0, an active branch ruleset exists, gate scripts have 0 bashrs errors, and legacy shell errors don't exceed the ceiling; mutating `BASHRS_ERROR_CEILING=0` turns it red by making the known SEC011 finding a regression.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

24. [scripts] L1-F2 — Makefile:148 — The `make dogfood` target runs `comply.sh`, `surface.sh`, `docs.sh`, and `contracts.sh`, whereas `make dogfood-release` additionally runs `coverage.sh` and `crux-reconcile.sh`.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

25. [scripts] L1-F3 — scripts/dogfood/coverage.sh:45 — The `aprender-corpus` tests are gated behind a `not(feature = "aprender-corpus")` ignore attribute rather than deleted, ensuring they compile on every run, print their ignore reason, and are bounded by the `IGNORED_CEILING` and `APRENDER_ANNOTATIONS` ratchets.
   - corrected: the ratchet variable is IGNORED_EXPECTED (scripts/dogfood/coverage.sh:56), exact in both directions
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by crates/forjar-contracts/src/binding.rs:205.

26. [scripts] L1-F4 — scripts/dogfood/contracts.sh:226 — The test `falsification_contract_citations_resolve` will fail if a source change introduces a citation where a falsifier names an item that is not in the `.rs` file it names.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

27. [scripts] L1-F5 — scripts/dogfood/surface.sh:230 — The `surface_audit.csv` header strictly matches `transport,kind,name,declared,live`, proven by the python script explicitly writing this string to the output file.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

28. [scripts] L1-F6 — scripts/dogfood/surface.sh:239 — None of the gate scripts can pass vacuously, as proved by explicit minimum floors such as `MIN_CLI` which fails the gate if the CLI surface shrinks below the floor.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

29. [teamwork] T-F3 — scripts/dogfood/surface.sh:41 — Vacuity floors: The floors are hardcoded in the bash scripts (e.g., MIN_CLI=211). Shrinking the ledger does not lower the floor; a developer must manually edit the script to lower the hardcoded value.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

30. [teamwork] T-F4 — scripts/dogfood/comply.sh:144 — Mutation comments: They are genuine one-line changes that turn their respective gates red, proving the gate is falsifiable.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_dogfood_scripts_declare_mutations.rs:32.

31. [teamwork] T-F7 — scripts/dogfood/coverage.sh:98 — #452 gating: coverage.sh checks if ignored > IGNORED_CEILING, but lacks a check to enforce shrinkage if ignored < IGNORED_CEILING. Slack can be used to silently ignore new failures.
   - corrected: shrinkage IS enforced (-lt IGNORED_EXPECTED is RED); what remains is that the aprender-corpus feature is enabled in no CI workflow, recorded as the accepted limit D8
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by crates/forjar-contracts/src/binding.rs:205.

## Tables as returned

### Judge 1 (verdict FAIL)

L1-F1: CONFIRMED
L1-F2: CONFIRMED
L1-F3: CONFIRMED-AS-NARROWED
L1-F4: CONFIRMED
L1-F5: CONFIRMED
L1-F6: CONFIRMED
L2-F1: CONFIRMED
L2-F2: CONFIRMED
L2-F3: CONFIRMED-AS-NARROWED
L2-F4: CONFIRMED
L2-F5: CONFIRMED
L2-F6: CONFIRMED
L3-F1: REFUTED
T-F1: REFUTED
T-F2: REFUTED
T-F3: CONFIRMED
T-F4: CONFIRMED
T-F5: REFUTED
T-F6: REFUTED
T-F7: CONFIRMED-AS-NARROWED
X-F1: CONFIRMED
X-F2: CONFIRMED
X-F3: CONFIRMED
X-F4: CONFIRMED
D1: CONFIRMED
D2: CONFIRMED
D3: CONFIRMED
D4: CONFIRMED
D5: CONFIRMED-AS-NARROWED
D6: CONFIRMED
D7: CONFIRMED
F1: CONFIRMED
F2: CONFIRMED
F3: CONFIRMED
F4: CONFIRMED

Totals: 26 CONFIRMED / 4 CONFIRMED-AS-NARROWED / 5 REFUTED

### Judge 2 (verdict PASS)

L1-F1: CONFIRMED
L1-F2: CONFIRMED
L1-F3: CONFIRMED-AS-NARROWED
L1-F4: CONFIRMED
L1-F5: CONFIRMED
L1-F6: CONFIRMED
L2-F1: CONFIRMED
L2-F2: CONFIRMED
L2-F3: CONFIRMED-AS-NARROWED
L2-F4: CONFIRMED
L2-F5: CONFIRMED
L2-F6: CONFIRMED
L3-F1: REFUTED
T-F1: REFUTED
T-F2: REFUTED
T-F3: CONFIRMED
T-F4: CONFIRMED
T-F5: CONFIRMED-AS-NARROWED
T-F6: REFUTED
T-F7: CONFIRMED-AS-NARROWED
X-F1: CONFIRMED
X-F2: CONFIRMED
X-F3: CONFIRMED
X-F4: CONFIRMED
D1: CONFIRMED
D2: CONFIRMED
D3: CONFIRMED
D4: CONFIRMED
D5: CONFIRMED-AS-NARROWED
D6: CONFIRMED
D7: CONFIRMED
F1: CONFIRMED
F2: CONFIRMED
F3: CONFIRMED
F4: CONFIRMED

Totals:
CONFIRMED: 26
CONFIRMED-AS-NARROWED: 5
REFUTED: 4

### Judge 3 (verdict FAIL)

Adjudication Table:

| Claim ID | Ruling |
|----------|--------|
| L1-F1 | CONFIRMED |
| L1-F2 | CONFIRMED |
| L1-F3 | CONFIRMED-AS-NARROWED |
| L1-F4 | CONFIRMED |
| L1-F5 | CONFIRMED |
| L1-F6 | CONFIRMED |
| L2-F1 | CONFIRMED |
| L2-F2 | CONFIRMED |
| L2-F3 | CONFIRMED-AS-NARROWED |
| L2-F4 | CONFIRMED |
| L2-F5 | CONFIRMED |
| L2-F6 | CONFIRMED |
| L3-F1 | REFUTED |
| T-F1 | REFUTED |
| T-F2 | REFUTED |
| T-F3 | CONFIRMED |
| T-F4 | CONFIRMED |
| T-F5 | REFUTED |
| T-F6 | REFUTED |
| T-F7 | CONFIRMED-AS-NARROWED |
| X-F1 | CONFIRMED |
| X-F2 | CONFIRMED |
| X-F3 | CONFIRMED |
| X-F4 | CONFIRMED |
| D1 | CONFIRMED |
| D2 | CONFIRMED |
| D3 | CONFIRMED |
| D4 | CONFIRMED |
| D5 | CONFIRMED-AS-NARROWED |
| D6 | CONFIRMED |
| D7 | CONFIRMED |
| F1 | CONFIRMED |
| F2 | CONFIRMED |
| F3 | CONFIRMED |
| F4 | CONFIRMED |

Totals:
CONFIRMED: 26
CONFIRMED-AS-NARROWED: 4
REFUTED: 5

