# Claims: forjar#607, the T3 rule. One of gate T's remaining reds on v1.33.0, fixed in the tool

The ticket's full text, verbatim from the roadmap: .quorum/evidence/PMAT-607-ticket.md.

This PR books v1.33.0 with `release-goal.sh cut`, and its claim C6 says gate
T stays red. T3 is red on #643 and #646, which name their tickets only as a
bare `#642` and `#624`; the roadmap rows PMAT-642 and PMAT-624 declare
`github_issue: 642` and `624`. Gate T cannot name v1.33.0's tickets while it
reads those two PRs as unticketed, and naming them is this ticket's own title:
"book v1.33.0 with the tool gate T names".

- B1: `dogfood_pr_tickets` (shared by gates A and T), when branch, title and body name no `PMAT-<n>`, reads the FIRST bare `#N` in the title, then the body, and resolves it only through the roadmap row whose `github_issue:` is N.
- B2: with no such row the PR stays unticketed (red). The next `#N` is never tried.
- B3: two rows filed from one issue resolve to neither (red), as a twice-declared alias does.
- B4: a PMAT id anywhere is read before any `#N`.
- B5: `forjar#5`, `a/b#5`, `PR-#5`, `##5`, `v1.#5` and `x_#5` are not bare.
- B6: the falsifier discriminates. With `scripts/dogfood/lib/window.sh` as at main, 5 of its 8 tests go red. The 3 that stay green are the ones whose answer the old rule already gives: `an_issue_with_no_row_stays_red` and `the_next_issue_is_never_tried` (the PR stays unticketed) and `a_pmat_id_is_read_before_any_issue` (the PMAT id wins). Dropping the glue exclusion turns `every_glued_form_is_not_bare` red.

This widens what gates A and T accept, so it carries a non-author sign-off
bound to the final commit.
