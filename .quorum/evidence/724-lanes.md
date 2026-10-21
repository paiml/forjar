# Lanes — forjar#724

Author model: claude-opus-5-5. No lane or judge ran on the author's model.
Every lane was read-only on the repo; probes ran in a scratch directory.

## Round 1 — head 16b144b7 — NOT AGREED (2 FAIL)

- claude-sonnet (Claude Code subagent): FAIL. Measured that Linux lets a UDP
  socket BIND a subnet's directed broadcast: 192.168.1.255, 10.42.0.255,
  172.17.255.255 and 127.255.255.255 were classified local. Also raised the
  IPv6 sysctl read for ::ffff: forms, and that the route-source tests return
  early with no route.
- claude-haiku (Claude Code subagent): FAIL. Measured the same broadcast hole
  independently; raised the untested sysctl parse and the DNAT / ssh-config
  remap case.
- agy gemini-3.1-pro-high: did not run — `agy` on the author's host returned
  "authentication failed or timed out". See 724-agy.md.

## Round 2 — head abd16b12 — 3 judges, 2 PASS 1 FAIL

- claude-sonnet-5 judge 1: PASS (C1–C8 confirmed; measured).
- claude-sonnet-5 judge 3: PASS (callers; probe cost ~2.5 µs/call, measured).
- claude-haiku-4-5 judge 2: FAIL — C7 (broadcast refusal unmeasured off
  Linux), C6 (platform-neutral 127.0.0.2 assertion dropped), C1 (IPv6
  link-local is a false negative, measured), C5 (ssh-config remap undocumented).

## Round 3 — head 28519ba6 — 3 judges, 3 PASS

- claude-sonnet-5 judge A: PASS (F1–F4 confirmed; link-local measured).
- claude-haiku-4-5 judge B: PASS; NEW-1 comment wording and NEW-2 sandbox
  skip, both fixed in 984fb9b7.
- claude-sonnet-5 judge C: PASS (measured 10.42.0.20 and 127.0.0.2 still
  local under the new sysctl read).

## Round 4 — head 984fb9b7 — Gemini lane: FAIL (C1–C7 confirmed, C8 refuted)

- Gemini 4 Argon (High) via agy, read-only, file-reading tools only.
  Refuted C8 with an off-Linux test contradiction (blocker) and an untested
  sysctl fallback (minor). Both verified real; fixed in c72ccb3a. See 724-agy.md.

## Round 5 — head c72ccb3a — Gemini lane: PASS (C1–C8 confirmed, no new findings)

Rounds 1–3 were a same-family (Claude) quorum and are recorded as one. The
only cross-family seat, Gemini, found the one defect every Claude seat missed —
because every Claude seat ran on Linux — which is the case for having it.
