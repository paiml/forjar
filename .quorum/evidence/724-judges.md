# Quorum evidence — forjar#724 — adjudicated claims

Eight claims (C1–C8) were briefed to every lane and judge with the ticket and
the full diff against main b2918946. Five rounds: r1 two Claude lanes at
16b144b7, r2 three Claude judges at abd16b12, r3 three Claude judges at
28519ba6, r4 the Gemini lane at 984fb9b7, r5 the Gemini lane at c72ccb3a.
A claim is CONFIRMED when the last round to judge it confirmed it and the
author re-measured it; every refuted claim below was acted on or narrowed.

Probes ran on the author's Linux host: wlp0s20f3 holding a /24 LAN address and
10.42.0.20/24, lo, and docker0 (down) holding 172.17.0.1/16.

## CONFIRMED — 8 claims

1. [dispatch] C1 — an address held by a local interface is local, and `exec_script` runs it on the local transport, not over SSH to itself.
   - evidence: `src/transport/mod.rs:479` is the base `is_local_addr`, documented "check if the address matches any local interface" while comparing only loopback spellings and /etc/hostname. The branch ends it with `local_iface::addr_held_locally`, defined at `src/transport/local_iface.rs:22`. `tests/falsification_is_local_addr_interface.rs:56` dispatches a machine at the route-source address and asserts the script's `$PPID` equals the test's own pid, so the script ran as a child of this process and not of sshd. Confirmed by sonnet judges in r2 and r3 (measured bind+connect true for the LAN address, 10.42.0.20 and 172.17.0.1) and by Gemini in r5.

2. [dispatch] C2 — on Linux, 127.0.0.2 is local and dispatches locally with no network at all.
   - evidence: Linux routes all of 127.0.0.0/8 to lo, so the probe at `src/transport/local_iface.rs:49` binds and connects 127.0.0.2. `tests/falsification_is_local_addr_interface.rs:79` dispatches it and checks `$PPID` with no early return on a missing route, which is what makes it discriminate in a sandbox where the route-source tests return early. `src/transport/tests_local_iface.rs:48` pins the classification. Confirmed r2, r3 and r5.

3. [probe] C3 — TEST-NET, 0.0.0.0, ::, multicast, 255.255.255.255, directed broadcasts (and their ::ffff: forms) and non-IP strings stay remote.
   - evidence: `src/transport/local_iface.rs:46` excludes unspecified and multicast outright; an unheld address fails `bind` with EADDRNOTAVAIL; a directed broadcast, which Linux DOES let a UDP socket bind, fails `connect` with EACCES because SO_BROADCAST is not set (`src/transport/local_iface.rs:49`). Three judges measured 192.0.2.1, 198.51.100.1, 255.255.255.255, 192.168.1.255, 10.42.0.255, 172.17.255.255, 127.255.255.255 and ::ffff:192.168.1.255 all remote. Pinned by `tests/falsification_is_local_addr_interface.rs:104` and `src/transport/tests_local_iface.rs:59`.

4. [probe] C4 — with ip_nonlocal_bind set or unreadable the probe never claims local, and ::ffff:a.b.c.d is judged under the IPv4 sysctl.
   - evidence: `src/transport/local_iface.rs:28` folds IPv4-mapped IPv6 with `to_canonical` before `nonlocal_bind_path` at `src/transport/local_iface.rs:59` picks the sysctl, and `nonlocal_bind_read` at `src/transport/local_iface.rs:68` treats a failed read as enabled. `src/transport/tests_local_iface.rs:81` pins the unreadable decline, `src/transport/tests_local_iface.rs:90` the v4/v6 path choice, `src/transport/tests_local_iface.rs:69` that nonlocal_bind=true never claims local. Confirmed by Gemini in r5 after its own r4 refutation was fixed.

5. [callers] C5 — no caller of `is_local_addr` is made wrong for a genuinely held address, and what local dispatch bypasses is documented.
   - evidence: the callers read at base are `is_ssh_transport` (`src/transport/mod.rs:369`), `machine_is_local` (`src/transport/mod.rs:446`) and `controller_answers_for` (`src/transport/mod.rs:469`), which feed exec dispatch, drift, task probes, output verification and the lock baseline. Each asks one question — is this machine this host — and a held address is this host. The module doc at `src/transport/local_iface.rs:12` names what local dispatch skips: an OUTPUT DNAT rule, an ssh-config remap of the address, and the machine's user and ssh_key. Confirmed by three Claude judges in r2 and by Gemini in r4 and r5.

6. [tests] C6 — the edits to existing tests weaken no gate.
   - evidence: at base `src/transport/tests_dispatch.rs:8` and `src/transport/tests_dispatch.rs:232` asserted 192.168.1.100 and 192.168.1.1 not local, and `src/transport/tests_dispatch_b.rs:154` and `src/transport/tests_dispatch_b.rs:157` asserted the same of 192.168.1.1 and 10.0.0.1 — RFC 1918 addresses a runner can legitimately hold, which the fix would rightly call local. They now name RFC 5737 TEST-NET, never assigned. The base not-local assertion for 127.0.0.2 at `src/transport/tests_dispatch_b.rs:162` is kept under cfg(not linux) and inverted on Linux in `src/transport/tests_local_iface.rs:48`.

