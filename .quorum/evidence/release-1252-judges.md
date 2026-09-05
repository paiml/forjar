# Judges — PMAT-159 / v1.25.2

Four judges. Three adjudicated all twenty-one claims; a fourth, J4, was added in a later
wave to weigh a late refuter against them. Kill rule: majority of three.
The outcome was unanimous on every claim and on the overall verdict, which is worth
stating plainly because unanimity across three independent readings is either strong
agreement or a sign the panel was asked a question with an obvious answer.

## Where the judges had to break a tie the refuters left

- A2 — R1 said REFUTED, R2 said CONFIRMED-AS-NARROWED. All three judges resolved to
  CONFIRMED; one built a real git repository to check whether a renamed file actually
  reaches the added-file path.
- A6 — both refuters attacked the lanes reasoning about the continue statement on
  different grounds. The judges upheld the CONCLUSION while agreeing the lanes
  reasoning for it was wrong, and one checked the plus-one arithmetic against a file
  that ends in a newline.
- B6 — both refuters said REFUTED for different reasons; all three judges refused it,
  on R2s ground rather than R1s.
- C4 — R1 narrowed, R2 refuted over a prefix collision. All three judges REFUTED the
  lane claim, and all three gave the same reason, which is that the closing bracket in
  the expected heading defeats the collision the lane was worried about.

## The two refusals, as adjudicated

- B6: REFUTED. The claim that the target was running nowhere prior to this CI step is misleading because the target file `tests/falsification_version_matches_manifest.rs` is added entirely by this diff, so the target did not exist and could not have run in any CI job before this. Citation: tests/falsification_version_matches_manifest.rs:1.

- C4: REFUTED. The text contradicts the claim that `starts_with` would admit `1.25.20`. The expected head includes the closing bracket `]`, so `starts_with("## [1.25.2]")` will safely reject `## [1.25.20]`. Citation: tests/falsification_version_matches_manifest.rs:200.

## Overall verdicts

- J1 — VERDICT: PASS. The release artifacts (manifest, lockfile, changelog) are coherent and the new CI step and tests genuinely improve coverage. The defects found (the appended-line hole in the gate, an untested base-side bounds check, and minor changelog misattributions) do not break the release artifact or end-user code. Therefore, none of these findings are a BLOCKER for shipping v1.25.2.

- J2 — VERDICT: PASS. The branch correctly fixes PMAT-159 by loosening the quorum gate for files that legitimate release commits always modify, without turning off the verification shape entirely. The new version coherence assertions meaningfully protect the release pipeline against historical omissions (like a missing lockfile update) and document their own limitations transparently. There are no blockers for shipping v1.25.2.

- J3 — VERDICT: PASS. The branch successfully implements the version manifest checks and repairs the quorum gate to allow release commits to anchor their claims without breaking the required security properties. The refactor of `quorum_evidence.py` perfectly preserves existing behavior while smoothly introducing the added-file rule. While there is an untested branch in the base-side bounds check and the changelog entry has a minor misattribution, neither is a blocker for shipping v1.25.2. The release can proceed safely.


## J4 — a fourth judge, added to weigh a late refuter

The third refuter, R3, returned nothing usable on its first run and was rerun after the
panel had finished. The rerun REFUTED three claims all three judges had CONFIRMED — A2,
A6 and B1 — so a fourth judge was run over exactly those three, given the lane claim,
R1, R2, R3 and all three panel verdicts verbatim, the measured length of every citable
file, and an instruction to verify every line number in every argument itself.

J4 upheld the panel on all three, so the tally is unchanged at nineteen CONFIRMED and
two REFUTED, and no evidence item required rewriting. What it found about the late
refuter is the part worth keeping:

- A2 — CONFIRMED, narrowed as the panel narrowed it. The added-file rule is file-level,
  so the substance survives; the lane's `Cargo.toml:3` example is wrong because that
  file exists at the merge-base and resolves through the base rule. R3 added nothing:
  it repeated the attack R1 had already made on the example, and cited line 530 of an
  added test file that is 409 lines long. Citation: tests/falsification_quorum_anchors_release_shaped.rs:297.
- A6 — CONFIRMED, narrowed. The two paths are symmetric and the plus-one overcount is a
  convention inherited from the base branch, not something this branch introduced. R3
  repeated the same `continue` syntax argument the panel had already set aside, and
  cited line 624 of the same 409-line file. Citation: tests/falsification_quorum_anchors_release_shaped.rs:391.
- B1 — CONFIRMED outright. J4 read the lockfile at both revisions: Cargo.lock:1171
  carries the forjar package version at the merge-base and at HEAD, line 1170 is the
  package name and line 1172 opens the dependency list. R3's refutation asserted the
  opposite and is simply false. Citations: Cargo.toml:3, Cargo.lock:1171, CHANGELOG.md:10.

J4 was also asked to grade the rerun itself, because a quorum that counts a refuter must
be able to say whether that refuter was worth counting. Of the three R3 citations it
checked, one exists and two do not, and the one that exists does not say what R3 says it
says. That is recorded here rather than smoothed over: the refuters-per-claim count of
three is a count of lanes that returned real adjudications, not a claim that all three
attacks were sound.
