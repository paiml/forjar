# PMAT-520 — the crux audit, and who wrote which row

`docs/audits/crux-1.29.0.md` carries twelve rows against twelve behaviour
bullets, 28 surveyed systems and **36 `[X]` claims** — every third-party claim
asserted from documentation memory, none measured against a running instance.

**Thirty of the claims are the lane's.** One agy lane surveyed the ten bullets
that existed when it ran, reading only the release's CHANGELOG section, the row
keys `crux-reconcile.sh` asked for, and a read-only copy of the 1.28.0 audit as
a worked example. It contacted no network and invoked no reference system.

**Six are the orchestrator's**, in rows 11 and 12, written after the lane ran
because neither behaviour existed yet: pmat 3.40 turned gate B red the same
day, and the fork storm happened while fixing that. The audit says so in its
own limit section rather than presenting twelve rows as one lane's work.

## The four rows that say forjar is worse

A survey that always concludes we are right is not a survey. Four of twelve
conclude the opposite, and they are the four worth reading first:

| row | where forjar is worse |
|---|---|
| 5 | clearing shared `/tmp` by hand, where systemd's `PrivateTmp`, a container filesystem and a mount namespace are isolation |
| 8 | a release gate whose telemetry depends on shell pipelines is brittle by construction; eighteen SIGPIPE sites remain open as PMAT-240 |
| 11 | a dated ceiling records debt without naming the release at which it stops being tolerated, which is exactly what Bazel's `--incompatible_*` flags do |
| 12 | Bazel, Nix and systemd refuse a cycle at DECLARATION time from the graph itself; forjar refuses it at run time, and the tool's own side has no guard at all |

Two review lanes independently identified the same four and neither found a row
claiming a win it had not earned.

## One row was wrong and is corrected

Row 1 said gate T "refuses the tag if the cookbook cannot use it". It refuses
the ledger ROW — the tag already exists when the gate runs, which is why a
release is three PRs and why gate T is red on main between the tag and the
booking. A lane caught it.
