# CRUX Audit — forjar 1.31.0

## Method

**Who wrote this, and from what.** Like the 1.30.0 audit, this one was written by
the release orchestrator (Claude Opus 5) directly, from documentation memory,
with no network access and no live invocation of any reference system. The
1.29.0 audit was surveyed by an `agy` quorum lane; this one was not, and that
difference is recorded rather than implied.

Every claim about a third-party system below is marked `[X]`: asserted from
documentation memory, not measured against a running instance. Every claim about
forjar is measured in this release's window, and the measurements live in the
receipts each row's ticket names.

**What this audit can and cannot show.** It shows that each behaviour shipping in
1.31.0 has been held against at least three systems that solve the same problem,
and what that comparison found. It cannot show that the comparison is correct in
detail — `scripts/dogfood/crux-reconcile.sh` says the same about itself.

**What 1.31.0 is about.** All three behaviours are one question asked three ways:
what is a caller entitled to conclude from a run that printed something and
exited 0? Two of them (PMAT-562, PMAT-564) are the two signatures of
paiml/infra#605, found by an autonomous infra run against the live fleet; the
third (PMAT-560) is the defect that run was looking for when it found them.

## The comparison

| Behaviour | Systems surveyed | What the comparison found |
|---|---|---|
| (1) forjar drift declines — exit 2, the count named — when it inspected none of the resources it was asked about, and never grades a resource from a manifest it was not given (PMAT-564, #564) | Bazel, Ansible, Terraform, Kubernetes | The field splits, and forjar was on the wrong side of the split for the wrong reason. Bazel treats a target pattern that matches nothing as an error rather than an empty success `[X]`, and modern Ansible refuses a `--limit` that matches no host instead of reporting a play that did nothing `[X]` — both encode "you asked about something I could not find" as its own outcome. Kubernetes goes the other way: `kubectl get` over an empty selection prints `No resources found` and exits 0 `[X]`, which is the shape forjar had. Terraform is the closest analogue and the sharpest contrast: its state is authoritative, and a `-target` naming nothing in the configuration is refused rather than silently planned around `[X]`. forjar's defect was worse than the Kubernetes shape, because the empty selection was not empty: the run graded two resources from a manifest it had never been given, so the zero it reported and the two it printed came from different questions. The decline (exit 2) and the `in the lock, not in the config` skip are the two halves of that repair. |
| (2) A service is converged only while the loaded unit executes the declared program — exec_start / exec_sha256 (PMAT-560, #560) | systemd, Ansible, Puppet, Chef, Podman | Unanimous, and the gap was forjar's alone. systemd itself distinguishes the unit file on disk from the unit it has loaded, which is why `systemctl show -p ExecStart` reports the LOADED program and why a drop-in or a hand edit plus `daemon-reload` can make the file and the behaviour disagree `[X]` — the data forjar needed was always there and it never asked for it. The configuration managers all stop where forjar stopped: Ansible's `service`/`systemd` modules converge on `state` and `enabled` `[X]`, Puppet's `service` resource on `ensure` and `enable` `[X]`, and Chef's `service` resource on the same pair `[X]`; each manages the unit FILE as a separate file resource, so each can write a file and then report a service converged while systemd runs something else. The container world is where the answer already exists: Podman and Docker identify what is running by image DIGEST, not by the name that was asked for `[X]`. `exec_sha256` is that idea applied to the unit's program — and, as the ticket insists, hashed at the LIVE path, since hashing the declared one would confirm the file forjar wrote and prove nothing about the unit. |
| (3) forjar drift exits 1 on any DRIFTED line, on every run, with no flag (PMAT-562, #562) | Terraform, Puppet, Kubernetes, Ansible | The field agrees that a difference belongs in the exit code, and disagrees only about whether you must ask for it. `terraform plan -detailed-exitcode` exits 2 when there are changes to apply `[X]`; Puppet's `--detailed-exitcodes` exits 2 when it changed something and 4 on failure `[X]`; `kubectl diff` exits 1 when live and desired differ `[X]`. Ansible is the outlier that shows the cost: `--check` reports changed hosts in the recap but exits 0 unless a task fails, so the recap has to be parsed `[X]`. forjar had Terraform's and Puppet's design — the verdict was real, behind a flag — and a flag-gated verdict is an un-gated bug: every caller that did not know the flag got a zero. The comparison's finding is that the two systems with an opt-in flag both default to "changes are not an error" for a plan, whereas forjar's `drift` is a CHECK, whose only reason to run is the question the exit code now answers. |

## What is not reconciled here

The three rows above are the three behaviour bullets in the 1.31.0 CHANGELOG
section, which is the whole set `crux-reconcile.sh` requires. Everything else in
the window — the roadmap bookkeeping this cut had to repair to keep gate B inside
its ceilings, the milestone and `release:` bindings for the issues filed during
the window — changes no behaviour of the shipped binary and is deliberately
absent.

PMAT-565 (the per-machine lock names the binary that wrote it) is NOT in this
release: #570 was still open when the cut was made, and a behaviour that has not
merged owes no row. It reconciles in the release that ships it.
