# Claims — forjar#724: is_local_addr recognises addresses held by a local interface

Briefed to every lane and judge with the ticket and the full diff against
main b2918946. Rounds judged heads 16b144b7 (r1), abd16b12 (r2), 28519ba6 (r3),
984fb9b7 (r4, Gemini) and c72ccb3a (r5, Gemini). The adjudication is in
724-judges.md.

- C1: an address held by a local interface is local, and `exec_script` runs it on the local transport.
- C2: on Linux, 127.0.0.2 is local and dispatches locally with no network.
- C3: TEST-NET, 0.0.0.0, ::, multicast, 255.255.255.255, directed broadcasts (and their ::ffff: forms) and non-IP strings stay remote.
- C4: with `ip_nonlocal_bind` set, or unreadable, the probe never claims local; ::ffff:a.b.c.d is judged under the IPv4 sysctl.
- C5: no caller of `is_local_addr` is made wrong for a genuinely held address, and what local dispatch bypasses is documented.
- C6: the edits to existing tests weaken no gate.
- C7: no new dependency; off Linux the probe declines; src/transport/mod.rs stays within 500 lines.
- C8: each fix has a test that goes red without it.
