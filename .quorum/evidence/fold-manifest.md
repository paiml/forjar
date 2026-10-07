# Fold manifest: #695

forjar is over its open-PR cap, and it stays over the cap after this PR
merges, so a second PR for this ticket could never open on its own. The cap's
remedy is to fold the work into the open PR of the same ticket. That is what
this PR now carries.

| Folded from | Commits | Ticket |
|---|---|---|
| branch `fix/607-t3-github-issue` (never pushed, never opened) | be506c19, 421e1082, f0b08845, 66c8891c | PMAT-607 |

How it was folded: a forward `git merge --no-ff` into `fix/607-book-releases`
(a02ea4a7). No rebase and no force-push. Nothing already on the PR changed.

What the fold adds:

- the T3 rule: a PR that names no PMAT id resolves a bare `#N` through the row filed from issue N (claims: `PMAT-607-p1-claims.md`);
- phase 2: a booked row is amended, never edited (`release-goal.sh amend`), and gate T reads the row plus its amendments (claims: `PMAT-607-p2-claims.md`).

The booking path is unchanged. `cut` still books below `releases:`
(`a_cut_after_an_amendment_books_below_it`). The ledger this PR's cut wrote
for v1.33.0 is byte-identical, and it loads under the new shape check.

Reviews: there is one ticket, so there is one quorum round over the whole
diff against main, bound to the final commit, plus a non-author sign-off.

Measured on the fold head a02ea4a7:

- `cargo fmt --check` and `cargo clippy --all-targets -D warnings` are clean.
- 12 suites pass: the three PMAT-607 falsifiers (8, 6 and 5 tests) and the nine release-goal and dogfood suites beside them.
