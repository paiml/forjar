# Crux — how other review gates bind a claim to a diff line

This survey was briefed to refuter R3, which returned nothing, and was reassigned to
judge J3. That reassignment is recorded rather than hidden: the survey below is one
readings work, not three, and should be weighted accordingly.

- X1: Danger (danger-js / danger-ruby) falls back to posting the comment at the file level or as a general PR comment if the cited line is not in the diff. GitHub's API rejects inline comments outside the diff, so Danger handles this by gracefully degrading the anchor rather than dropping the message entirely.

- X2: reviewdog's `filter_mode` dictates anchoring strictly against the diff. In `added` mode, it requires the linter finding to exactly match a line the diff added; findings on untouched lines are filtered out completely, making it highly granular.

- X3: Gerrit anchors comments to a specific patch set and line. When a new patch set is uploaded and the line no longer exists, the comment is not lost but remains attached to the older patch set, and is surfaced as an unresolved thread (often at the file or change level) rather than being physically anchored to the new tree.

- X4: semantic-release (`commit-analyzer`) binds its release decisions entirely to the text of the commit message (e.g., Conventional Commits). It does not cite lines or check the commit message against the diff at all. The absence of code anchoring is the finding: it relies 100% on trusting the author's prose.

- X5: CRUX VERDICT: MATCHES ON GRANULARITY, STRICTER THAN THE FIELD ON PROVENANCE. (i) GRANULARITY: reviewdog's `added` mode is line-exact and strictly stricter than forjar's file-level rule. However, forjar's looseness is correct: its subject is a review claim about a conceptual change, not a syntax error at a point, so citing `Cargo.lock:1` to assert a lockfile update is valid. (ii) PROVENANCE: None of the surveyed systems derive trust from authorship-independence; they all trust the author's code or commit messages and rely on human reviewers. Forjar's old rule was uniquely stricter than the field. By relaxing the rule for added files, forjar has moved TOWARD the industry standard, not below it, while still retaining base-tree anchoring for pre-existing files.