7. [platform] C7 — no new dependency; off Linux the probe declines; src/transport/mod.rs stays within the 500-line file-health limit.
   - evidence: the probe uses only std::net and std::fs and returns false whenever `!cfg!(target_os = "linux")` (`src/transport/local_iface.rs:46`), pinned off Linux by `src/transport/tests_local_iface.rs:129`. Cargo.toml is not in the diff. mod.rs measured at 497 lines on the branch; the probe was moved into local_iface.rs because the first cut put mod.rs at 528 and file-health refused it.

8. [falsification] C8 — each fix has a test that goes red without it.
   - evidence: revert probes, each applied, verified applied by grep count, run, and restored: interface arm removed from `is_local_addr` → 4 of 6 in `tests/falsification_is_local_addr_interface.rs:40` and siblings fail; `connect` removed (the r1 bind-only design) → `tests/falsification_is_local_addr_interface.rs:104` fails; `to_canonical` removed → `src/transport/tests_local_iface.rs:115` fails; link-local assertion inverted → `src/transport/tests_local_iface.rs:141` fails on a real fe80:: address; `map_or(false)` → `src/transport/tests_local_iface.rs:81` fails; v4/v6 path branch inverted → `src/transport/tests_local_iface.rs:90` fails. Gemini refuted this claim in r4 and confirmed it in r5.

## REFUTED — 9 claims

1. [r1] The bind-only probe keeps directed broadcasts remote.
   - corrected: false — Linux lets a UDP socket bind a subnet's directed broadcast. Both r1 lanes measured 192.168.1.255, 10.42.0.255, 172.17.255.255 and 127.255.255.255 classified local at 16b144b7. Fixed in abd16b12 by requiring `connect` to succeed as well (`src/transport/local_iface.rs:49`), which refuses a broadcast destination without SO_BROADCAST.

2. [r1] The probe judges ::ffff:a.b.c.d under the right sysctl.
   - corrected: false at 16b144b7 — an IPv4-mapped literal read the IPv6 ip_nonlocal_bind while the kernel binds it through the IPv4 stack. Fixed in abd16b12 by `probe_target` folding with `to_canonical` (`src/transport/local_iface.rs:28`), pinned by `src/transport/tests_local_iface.rs:115`.

3. [r1] The route-source tests always measure something.
   - corrected: false — with no IPv4 route they return early and assert nothing, so on a network-less runner the fix had no discriminating test. abd16b12 added the 127.0.0.2 dispatch test at `tests/falsification_is_local_addr_interface.rs:79`, which needs no network on Linux.

4. [r2] The probe is correct on every platform forjar runs on.
   - corrected: unbacked — the broadcast refusal rests on Linux `connect` semantics and nobody measured macOS. 28519ba6 gates the probe to Linux (`src/transport/local_iface.rs:46`), so elsewhere it declines to SSH, the behaviour before it existed.

5. [r2] An unreadable sysctl declines the probe, as the module doc says.
   - corrected: false at abd16b12 — an unreadable /proc/sys/net read counted as disabled, so the probe answered. Found by the author while fixing r2; 28519ba6 makes a failed read count as enabled (`src/transport/local_iface.rs:69`).

6. [r2] Every address a local interface holds is recognised.
   - corrected: false for IPv6 link-local — fe80:: cannot be bound without a scope id and a scoped literal does not parse as an IpAddr. Narrowed and documented as a false negative, never a false positive, and pinned by `src/transport/tests_local_iface.rs:141`, which reads a real scope-20 row from /proc/net/if_inet6.

7. [r2] Any 127/8 address is local on Linux.
   - corrected: false for 127.255.255.255, lo's broadcast. This was the brief's wording, not the code's: the code and `tests/falsification_is_local_addr_interface.rs:104` already assert 127.255.255.255 remote. The comment that repeated the wording was corrected in 984fb9b7.

8. [r4] The test suite passes off Linux with the fix in place.
   - corrected: false at 984fb9b7 — Gemini found `test_local_iface_nonlocal_bind_never_claims_local` asserting `ip_held_locally(127.0.0.1, false)` on every platform while the probe declines off Linux, so it would panic on macOS and contradict the off-Linux test. Every Claude seat ran on Linux and missed it. c72ccb3a gates that assertion to Linux (`src/transport/tests_local_iface.rs:69`).

9. [r4] The unreadable-sysctl decline and the v4/v6 sysctl choice are pinned by tests.
   - corrected: false at 984fb9b7 — Gemini showed that mutating `map_or(true, …)` to `map_or(false, …)`, or swapping the two sysctl paths, left every test green. c72ccb3a splits the read into `nonlocal_bind_path` and `nonlocal_bind_read` and pins each (`src/transport/tests_local_iface.rs:81`, `src/transport/tests_local_iface.rs:90`); both mutations were measured red.
