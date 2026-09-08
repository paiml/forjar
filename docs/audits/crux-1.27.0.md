# CRUX Audit — forjar 1.27.0

## Method

One `agy` plan lane (conversation an agy plan lane recorded in the session transcript) surveyed
four world-class configuration-management and infrastructure-as-code systems
against each of the three behaviour changes in this release, from documentation
memory only — no network access and no live invocation of any reference system.

Every claim about a third-party system is marked `[X]`: asserted from
documentation memory, not measured against a running instance. Every claim about
forjar is marked `[V]` and is measured in this session against the real binary;
the measurements are in `docs/audits/impl-PMAT-212-receipt.md`,
`impl-PMAT-213-receipt.md` and `impl-PMAT-214-receipt.md`.

This release is unusual for forjar in that all three behaviours are DEFECTS
found by an operator running the tool against a real fleet, not designed
features. The comparison is therefore asking a sharper question than usual: not
"is forjar's design competitive" but "did the surveyed systems ever have this
defect available to them, and why not".

## The comparison

| Behaviour | Systems surveyed | What the comparison found |
|---|---|---|
| (1) `drift -f <config> answered about machines` it never declared, and judged another computer's file by the local one; now scoped, with `--all-stacks` to opt into the aggregate and the machine named on every row | Terraform, Ansible, Puppet, Pulumi | Every one of the four scopes a state query to the addressed target, three of them BY CONSTRUCTION rather than by flag [X]. Terraform's plan is bounded by the workspace's own state file, Puppet's agent requests a catalog for itself and can evaluate no other, Pulumi's stacks are isolated, and Ansible bounds a run by inventory and `--limit` with every result line carrying its host [X]. None of the four has a shape in which a command addressed to one host can report another host's divergence, which is why none of them needed a `--all-stacks` opt-out: the wide question is simply not expressible by accident. forjar's defect was possible because one shared state directory holds every stack and the walk read the directory rather than the config [V]. accept(scope the walk to the config; keep the aggregate as an explicit request, because paiml/infra's nightly tripwire genuinely asks the wide question) |
| (2) `A task whose command exits non-zero` latched, and `--refresh` could not free it; it now re-checks a failed entry and writes down what it measured | Terraform, Puppet, Chef, Ansible | Three of the four keep NO failure verdict at all: Puppet, Chef and Ansible start each run fresh and re-evaluate live state, so the latch is unreachable — there is no way back because there is nothing to come back from [X]. Terraform is the one system that persists a verdict outliving its cause, as a tainted resource, and its documented way back is an explicit operator action (`untaint`, now expressed as `-replace`) that forces destroy-and-recreate rather than a re-read [X]. forjar sits between the two and had the worst of both: it persisted the verdict like Terraform and offered no explicit release like Terraform's [V]. The fix takes the third option neither surveyed design offers — re-read live state and record what it found — which is coherent only because forjar's `completion_check` is a pure predicate the tool can re-run on demand [V]. accept(a re-check is a better release than a forced rebuild when the check is cheap and total; the persistence is what makes it a way back rather than a flag to pass for ever) |
| (3) `The cargo provider's check could not` see what its own install had written, because only the install repaired PATH; all three sites now share one prelude | Ansible, Puppet, Chef, Terraform | All four make the check and the action share one environment BY CONSTRUCTION or by a single named declaration [X]. Puppet's `Exec` has one `path` attribute that `onlyif`, `unless` and `command` all resolve through; Ansible's `environment:` applies to a task or block and therefore to check mode and execution alike; Chef's `execute` takes one `environment` hash; a Terraform provider is one binary whose plan and apply run in the same process environment [X]. The universal failure mode when they diverge is exactly what forjar suffered: lost idempotency, the action repeating for ever because the check cannot see its result [X]. forjar had three emitters and repaired PATH in one [V]. reject(the asymmetry, not the mechanism — the fix is one shared prelude all three sites emit, which is Puppet's `path` attribute expressed in generated shell) |

## Gate H keys

`scripts/dogfood/crux-reconcile.sh` reads `CHANGELOG.md` for paragraphs that
open with a bold span at column 0 after a blank line, takes the first six words
of that span (backticks removed) as the key, and requires one row above
containing the key verbatim and naming at least three surveyed systems.

| Key, verbatim as the gate derives it | Systems the mapped row names |
|---|---|
| drift -f <config> answered about machines | Terraform, Ansible, Puppet, Pulumi |
| A task whose command exits non-zero | Terraform, Puppet, Chef, Ansible |
| The cargo provider's check could not | Ansible, Puppet, Chef, Terraform |

## The limit of this audit

Twelve system claims, all `[X]`. The lane read no vendor documentation during
the run and contacted no running instance, so each is documentation memory and
could be out of date — Terraform's taint surface in particular has moved, and
the row above says so rather than picking one spelling and asserting it. What
the audit is for is the SHAPE of the comparison, and the shape is not delicate:
that three of four systems make cross-target reporting inexpressible, that three
of four keep no failure verdict at all, and that four of four bind check and
action to one environment, would each survive a correction to any single row.

CRUX-1.27.0-END
