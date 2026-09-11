# PMAT-241 — adjudicated claims

Two rounds of three sandboxed lanes. Both returned 3/3 FAIL. Every refutation
below was RE-RUN by the orchestrator before it was acted on, and the
reproduction is what the fix was written against rather than the lane's prose.

## CONFIRMED

1. [count] Ten `v*` tags have been cut since the cookbook's master commit
   `7c100454`, dated 2026-08-29T15:05:21Z — which is why `cookbook_floor`
   starts at v1.29.0 rather than retrofitting a record nobody took.
   - evidence: all three lanes of round two measured this independently from the
     repository's own tags and all three returned 10; the orchestrator measured
     it a fourth time with the tagger dates converted to seconds rather than
     compared as strings, because two of the lanes compared ISO strings whose
     offsets were `+02:00` against a threshold written in `Z` and happened to be
     right for a reason that would not survive a tag cut in another timezone.
   - evidence: the count is quoted in the ledger header and in the roadmap row,
     and the test file header at
     `tests/falsification_release_cookbook_is_part_of_the_release.rs:1` carries
     the same number, so a future correction has one number to change and three
     places that disagree if it is changed in only one.

2. [containment] Nothing in the diff can make gate T pass where it previously
   failed, other than the cookbook arm itself.
   - evidence: all three lanes checked the same three things and agreed: `crc`
     is initialised at its declaration rather than on first assignment, so an
     unset value cannot read as 0; `dogfood_req_admits` ends in an explicit
     `return 0` rather than falling off the end with the exit status of whatever
     ran last; and every `fail` in the new path is a direct call, not one inside
     a subshell whose `exit 1` would be discarded by the parent.
   - evidence: the nineteen cases of `falsification_dogfood_harness_and_quorum`
     and the nineteen of `falsification_release_goals_are_measured` are green
     with and without the change, which is the same containment measured from
     the other side.

## REFUTED

1. [rule] That `sort -V` is the right predicate for a Cargo requirement.
   - evidence: measured by the orchestrator BEFORE the lanes reported it, and
     then by all three of them: `dogfood_semver_ge v2.0.0 v1.2` is true, so the
     gate would have passed a 2.0.0 release against a cookbook requiring `1.2`,
     which Cargo refuses. That is a false green at precisely the release this
     arm exists to catch — the one that breaks the cookbook.
   - corrected: `dogfood_req_admits` applies Cargo's ceiling rule, and
     `tests/falsification_release_cookbook_is_part_of_the_release.rs:174` drives
     35 rows across all three operators, `2.0.0` against `^1.2` among them.

2. [operator] That stripping `^`, `~` and `=` and evaluating the remainder as a
   caret is equivalent to reading the requirement.
   - evidence: Cargo reads `~1.2` as `>=1.2.0, <1.3.0` and `=1.2.3` as exactly
     1.2.3, where a caret runs to 2.0.0. Reading an operator as a caret is
     therefore always WIDER than Cargo admits, and wider is the direction that
     produces a false green: a cookbook pinned `=1.28.0` would have passed
     against a 1.29.0 release it cannot use.
   - corrected: the operator is taken with the requirement and reaches the rule;
     `tests/falsification_release_cookbook_is_part_of_the_release.rs:263` drives
     the whole arm with a tilde and an exact requirement rather than the rule in
     isolation, which is where the defect lived.

3. [parser] That the requirement parser reads the requirement.
   - evidence: measured on `forjar = "1.2" # version = "2.0"`, which produced
     `2.0` — the `version = "…"` pattern matched inside the trailing comment.
     All three lanes of round two found it, and one of them found it with the
     inline-table spelling as well.
   - corrected: awk cuts each line at the first `#` outside a string, the way
     TOML reads one, and
     `tests/falsification_release_cookbook_is_part_of_the_release.rs:263`
     asserts the requirement is the 0.0.1 and not the 9.9 in the comment.

4. [validation] That the versions reaching the rule are versions.
   - evidence: `$((10#3-9 + 1))` is `-5`, so a requirement of `0.0.3-9` produced
     an upper bound of `0.0.-5` and admitted 0.0.4; `0.0.3.4` was silently read
     as `0.0.3`; `01.0.0` was evaluated rather than refused; and `sort -V` puts
     `1.2.4-alpha` ABOVE `1.2.4` where Cargo puts it below. The RELEASED version
     was never checked at all, and it comes straight from the tag name.
   - corrected: both sides must be one to three numeric components with no
     leading zeros, and anything else exits 4 and is refused by name. Seven rows
     of the table at
     `tests/falsification_release_cookbook_is_part_of_the_release.rs:174` are
     those inputs.

5. [falsifier] That the rule's test measures the shipped function.
   - evidence: it sliced the function out of the script with
     `sed -n '/^caret_admits()/,/^}/p'`, and a lane showed both directions of
     failure. Writing `caret_admits () {` with one extra space makes the slice
     empty, so the test goes red while the gate is fine; a function inside a
     string literal would be extracted and defined, so the test goes green while
     the gate is broken. A test whose subject depends on the formatting of the
     file it reads is measuring the formatting.
   - corrected: the rule moved to `scripts/dogfood/lib/releases.sh` beside
     `dogfood_semver_ge`, which is where it belongs anyway, and the test at
     `tests/falsification_release_cookbook_is_part_of_the_release.rs:174` sources
     the library. It still asserts the function is DEFINED before calling it: a
     missing one comes back 127 and `|| return 2` inside the rule would read
     that as "below the requirement", a measurement of nothing.

6. [regex] That the first requirement parser could read what Cargo writes.
   - evidence: round one. The `sed` expression required the value to begin with
     a digit, so `^1.2` — the spelling Cargo writes by default — produced an
     empty requirement and the gate reported "declares no forjar version
     requirement". That is not a miss; it is a false statement about the
     cookbook, in a sentence a reader would act on.
   - corrected: `tests/falsification_release_cookbook_is_part_of_the_release.rs:129`
     drives `^0.0.1` and asserts it is the same requirement as `0.0.1`.

7. [section] That a `[dev-dependencies]` entry is not read as the dependency.
   - evidence: round one. The first parser matched `forjar =` in any table, so a
     dev-dependency pinned `9.9` would be read as the real requirement and the
     release would be measured against a version nothing depends on.
   - corrected: the parser keeps the section and reads only `[dependencies]` and
     `[workspace.dependencies]`;
     `tests/falsification_release_cookbook_is_part_of_the_release.rs:129` puts a
     `9.9` dev-dependency above a `0.0.1` real one and asserts the real one wins.

8. [wording] That "four tags went out claiming to be dogfooded against it" and
   that "CLAUDE.md and SKILL.md both say a cookbook which cannot use the release
   is bumped as part of the cut".
   - evidence: round one refuted both. The count is ten, not four. And only the
     skill carried the bump clause; `CLAUDE.md` did not, so the receipt claimed
     a document said something it did not say — the cheapest kind of false claim
     to make and the hardest to notice, because nobody re-reads the file they
     just cited.
   - corrected: both corrected in place, and the count is now measured in
     seconds rather than by comparing ISO strings with mixed offsets.
