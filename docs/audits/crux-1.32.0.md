# CRUX Audit — forjar 1.32.0

## Method

**Who wrote this, and from what.** Like the 1.30.0 and 1.31.0 audits, this one
was written by the release orchestrator (Claude Opus 5) directly, from
documentation memory, with no network access and no live invocation of any
reference system. The 1.29.0 audit was surveyed by an `agy` quorum lane; this
one was not, and that difference is recorded rather than implied.

Every claim about a third-party system below is marked `[X]`: asserted from
documentation memory, not measured against a running instance. Every claim about
forjar is measured in this release's window, and the measurements live in the
receipt its ticket names.

**What this audit can and cannot show.** It shows that the behaviour shipping in
1.32.0 has been held against at least three systems that solve the same problem,
and what that comparison found. It cannot show that the comparison is correct in
detail — `scripts/dogfood/crux-reconcile.sh` says the same about itself.

**What 1.32.0 is about.** One behaviour, and it is the THIRD signature of
paiml/infra#605 to be answered: a state file that names a writer it did not have.
The CHANGELOG paragraph for this release says "third signature" in PMAT-565's own
words, and 1.31.0's says "second"; an earlier draft of this file said "fourth"
and a review lane caught it against those two sentences.
The window is small on purpose, and the cut is LATE rather than on time. 1.31.0
was cut 2026-09-16 and this is 2026-09-20: four days, against the
`cadence_days: 2` rule in `docs/roadmaps/releases.yaml`, so it went out 50 hours
overdue. An earlier draft of this paragraph called it "a cadence release cut two
days after 1.31.0", which the CHANGELOG's own two dates in this same diff refute;
a review lane quoted it back. The discipline the cadence exists to enforce is
still the point — one fleet-affecting fix ships as soon as it is ready rather
than waiting for company — but this release is evidence of the rule being missed,
not of it being kept.

## The comparison

| Behaviour | Systems surveyed | What the comparison found |
|---|---|---|
| (1) The per-machine lock names the binary that wrote it, on every write, preserving the original creator, and lock --restamp converges a state dir (PMAT-565, #565) | Terraform, Kubernetes, SLSA, Nix, Puppet | The field is unanimous that a state artifact must name its writer, and splits on two questions forjar had answered by accident rather than by design: **creator or last writer**, and **is a mismatch enforced**. Terraform is the closest analogue and the sharpest contrast on both counts: its state file carries a top-level `terraform_version`, it is rewritten to the CURRENT binary's version on every state write, and a state written by a newer version is REFUSED rather than silently parsed `[X]` — record, refresh, enforce. Kubernetes answers the "creator or last writer" question at a finer grain than forjar does: server-side apply records `metadata.managedFields`, naming which manager owned which field on which operation, so the object carries a per-field writer history rather than one name `[X]`. SLSA makes the writer's identity the whole point of the artifact — a provenance attestation's `builder.id` exists to say which tool, at which version, produced this thing, and a consumer that cannot resolve it is expected to reject the artifact `[X]`. Nix sidesteps the question by construction: the builder is part of the derivation's input closure, so a different builder produces a different store path and the two never occupy one name `[X]`. Puppet is the weakest of the five and the most like where forjar was: reports carry `puppet_version`, but the catalog and the agent state are not versioned artifacts a later run must reconcile against `[X]`. **forjar's defect was worse than any of these, and not because it recorded nothing.** It recorded the CREATOR under a field named `generator`, on a file whose `generated_at` was refreshed on every write — so the pair was a claim no version of forjar could have made: four fleet locks under one 1.30.0 binary read `forjar 1.1.1`, `1.13.1`, `1.27.0` and `1.10.0` with current timestamps. A field that is stale while its neighbour is fresh is worse than an absent field, because an absent field is read as unknown and a stale one is read as measured. The repair takes Terraform's first two moves — stamp the writing binary on every write — and adds what Terraform does not keep: the value being replaced moves into `created_by` the first time, so the creator is named rather than erased, which is the Kubernetes instinct that a writer history is worth more than a writer. Terraform's third move, refusing a mismatch, is deliberately NOT taken here and is tracked separately as #579, because refusing on a version mismatch is what `apply`/`plan` should do and a lock writer is the wrong place for it. `lock --restamp` exists because the other four systems all converge their metadata on the next ordinary operation, and forjar's would have converged only on the next incidental apply of each machine — a fleet would have carried the wrong answer for as long as it happened not to be applied. |

## What is not reconciled here

The row above is the single behaviour bullet in the 1.32.0 CHANGELOG section,
which is the whole set `crux-reconcile.sh` requires. Everything else in the
window is bookkeeping with no behaviour attached and therefore no comparison to
hold it against:

- **#577** (PMAT-576) booked v1.31.0's ledger row and declared v1.32.0.
- **#583** (PMAT-579, PMAT-581, PMAT-582, PMAT-587) registered rows for defects
  found auditing a fleet pin under 1.31.0. Registration rows ship no
  implementation by their own acceptance criteria, so none of them is a
  behaviour this release claims.

## What the window does NOT contain, and why it is worth saying

Two fleet-wide defects were reproduced during this cut and are NOT in 1.32.0:

- **#590** — `apply --refresh` never consults `completion_check` for a resource
  containing a template, so every guard on the fleet of the shape CLAUDE.md
  prescribes is permanently RED while satisfied. Reproduced against the released
  1.31.0 binary with a two-manifest fixture that differs by one `{{params.home}}`;
  narrowed to `refresh_seed::check_passes_on`, not yet to a root cause.
- **#591** — nothing compares LIVE against the DECLARATION. `drift` is
  live-versus-lock and `plan` is config-versus-lock, so a declared `version:` is
  an input to neither verdict, which is how a fleet pin sat two minors stale with
  both tools behaving exactly as documented.

Both are targeted at 1.33.0, and they are held back for DIFFERENT reasons — an
earlier draft gave them one reason and a review lane refuted it, correctly:

- **#590 has a reproduction and no root cause.** Three `false` returns in
  `check_passes_on` are collapsed into one, and which of them the fixture is
  taking has not been measured. A fix for a fleet-wide guard path that cannot
  explain itself is not a release candidate.
- **#591's cause IS named**, to the line: the `cargo` and `uv` drift observables
  discard the installed version (`src/resources/package/mod.rs:231`,
  `cargo.rs:405`), and nothing anywhere compares live against the declaration.
  It is held back because it is two independent halves — repairing the
  observables closes live-versus-lock, probing the declaration closes
  live-versus-config — and shipping either alone produces an instrument that
  would have missed the incident that produced it.

Neither was held back for lack of importance. Shipping 1.32.0 two days past its
due date with one proven fix is the cadence being enforced late, not waived.
