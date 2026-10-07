# Claims — forjar#607: book v1.33.0 with the tool, and run the cut path without python3

v1.33.0 was tagged and never booked, so the release goal on main is red. The
tool gate T names for that, `scripts/release-goal.sh cut`, ran python3
heredocs to load the ledger and edit rows. And `git tag --sort=-v:refname`
ranks v1.33.0-rc.1 above v1.33.0. Briefed to every lane with the full diff
against main ce9e4906 and the ticket's acceptance criteria.

- C1: the ledger loader is awk (`scripts/dogfood/lib/releases.awk`). It prints the JSON the old loader printed for every form the ledger uses, and refuses any other shape by line number.
- C2: the label, unlabel and book edits in `scripts/release-goal.sh` are awk over the file text, with the old messages and exit codes.
- C3: a release is a vX.Y.Z tag. Every script that picks a release tag goes through `scripts/dogfood/lib/tags.sh`, so an rc tag has no row and opens no window.
- C4: the falsifier discriminates: it goes red with each port reverted, and with the tag rule put back.
- C5: the v1.33.0 row and the v1.34.0 goal were written by `release-goal.sh cut v1.33.0 --next v1.34.0`, not by hand.
- C6: gate T is still red on this head, and the ticket says so: T3 on #646 and #643, then T5 on the dogfood receipt, which was not measured and so is not written.
- C7: the 1.33.0 crux audit was measured with the tag's own gate H, and says it was written after the cut.
