# Quorum evidence — PMAT-235 — the claims as put to the lanes

# PMAT-235 — claims for the quorum lanes (sync the roadmap's status with what shipped)

Branch PMAT-235-sync-shipped-status, one commit (691c1faa) on main (d7028f5b).
Judge the diff `main...691c1faa`. kind: triage — a roadmap, a receipt, a log.

1. Before this commit, 16 tickets that a tagged release or a merged PR names
   read `planned` or `inprogress`: PMAT-137, 159 (v1.25.2), 162, 204
   (v1.26.0), 208 (v1.27.0), 215, 217, 219, 220, 221, 222, 223, 224
   (v1.28.0), and 227, 230, 231 (merged since v1.28.0). All 16 now read
   `completed`.
2. No ticket was marked completed that is not named by a `docs/roadmaps/
   releases.yaml` row or by a PR merged since v1.28.0 — the change is a
   backfill of what shipped, not a sweep.
3. Over the whole file the diff is exactly: 16 `status`, 16 `updated`, 14
   top-level `kind:` fields removed, 29 list items and 2 `notes` requoted.
   183 rows before, 183 after, none lost or added.
4. Field by field, there are ZERO semantic differences outside `status`,
   `updated` and `kind` — every title, description, acceptance criterion,
   note, label and timestamp on every other row is byte-equal in meaning.
5. All 14 rows that lost their top-level `kind:` field carry the matching
   `kind:*` label, and `kind-gate.sh` reads the label before the field, so
   nothing the gates use was lost.
6. Every `release:<tag>` label is intact: v1.25.0=1, v1.25.2=2, v1.26.0=9,
   v1.27.0=4, v1.28.0=10, v1.29.0=10.
7. `pmat work edit <id> -s completed` REFUSES a `planned` ticket
   (`Invalid transition: Planned → Completed`), and every ticket was moved
   `planned -> inprogress -> completed` with that command. The YAML was not
   edited by hand for any status.
8. `pmat work validate` passes and `scripts/dogfood/tagged.sh` is green on
   the branch.
9. PMAT-235 and PMAT-236 are new rows carrying `release:v1.29.0`, a `kind:`,
   acceptance criteria and notes; PMAT-236 records the missing gate arm —
   gate T checks the `release:<tag>` label in both directions and never
   looks at `status`, which is why this drifted for five releases with no
   gate going red.
10. The diff touches only `docs/roadmaps/roadmap.yaml` and `docs/audits/**`
    — the kind: triage rail — and no code.
