# CRUX Audit — forjar 1.34.2

## Method

**When, and from what.** Written on 2026-11-17 on `release/v1.34.2`, cut from
origin/main `7a8eda78`, against the `[1.34.2]` section of `CHANGELOG.md`. Gate H
(`scripts/dogfood/crux-reconcile.sh`) reads one behaviour paragraph in that
section (the PR Gate trigger change, PMAT-913, #935); it has one row below. The
one-line "also merged" bullets are not behaviour paragraphs for the gate and
have no row.

**Who wrote it.** A Claude model (Sonnet 5.5, a release-prep worker). No quorum
has reviewed this file.

**What was measured, and what was not.**

- The forjar half of the row is carried from the CHANGELOG paragraph and the
  falsifier `tests/falsification_no_pull_request_target.rs` that #935 added. No
  forjar binary or workflow was run for this file.
- Every comparator is `not_measured`. The comparator text is from each project's
  public documentation as the author knows it; no comparator was installed or
  run, and no version is claimed. The surveyed roster has no CI system, so the
  systems named are the nearest supply-chain practices for untrusted build input.

This file records that a comparison was written for each behaviour. It is not
evidence that forjar matches or beats any system named here, and gate H cannot
tell a real comparison from three names typed into a cell.

## Reconciliation

| Behaviour | Systems | What the comparison found |
|---|---|---|
| PR Gate no longer runs on pull_request_target (PMAT-913, #935) | SLSA, Sigstore, in-toto | forjar (carried): `pr-gate.yml` triggers on `pull_request` and runs only for a same-repository head; a new job `no-pull-request-target` fails a PR whose workflow files use the `pull_request_target` trigger. Comparators (`not_measured`): SLSA's build requirements separate the build from untrusted inputs and ask that a build's parameters not be controllable by the submitter beyond those declared; Sigstore and in-toto record who built what from which source, and neither runs a submitter's code with the repository's privileges. The finding: the change moves the PR gate to the same separation, with the trigger no longer granting a fork's code the base repository's token. Difference that stays: a PR from a fork no longer gets the gate at all (same-repository heads only), so fork contributions are not gated by this job. |

## Open

- No comparator was run for this file. An audit that installs the named systems
  at stated versions would turn `not_measured` into measurements.
- The 1.34.2 behaviour changes merged as `cloud_instance`, runner-pool and
  `github_release` work are listed as one-line bullets in the CHANGELOG, as the
  1.34.1 list did for merge-subject items, and so have no gate H row. A reviewer
  may want any of them promoted to a behaviour paragraph with a row.
