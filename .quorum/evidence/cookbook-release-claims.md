# PMAT-241 — the claims put to the lanes, and where each is anchored

Two rounds. Round one put the ticket's design to three lanes; round two put
the implementation to three more. Both rounds returned 3/3 FAIL. Every claim
below is anchored in a file at the branch tip, and the anchor is the thing a
reader re-runs rather than a paraphrase of it.

## Round two — the claims as written in the brief

1. **`dogfood_req_admits` implements Cargo's requirement rule.**
   `scripts/dogfood/lib/releases.sh:192`. The rule's own documentation is the
   table at `scripts/dogfood/lib/releases.sh:161`. Driven by
   `tests/falsification_release_cookbook_is_part_of_the_release.rs:174`, 35
   rows across caret, tilde and exact.
   REFUTED 3/3 in its first form, which used `dogfood_semver_ge` alone.

2. **The requirement parser reads only `[dependencies]` and
   `[workspace.dependencies]`, and refuses by name what it cannot evaluate.**
   `scripts/dogfood/tagged.sh:214` (the comment cutter) and the section rule
   below it. Driven by
   `tests/falsification_release_cookbook_is_part_of_the_release.rs:129`.
   REFUTED 3/3: a trailing comment was read as the requirement.

3. **The rule's falsifier measures the shipped function and cannot pass
   vacuously.** `tests/falsification_release_cookbook_is_part_of_the_release.rs:174`.
   REFUTED 3/3, in two different directions, both of them about the `sed`
   slice the test used to extract the function from a script.

4. **Ten `v*` tags have been cut since the cookbook's master commit.**
   `docs/roadmaps/releases.yaml:18` and the roadmap row for PMAT-241.
   CONFIRMED 3/3, each lane measuring it independently from the repository's
   own tags.

5. **Nothing in the diff can make gate T pass where it previously failed,
   other than the cookbook arm.** `scripts/dogfood/tagged.sh:182`.
   CONFIRMED 3/3: `crc` initialised, an explicit `return 0` rather than
   falling off the end, `fail` exiting 1 outside any subshell.

## Round one — the claims that round refuted

6. **The requirement regex can read what Cargo writes.** REFUTED: it could not
   read `^1.2` at all and reported "declares no forjar version requirement",
   a false statement. Now
   `tests/falsification_release_cookbook_is_part_of_the_release.rs:129`.

7. **`sort -V` is the right predicate for a Cargo requirement.** REFUTED, and
   independently measured here first: `v2.0.0 >= v1.2` is true where Cargo
   refuses. Now `tests/falsification_release_cookbook_is_part_of_the_release.rs:93`.

8. **A `[dev-dependencies]` entry is not the dependency.** REFUTED of the
   first parser, which ignored sections. Now
   `tests/falsification_release_cookbook_is_part_of_the_release.rs:129`.

9. **"Four tags went out claiming to be dogfooded against it."** REFUTED as a
   count. Measured: ten. Corrected at `docs/roadmaps/releases.yaml:18`, in the
   roadmap row, in the skill and at
   `tests/falsification_release_cookbook_is_part_of_the_release.rs:1`.

10. **"CLAUDE.md and SKILL.md both say a cookbook which cannot use the release
    is bumped as part of the cut."** REFUTED: only the skill said it.
    `CLAUDE.md:61` now says it too.

The adjudicated tally in `cookbook-release-judges.md` is **2 CONFIRMED, 8
REFUTED**. It counts a claim once, not once per lane, and it counts only the
claims whose per-claim verdict this orchestrator can cite: round two's five,
plus the five round one refuted. Round one's confirmations are not enumerated,
because reconstructing them from memory rather than from the lane output is
exactly the kind of number this gate exists to refuse.
