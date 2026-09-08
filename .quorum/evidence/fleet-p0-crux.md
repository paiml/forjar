# Quorum evidence — 1.27.0 fleet P0 — CRUX

Full audit in `docs/audits/crux-1.27.0.md`; conversation id recorded in the receipt, verdict PASS. Systems surveyed: Terraform, Ansible, Puppet, Chef, Pulumi. Every third-party claim is `[X]`, asserted from documentation memory.

The sharpest row is the second. Three of the four systems surveyed for it keep NO failure verdict at all — Puppet, Chef and Ansible re-evaluate live state on every run, so forjar's latch is not a defect they could have. Terraform is the one system that persists a verdict outliving its cause, and its documented way back is a forced destroy-and-recreate rather than a re-read. forjar had Terraform's persistence and no release of any kind. The fix takes a third option neither offers, which is coherent only because a forjar `completion_check` is a pure predicate the tool can re-run on demand.

Gate H: PASS, 3 of 3 behaviour paragraphs under `[1.27.0]` reconciled, each naming at least three of the 28 surveyed systems.
