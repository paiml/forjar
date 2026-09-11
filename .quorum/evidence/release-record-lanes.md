# PMAT-231 — quorum lanes (agy, --sandbox, writes=false)

Six lanes over two rounds, each in its own `git clone --shared` with `origin/main` pinned to aff6be71, each measuring the live release, crates.io, docs.rs and the workflow runs rather than reading the receipt back to itself. Each lane was also asked to read the whole receipt as a hostile reader and quote any sentence a reader could check and find false.


## round 1 (fcbc5b7e)

### round 1 lane 1 — verdict FAIL (conv-01453612, 1 turn(s), 0s)

C1 REFUTED, C2 CONFIRMED, C3 CONFIRMED, C4 CONFIRMED, C5 CONFIRMED, C6 CONFIRMED, C7 CONFIRMED, C8 REFUTED, C9 CONFIRMED, C10 REFUTED

- C1 REFUTED: crates.io reports created_at 2026-09-10T16:12:45.938977Z, not 2026-09-10T16:12:45Z. The timestamp was rounded in the receipt.

- C8 REFUTED: PMAT-231's notes do not carry an orch-basis: token. What is actually true is that notes is null for PMAT-231 in docs/roadmaps/roadmap.yaml.

- C10 REFUTED: The first tag run (34500075606) had exactly 3 jobs cancelled at 16:53:40Z (the build-binaries jobs for aarch64-unknown-linux-gnu, x86_64-unknown-linux-musl, aarch64-unknown-linux-musl), not 4 jobs.

### round 1 lane 2 — verdict FAIL (conv-f478c764, 1 turn(s), 0s)

C1 CONFIRMED, C2 CONFIRMED, C3 CONFIRMED, C4 CONFIRMED, C5 CONFIRMED, C6 CONFIRMED, C7 CONFIRMED, C8 REFUTED, C9 CONFIRMED, C10 REFUTED

- C8 REFUTED: PMAT-231's row in `docs/roadmaps/roadmap.yaml` has `notes: null` and does not carry an `orch-basis:` token. Only PMAT-230 carries `orch-basis: release`.

- C10 REFUTED: `gh api /repos/paiml/forjar/actions/runs/34500075606/attempts/1/jobs | jq '.jobs[] | select(.conclusion == "cancelled")'` output reveals exactly 3 jobs (build-binaries for aarch64-unknown-linux-gnu, x86_64-unknown-linux-musl, aarch64-unknown-linux-musl) were cancelled at 16:53:40Z, not 4.

### round 1 lane 3 — verdict FAIL (conv-2292ace1, 1 turn(s), 0s)

C1 REFUTED, C2 CONFIRMED, C3 CONFIRMED, C4 CONFIRMED, C5 CONFIRMED, C6 CONFIRMED, C7 CONFIRMED, C8 REFUTED, C9 CONFIRMED, C10 REFUTED.

- C1 REFUTED: crates.io reports created_at 2026-09-10T16:12:45.938977Z (fractional seconds present in API response), not 2026-09-10T16:12:45Z. Other identity facts (git tag, gh release, doc_status) match exactly.

- C8 REFUTED: PMAT-231 has `notes: null` in `roadmap.yaml`, meaning it does not carry an `orch-basis:` token. The other rows do carry `release:v1.29.0` and a `kind:` field.

- C10 REFUTED: `gh api` for run 34500075606 attempt 1 shows exactly 3 jobs were cancelled at 16:53:40Z, not 4. The other statements (homebrew formula, cargo mutants) are accurate.


## round 2 (f2c5cb58)

### round 2 lane 1 — verdict FAIL (conv-4f8f5c5f, 1 turn(s), 0s)

C1 CONFIRMED, C2 CONFIRMED, C3 CONFIRMED, C4 CONFIRMED, C5 CONFIRMED, C6 CONFIRMED, C7 CONFIRMED, C8 CONFIRMED, C9 CONFIRMED, C10 CONFIRMED. HOSTILE: The receipt falsifies the output of Gate R in its summary table. It claims the gate output a perfectly clean 'PASS' sentence, when the verbatim log shows it actually printed a buggy 'PENDING until the tag is cut' message, thus making the gate appear better than the record supports.

- HOSTILE: "| R, post-tag | `scripts/dogfood/release-check.sh` | PASS — 13 PRs since v1.27.0 all carry a receipt; the tag is on origin, the release is published, crates.io and docs.rs serve it |"

### round 2 lane 2 — verdict PASS (conv-0fe0e63a, 1 turn(s), 0s)

C1 CONFIRMED, C2 CONFIRMED, C3 CONFIRMED, C4 CONFIRMED, C5 CONFIRMED, C6 CONFIRMED, C7 CONFIRMED, C8 CONFIRMED, C9 CONFIRMED, C10 CONFIRMED. HOSTILE: I found a sentence that makes the author look better than the record supports by claiming a step was made necessary by a defect when it was actually taken by design.

- HOSTILE: "Two of the last three steps were done by hand and both are named below with the defect that made them necessary: the workflow declined to un-draft a release it had not created (PMAT-232), and promotion from prerelease to full release is the operator's step by design, taken after `make dogfood-published VERSION=1.28.0` passed."

### round 2 lane 3 — verdict FAIL (conv-41808662, 1 turn(s), 0s)

C1 CONFIRMED, C2 CONFIRMED, C3 CONFIRMED, C4 CONFIRMED, C5 CONFIRMED, C6 CONFIRMED, C7 REFUTED (claim truncated the actual log output), C8 REFUTED (several PMAT rows lack a notes field entirely), C9 CONFIRMED, C10 CONFIRMED. The receipt's gate R table row falsely claims it checked origin, crates.io, and docs.rs, but the log shows it printed PENDING.

- C7 REFUTED: The claim truncates the output; gate R actually printed 'GATE R PASS pre-tag: ... PENDING until the tag is cut: no docs/audits/crux-1.28.0.md: Cargo.toml is still at v1.28.0's version (1.28.0), no release is being cut'

- C8 REFUTED: PMAT-231, PMAT-232, and PMAT-234 do not carry a notes field at all, let alone a non-null one.

- HOSTILE: "| R, post-tag | `scripts/dogfood/release-check.sh` | PASS — 13 PRs since v1.27.0 all carry a receipt; the tag is on origin, the release is published, crates.io and docs.rs serve it |"


## reduction

Round 1: 0 PASS / 3 FAIL, three refutations, every one reproduced and corrected. Round 2: 1 PASS / 2 FAIL — all ten claims confirmed by two lanes, one lane refuted two (one real, one that does not reproduce), and two lanes independently found the same honesty defect in the receipt's gate table. Both were fixed. No third round: the corrections are the receipt's own text and the facts under them were measured by six lanes and by the orchestrator.
