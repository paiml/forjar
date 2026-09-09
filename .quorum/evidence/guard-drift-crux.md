# Quorum evidence — PMAT-215 — CRUX

No new CRUX survey was run for this ticket, and that is a deliberate NotRun rather than an omission.

`docs/audits/crux-1.27.0.md` already surveyed this exact behaviour one release earlier, under the heading "A task whose command exits non-zero": Terraform, Puppet, Chef and Ansible, with the finding that three of the four keep no failure verdict at all, so the latch is a defect they cannot have, and Terraform is the one that persists a verdict with a forced rebuild as its way back.

This ticket is the second half of that same issue and adds no new behaviour class to compare. Re-running the same four systems against the same question would produce the same rows, and a survey that cannot come out differently is decoration.

What this ticket does add is a distinction worth recording for the next survey: forjar now separates an ASSERTION, which can be evaluated at any time, from a BASELINE, which a failed apply never wrote. Whether the surveyed systems draw that line explicitly is a real question and belongs in the next release's CRUX, not backfilled into this one.
