# Quorum evidence — PMAT-159 / v1.25.2 — the twenty-one adjudicated claims

Three claim lanes returned eighteen claims (A1-A6 the gate change, B1-B6 the release
commit and the CI step, C1-C6 falsifier honesty). The assembler added three measured
claims (M1-M3) after running the anchor rule by hand. Three refuters attacked all
twenty-one: R1 and R2 in wave 2, and R3 — whose first run returned claim headlines
with no verdict, reason or citation — on a wave-4 rerun that returned twenty-one real
adjudications, two of whose three citations a fourth judge then found do not exist.
Three judges adjudicated independently and agreed on every one of the twenty-one,
including both refusals; the fourth judge, J4, re-adjudicated exactly the three claims
the R3 rerun attacked (A2, A6 and B1) and upheld the panel on all three, so the tally
is unchanged. This paragraph is the assembler's correction of a stale draft header
that said the third refuter was not counted; lanes and judges record the rerun.

## CONFIRMED

1. [A1 — upheld by three judges] -AS-NARROWED. The remaining checks force the forgery to take the shape of reviewable, committed text within the PR, preventing complete fabrications. Citation: tests/falsification_quorum_anchors_release_shaped.rs:302.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

2. [A2 — upheld by three judges, and again by a fourth after a late refutation] -AS-NARROWED. The substance holds that the rules operate at the file level (`p in touched`) rather than the line level, meaning old lines in renamed files could theoretically anchor. However, the reasoning about Cargo.toml:3 is flawed: Cargo.toml existed at the merge-base, so it is evaluated by `anchors_at_base`, not the added-file rule. Citation: tests/falsification_quorum_anchors_release_shaped.rs:297.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

3. [A3 — upheld by three judges] -AS-NARROWED. Any release branch can trivially satisfy the 33% floor using the root files, which is an acceptable trade-off to avoid using the waived mechanism. Citation: tests/falsification_quorum_anchors_release_shaped.rs:302.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

4. [A4 — upheld by three judges] -AS-NARROWED. The test assertions are correct and discriminating. The check for the 9999 boundary is now exactly at tests/falsification_quorum_anchors_release_shaped.rs:404.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

5. [A5 — upheld by three judges] -AS-NARROWED. Threading `head` correctly avoids the self-referentiality problem. The dynamic pass `&f.head` via run_evidence is at tests/falsification_quorum_anchors_release_shaped.rs:277.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

6. [A6 — upheld by three judges, and again by a fourth after a late refutation] -AS-NARROWED. The paths are symmetric and correctly share the `count(b"\n") + 1` formula which allows citing an empty trailing line. However, the reasoning about `continue` being structurally necessary is wrong because the refactor completely eliminated the inline `continue`. Citation: tests/falsification_quorum_anchors_release_shaped.rs:404.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

7. [B1 — upheld by three judges, and again by a fourth after a late refutation] Cargo.toml line 3 bumps to 1.25.2, Cargo.lock line 1171 updates the forjar version, and CHANGELOG.md adds a 1.25.2 entry with a plausible date. Citations: Cargo.toml:3, Cargo.lock:1171, CHANGELOG.md:10.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

8. [B2 — upheld by three judges] The lockfile change moves exactly one line (the forjar version) and no other dependency versions, checksums, or package entries were touched. Citation: Cargo.lock:1171.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

9. [B3 — upheld by three judges] -AS-NARROWED. A tag `v1.25.2` would satisfy the regex and equality check in `release.yml`. However, `.github/workflows/release.yml` is not in this diff and this is a pre-existing property out of scope for the current branch's defects. Citation: Cargo.toml:3.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

10. [B4 — upheld by three judges] The changelog entry overclaims by describing fixes already in main, and misattributes the sudo-transport fix to PMAT-159 (which is the version coherence ticket). Citation: CHANGELOG.md:49.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

11. [B5 — upheld by three judges] -AS-NARROWED. The CI step safely adds the check without weakening existing ones. However, the correct citation pinning the `examples-validate` job and its use of `--locked` is tests/falsification_version_matches_manifest.rs:57.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

12. [C1 — upheld by three judges] The citations correctly match the vacuous legs and the one discriminating leg (the changelog). Citations: tests/falsification_version_matches_manifest.rs:132, tests/falsification_version_matches_manifest.rs:149, tests/falsification_version_matches_manifest.rs:177, tests/falsification_version_matches_manifest.rs:194.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

13. [C2 — upheld by three judges] The new documentation is honest and explicitly states the vacuity of the tests under a bare `cargo test`. Citation: tests/falsification_version_matches_manifest.rs:38.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

14. [C3 — upheld by three judges] The lockfile invariant is enforced by cargo itself under `--locked`, causing a build error before the test can run. Citation: tests/falsification_version_matches_manifest.rs:51.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

15. [C5 — upheld by three judges] The old assertion in integration_smoke.rs passed on the binary's name alone. The new assertion is robust and admits its limitations. Citations: tests/integration_smoke.rs:20, tests/integration_smoke.rs:31.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

16. [C6 — upheld by three judges] The suite is a net improvement that adds two genuinely falsifiable propositions: a broken `--version` renderer and an undocumented release. Citations: tests/integration_smoke.rs:27, tests/falsification_version_matches_manifest.rs:194.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

17. [M1 — upheld by three judges] `anchors_at_base` bounds-checks against the file length at the merge-base. A file the branch appends to fails the gate if a citation points to the added lines. This is an inconsistency the branch should close, as appended lines are exactly as reviewable as added files. The 500-line ceiling is a convenient excuse for not appending. Citation: tests/falsification_quorum_gate_reads_the_pushed_ref.rs:43.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

18. [M2 — upheld by three judges] The refactor preserved behavior exactly. `p in touched` is still required on both paths, the added-file path still bounds-checks against HEAD and dies, out-of-range citations in base die, and the 33% floor arithmetic is unchanged. Citation: tests/falsification_quorum_anchors_release_shaped.rs:390.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

19. [M3 — upheld by three judges] The four moved tests still cover what they did and assert on specific messages. However, nothing pins the BASE-side bounds check — the `die` in `anchors_at_base`. This leaves the base-side bounds check as an untested branch of a security check. Citation: tests/falsification_quorum_anchors_release_shaped.rs:390.
- evidence: adjudicated against the tree at the commit under review rather than against the receipt prose; both refuters that returned content attacked this claim and neither overturned it.

## REFUTED

1. [B6 — refused by three judges] The claim that the target was running nowhere prior to this CI step is misleading because the target file `tests/falsification_version_matches_manifest.rs` is added entirely by this diff, so the target did not exist and could not have run in any CI job before this. Citation: tests/falsification_version_matches_manifest.rs:1.
- corrected: the claim as posed does not survive contact with the file; the sentence above is the correction a reader can act on, and it is the reason this item sits under REFUTED rather than being quietly dropped.

2. [C4 — refused by three judges] The text contradicts the claim that `starts_with` would admit `1.25.20`. The expected head includes the closing bracket `]`, so `starts_with("## [1.25.2]")` will safely reject `## [1.25.20]`. Citation: tests/falsification_version_matches_manifest.rs:200.
- corrected: the claim as posed does not survive contact with the file; the sentence above is the correction a reader can act on, and it is the reason this item sits under REFUTED rather than being quietly dropped.

