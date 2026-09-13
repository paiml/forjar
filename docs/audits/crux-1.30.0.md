# CRUX Audit — forjar 1.30.0

## Method

**Who wrote this, and from what.** The 1.29.0 audit was surveyed by an `agy`
quorum lane. This one was not: it was written by the release orchestrator
(Claude Opus 5) directly, from documentation memory, with no network access and
no live invocation of any reference system. That is the same class of source the
1.29.0 lane used and it is recorded here rather than implied, because the
difference between "a lane surveyed this" and "the author wrote it" is exactly
the kind of provenance claim this repository refuses to leave unstated.

Every claim about a third-party system below is marked `[X]`: asserted from
documentation memory, not measured against a running instance. Every claim about
forjar is measured in this release's window and the measurements live in the
receipts each row's ticket names.

**What this audit can and cannot show.** It shows that each behaviour shipping
in 1.30.0 has been held against at least three systems that solve the same
problem, and what that comparison found. It cannot show that the comparison is
correct in detail — `scripts/dogfood/crux-reconcile.sh` says the same about
itself, and it is right to.

## The comparison

| Behaviour | Systems surveyed | What the comparison found |
|---|---|---|
| (1) forjar drift reported a host it could not reach as drifted (PMAT-549) | Ansible, Kubernetes, Terraform, Puppet | Every one of them already has the third state forjar lacked. Ansible's play recap counts `unreachable` separately from `failed` and from `ok` — a host that did not answer is not a host that answered badly `[X]`. Kubernetes gives a Node's `Ready` condition the value `Unknown` when the kubelet stops reporting, and the scheduler treats that as not-known rather than not-ready `[X]`. `terraform plan` fails when it cannot refresh state through a provider, rather than emitting an empty diff `[X]`. Puppet reports carry an unresponsive-node status distinct from a failed run `[X]`. forjar had two states where the field has settled on three, and the missing one is the dangerous direction: silence read as an answer. |
| (2) The latest release is the latest release (PMAT-534) | Docker, SLSA, Sigstore, in-toto | The mutable-pointer hazard is well known and the field's answer is "do not trust the pointer, verify the artifact". Docker's `latest` tag is just a tag, and the standing advice is never to deploy from it `[X]`. SLSA provenance binds an artifact to the build that produced it, so a pointer resolving to an older artifact breaks the chain a consumer follows `[X]`; in-toto layouts have the same property step by step `[X]`. Sigstore verification is per-artifact, which means a stale pointer yields a *valid* signature over the *wrong* release `[X]`. forjar's gap was narrower and worse: it published the pointer itself and never asserted where it resolved. |
| (3) A PR and its commits name one ticket (PMAT-540) | Kubernetes, Chef, Terraform | All three check the pull request and not the branch. Kubernetes' prow requires a `release-note` block and `/kind` labels on the PR, enforced by bot `[X]`. Chef's Expeditor reads a per-PR changelog label `[X]`. Terraform requires a changelog entry file per PR `[X]`. None of the three cross-check the PR against the commits it contains, which is the arm forjar added: the id the PR is filed under is compared against what the merge actually claims. |
| (4) CI selects the dogfood gate the change can move (PMAT-542) | Bazel, Nix, Kubernetes | Selective execution is standard, and forjar's is the coarsest of the three. Bazel derives the affected test set from the build graph, so selection is exact `[X]`. Nix rebuilds exactly the derivations whose input hashes changed `[X]`. Kubernetes' prow uses `run_if_changed` path regexes, which is the same mechanism forjar now uses `[X]`. The comparison's finding is that a path-class classifier is the weakest of the three and is honest about it: it decides WHICH gate, not whether the gate is sound. |
| (5) A branch names the ticket its work claims (PMAT-535) | Kubernetes, Terraform, Chef (and Gerrit, outside the surveyed set) | Almost nobody validates branch NAMES, because almost nobody derives anything from them. Gerrit binds a change to a `Change-Id` trailer in the commit message, not to a branch `[X]`; Kubernetes and Terraform both key their tooling off the PR object `[X]`, and Chef's Expeditor reads PR labels rather than the branch `[X]`. The comparison's finding is therefore about forjar's own design: three gates read the branch name as data (gate A resolves the receipt path, gate E the quorum slug, gate T the release window), so forjar owes a validation the surveyed systems do not, precisely because they do not read it. |
| (6) The SIGPIPE class is closed, and the rule covers the tree (PMAT-240) | Ansible, Salt, cdist | All three shell out to the managed host and all three therefore live with the same hazard, and none of them treats it as a checked property of their own scripts `[X]`. The comparison found no prior art to copy: the interesting part is not the fix but the census — a rule written over three files says nothing about the eighteen other sites in the tree, and the eighteen were found by looking rather than by reasoning. |
| (7) Gate T reads the cookbook's LOCK, not its requirement (PMAT-537) | Nix, Bazel, Terraform | Unanimous, and forjar was on the wrong side of it. `flake.lock` is what Nix builds; the flake input's requirement is not `[X]`. `MODULE.bazel.lock` plays the same role for bzlmod `[X]`. `.terraform.lock.hcl` pins provider versions and Terraform will refuse a run whose lock does not admit the configured constraint `[X]`. In all three the requirement admits and the lock decides; forjar's gate was reading the admission and calling it evidence. |
| (8) The 1.29.0 release record (PMAT-533, #533). | SLSA, in-toto, Sigstore | The field has moved to machine-checkable release records and forjar's is prose. SLSA provenance is a signed statement of what was built, by what, from what `[X]`. in-toto records each step of the supply chain against a layout `[X]`. Sigstore's transparency log makes the record append-only and publicly auditable `[X]`. forjar's release record is a markdown audit whose sentences are checked by review lanes — weaker in kind, and the five false sentences caught in this one are the argument both for the review and for eventually replacing it with something a machine can check. |
| (9) Book v1.29.0, and the first cookbook commit any release has named (PMAT-531) | Kubernetes, Terraform, Nix | Versioned documentation is universal: Kubernetes publishes docs per release branch `[X]`, the Terraform registry pins provider docs to the provider version `[X]`, and the nixpkgs manual is per channel `[X]`. What none of them does is name the external repository commit the release was qualified AGAINST — the closest analogue is a conformance suite pinned by version rather than by commit `[X]`. That is the part of this behaviour with no prior art in the survey, and it is the part that turned "dogfooded" from an adjective into a commit hash. |

## What is not reconciled here

The nine rows above are the nine behaviour bullets in the 1.30.0 CHANGELOG
section, which is the whole set `crux-reconcile.sh` requires. Everything else in
the window — the roadmap bookkeeping this cut had to repair to get gate B green,
the milestone and `release:` bindings for six issues filed during the window —
changes no behaviour of the shipped binary and is deliberately absent.
