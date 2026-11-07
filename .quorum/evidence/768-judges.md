# Judge scores — #768 / PR 771 — final verdict per claim

Three independent judges (agy gemini-4-argon-high, agy gemini-3.8-flash-high, claude-haiku-4-5), each given the checkout,
the diff, the claims and the refuters' answers, each told to decide from the code and to treat the refuters as evidence only.

| claim | judge 1 | judge 2 | judge 3 | final |
|---|---|---|---|---|
| C1 table matches the 19 relaxed jobs | confirmed | confirmed | confirmed | confirmed |
| C2 unlisted unpinned job fails | confirmed | confirmed | confirmed | confirmed |
| C3 two-sided staleness | confirmed | confirmed | confirmed | confirmed |
| C4 mixed matrix caught | refuted | refuted | refuted | refuted (hole in the guard) |
| C5 staleness sees leaving the fleet | refuted | refuted | refuted | refuted (hole in the guard) |
| C6 release jobs keep X64 | confirmed | confirmed | confirmed | confirmed |
| C7 20 sites keep self-hosted + clean-room | confirmed | confirmed | confirmed | confirmed |
| C8 delta is workflows plus the fleet test | confirmed | confirmed | refuted | confirmed 2 to 1 (judge 3 scored the gate's wider 49-file diff) |
| C9 PLANTED: file-level admission | refuted | refuted | refuted | refuted, control killed 3 of 3 |
| C10 diff guarantees gx10 pickup | refuted | refuted | refuted | refuted |
| C11 epic-base test red, PR test green | confirmed | confirmed | confirmed | confirmed, and measured |
| C12 test reads workflows at run time | confirmed | confirmed | confirmed | confirmed |

Every judge's PLANT-CHECK line named C9 among the claims the code plainly contradicts.
