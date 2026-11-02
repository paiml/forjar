# FJ-METAL-001 — forjar Bare Metal: Netboot, USB Parity, Failover, Self-Repairing Boot OS

## ELI5

**Goal:** forjar owns a machine from bare metal up: boot, install, fail over, repair. Today forjar starts only after an OS exists, so the most fragile step lives outside it.

**Key idea:** build **one** signed boot image. Ship the same bytes three ways: over the network, on a USB stick, and in a rescue partition on the local disk. Parity is a hash comparison, not a promise.

**What the image is:** a small RAM-only Linux whose job is to run the `forjar` binary. forjar diagnoses the machine, installs it, applies its declared state, restores data and verifies, then reboots into the real OS.

**What stops it:** any transport boots a different hash, or a repair step leaves no receipt.

| Question | Answer |
| --- | --- |
| Netboot and USB the same? | Yes, byte-identical image; a gate compares hashes |
| Works with the network down? | Yes, from USB or the local rescue partition |
| Works with the disk dead? | Yes, from network or USB |
| Self-repairing? | Yes: `forjar repair` is an idempotent state machine with a receipt per step |
| New dependencies? | No Python. The image is built by forjar in Rust; kernel from the distro archive |
| Testable without hardware? | Yes: QEMU with UEFI firmware boots all three transports in CI |
| Wipes disks by itself? | Never. Destructive steps need a serial-pinned target and, for restore, an approval |

## Purpose and scope

This spec extends forjar, an open-source Rust infrastructure tool, to the machines it manages. Third-party software named here is used under its own license.

Owning repo: `paiml/forjar`. This spec is generic: no host names, addresses, capacities or other site detail. Site-specific detail belongs in each consumer's own configuration.

## §0 Five-whys and assumptions

The gap is that forjar begins after the OS exists, so every machine's first and last mile is manual.

1. **Why does a dead OS disk cost hours?** The rebuild started from a hand-made USB stick and a person.
2. **Why no automated path?** forjar has no resource for boot media, network boot or disk layout.
3. **Why does that matter beyond speed?** Every step outside forjar is undeclared, and undeclared provisioning drifts silently: checks can stay green while unmanaged units run.
4. **Why do netboot and USB drift apart in other tools?** They are built by different pipelines from different inputs.
5. **Why is repair manual?** No program owns the state machine from "disk suspect" to "host green".

**Terminal cause (mechanism):** no forjar resource types below the OS, and no single boot artifact. Both are fixable by construction.

Assumptions:

- `[U]` What forjar ships today for machines, disks or boot. M0 reads HEAD before any row.
- `[A]` Hosts boot UEFI on x86\_64 and aarch64. Legacy BIOS is out of scope.
- `[A]` A static, musl-linked forjar binary runs in a minimal initramfs.
- `[A]` Kernel and firmware blobs come from the distro archive, pinned by hash, not built here.

## §1 Ground truth and reference systems

We copy three proven ideas and drop the parts that break sovereignty. Marks: `[X]` third-party claim · `[A]` assumption · `[U]` unmeasured · `[V]` verified spec.

| System | Idea we take | What we drop | Mark |
| --- | --- | --- | --- |
| [Tinkerbell](https://tinkerbell.org/) | A DHCP/iPXE service that answers only known MACs, so it coexists with an existing DHCP server; an in-memory OS that runs a declared workflow | Kubernetes control plane, gRPC workflow server, container-image actions | `[X]` |
| [Talos image factory](https://github.com/siderolabs/image-factory) | One build emits ISO, PXE kernel/initrd and a Secure Boot UKI from the same inputs | Kubernetes-only OS, no shell, hosted factory | `[X]` |
| [systemd boot assessment](https://systemd.io/AUTOMATIC_BOOT_ASSESSMENT) | Tries-left / tries-done counters in the boot entry filename; a service blesses the entry after a good boot, else the loader falls back | Nothing: use as-is on the installed OS | `[X]` |
| [Unified Kernel Image](https://www.man7.org/linux/man-pages/man1/ukify.1.html) | Kernel + initrd + command line in one signed PE file, so one signature covers the whole boot payload | `ukify` and `mkosi` (both Python): forjar assembles the PE itself | `[X]` |
| pv ([provable-contracts](https://crates.io/crates/provable-contracts)) | Contracts with SHACL shapes over declared entities; Unknown never arms | — | `[V]` spec |

Forjar facts to verify at HEAD in M0: existing resource types, the drift checker's reach, whether `verify-systemd-units` checks exec hashes, and the release binary's link mode `[U]`.

## §2 Architecture: one image, three transports

Parity is by construction: there is one build, and every transport carries its output unchanged.

Each transport covers a different failure. Network covers a dead disk. USB covers a dead network or firmware without PXE. The rescue partition covers both being down while the disk still reads.

```
inputs (pinned) -> forjar image build -> UKI (hash H)
   -> network | USB stick | rescue partition  (each must hash to H)
   -> repair OS -> forjar repair
```

## §3 New resource types and verbs

Seven resource types take forjar below the OS. Each has a pv shape (§9) and a drift check.

| Resource | Declares | Converges by | Drift check |
| --- | --- | --- | --- |
| `machine` | Arch, firmware caps (UEFI HTTP, PXE, Secure Boot), NIC MACs, disk serials, which disks are data and which is the spare | Inventory only | Live MACs and serials equal the declaration |
| `boot_image` | Kernel + initrd inputs (archive pins by hash), forjar version, embedded config bundle, signing key ref | `forjar image build`: reproducible UKI per arch | Built hash equals pinned hash |
| `netboot_server` | proxyDHCP for listed MACs only, TFTP, HTTP, per-MAC boot decision, heartbeat socket, package cache | Units + rendered per-MAC configs | Units, configs and served image hash |
| `boot_media` | A USB stick by serial: ESP holding the same UKI, label naming the host | `forjar media write` then read-back verify | Stick image hash equals `boot_image` hash |
| `rescue_partition` | A small ESP-adjacent partition on the local disk holding the same UKI | Written on install and on each image bump | Partition hash equals `boot_image` hash |
| `disk_layout` | Partitions, filesystems, RAID, target selected **by serial**; protected serials listed | Applied only from the repair OS onto the declared target | Live layout equals declaration |
| `boot_policy` | Boot order, local boot counting (tries), heartbeat deadline, watchdog timeout | Firmware entries via `efibootmgr`-equivalent, loader config | Live order and counters as declared |

Verbs:

- `forjar image build|verify` — build or check the UKI; prints its hash.
- `forjar media write --serial <s>` — write and read back; refuses a stick not declared.
- `forjar parity check` — compare the hash of network, USB and rescue copies for a machine; any mismatch exits 1.
- `forjar repair` — the state machine in §4, run inside the repair OS.
- `forjar boot status|reset <machine>` — show or clear the server-side counter and the local tries counters.

Exit codes follow pv: `0` pass, `1` fail, `2` unknown (never mapped to `0`), `3` error.

## §4 The repair state machine

`forjar repair` is idempotent: re-running it from any state reaches the same end, because each state checks its receipt before acting.

Only step 3 writes a disk and only step 5 needs a person. Apply runs in two stages: step 4 is the base stage and needs no secret; step 7 runs the full apply after restore, so anything needing a credential (runner registration, sshd on restored host keys, backup timers) converges only once secrets are back. A healthy disk exits at step 2 with nothing written.

State machine:

```
1 Diagnose -> 2 Decide --healthy--> boot reset, reboot to own disk
                       --dead/absent--> 3 Install (declared spare only; none -> report, stop)
-> 4 Apply (base stage, no secrets) -> 5 Approve (operator) -> 6 Restore (secrets, then state)
-> 7 Full apply + verify + recover gate -> 8 Bless and reboot
Every arrow writes a receipt; power loss resumes from the last receipt.
```

## §5 Hard rules

Each rule has a falsifier in §9.

1. **One image, three transports.** Network, USB and rescue partition carry byte-identical UKIs. `forjar parity check` compares hashes; any mismatch is RED. No transport has its own build.
2. **Reproducible.** The same inputs on two different build hosts give the same hash. Inputs are pinned by hash: kernel, firmware, forjar binary, config bundle.
3. **Self-contained.** The image carries a static forjar binary, the backup client, disk tools and a git bundle of the config repo at a pinned commit. It needs no internet and no credentials to reach the point where it asks for approval.
4. **Rebuild, never clone.** Repair installs from declarations. No block image of a live disk is ever restored.
5. **Destructive steps are serial-pinned.** `disk_layout` writes only to the serial declared as target and refuses any serial on the protected list. No serial, no write.
6. **Two failover layers.** Local: boot counting on the installed OS falls back to the previous entry, then to the rescue partition. Network: the boot server sends a host with no heartbeat after N boots to the repair OS. Either works when the other is down.
7. **The repair OS never decides alone to destroy.** It diagnoses and reports by itself. Install needs a dead or absent disk *and* the declared spare. State restore needs an operator approval.
8. **Every step leaves a receipt.** `forjar repair` writes a JSON receipt per state transition. No receipt, no next state.
9. **Unknown never arms.** A probe that cannot see its subject (no disks found, no MAC match) returns `2`, never `0`.
10. **Sovereign build.** No Python anywhere (`ukify` and `mkosi` excluded). PE assembly, initramfs cpio and parity hashing are forjar Rust code. Signing tool per D2.
11. **Signed.** Secure Boot stays on where firmware allows. Unsigned images are refused unless a host declares Secure Boot off by operator ruling.
12. **Testable without hardware.** Every transport and every repair path runs in QEMU with UEFI firmware in CI before any hardware drill.

**Rule 13 — SSH is the only access.** The repair OS listens on port 22 and nothing else. Login is private-key only, from known LAN hosts only: each authorized public key is declared in forjar and pinned to its host's LAN address with an `authorized_keys from=` restriction, so a key used from any other machine is refused. No passwords, no console login prompt (the console prints status, receipts and the host-key fingerprint only), no web or API shell. A fresh host key is generated each boot and its fingerprint is posted to the boot server with the heartbeat so clients can verify it. Keys and pins are public data, so the image keeps hash parity. SSH is for diagnosis and watching receipts; install and key release still go only through `forjar repair` and the operator approval.

Falsifiers: a listening-port scan shows exactly `22`; password login refused; a declared key used from an undeclared address refused; an undeclared key refused; the console offers no login; a fingerprint differing from the one posted to the boot server is flagged by the client. Anti-vacuity: one declared key from its declared host logs in during the QEMU job.

## §6 EV-ordered rows

One `pmat work add` ticket and one PR per row in `paiml/forjar`. Order: prove the image and the harness first, because every later row is tested on them.

| Row | Work | Done when |
| --- | --- | --- |
| M0 | **Read HEAD.** Inventory forjar resource types, drift reach, binary link mode. Add bare-metal rows to a comparison table (Tinkerbell, Talos). | Written gap list; every `[U]` in §1 resolved |
| M1 | **`machine` resource + pv shape.** Inventory only, no side effects. | Shape validates on fixtures; planted bad MAC or serial exits 1 |
| M2 | **`boot_image` builder.** Rust PE assembly, initramfs cpio, static forjar inside, x86\_64 and aarch64. | Same hash on two build hosts, both arches |
| M3 | **QEMU harness.** UEFI firmware; boot the image by network (QEMU built-in TFTP/HTTP), by USB-storage device, and from a rescue partition on a virtual disk. Runs in CI. | 3 transports × 2 arches boot to the forjar prompt; `parity check` green |
| M4 | **`netboot_server` resource.** proxyDHCP for listed MACs, TFTP, HTTP, per-MAC decision, heartbeat socket, package cache. | QEMU guest boots disk when healthy, repair OS after N missed heartbeats |
| M5 | **`boot_media` + `rescue_partition`.** Write, read back, verify by hash. | Planted one-byte change on a stick → parity RED |
| M6 | **`disk_layout`.** Serial-pinned, protected-serial list. | Planted protected serial → refused before any write |
| M7 | **`forjar repair`.** The §4 state machine with receipts and the approval gate. | QEMU: dead disk → green host with one approval and zero other input |
| M8 | **`boot_policy`.** Firmware boot order, local boot counting, watchdog, server heartbeat deadline. | QEMU: broken kernel entry falls back on its own; healthy host never trapped |
| M9 | **Hardware drills** on real machines by a downstream consumer. | Two consecutive drills per transport inside §7 targets |

Budget: K̂ = 10 PRs, K = 13, andon at 12 `[A]`; M2 and M7 may split. Re-estimate after M0.

## §7 Targets

Zeros and ones are hard. Times are `[A]` until two runs measure a baseline, then ratchet down only.

| Metric | Target | Measured by |
| --- | --- | --- |
| Parity mismatches across transports | 0 | `forjar parity check` per machine, nightly |
| Reproducibility | Identical hash on 2 build hosts | `forjar image build` on both, compare |
| QEMU matrix | 6 of 6 green (3 transports × 2 arches) | CI job receipt |
| Writes to a non-target serial | 0 | `disk_layout` refusal log + falsifier |
| Repair steps without a receipt | 0 | Receipt count equals transition count |
| Healthy hosts trapped in repair OS | 0 | Boot counter log vs heartbeat log |
| Repair OS: power-on to forjar prompt | ≤ 2 min `[A]` | Receipt timestamps |
| Dead disk to green host (QEMU) | ≤ 20 min `[A]` | M7 receipt |
| Image size | Measured, then ratcheted `[U]` | Build output |

## §8 Stops and decisions

**Stops (hard):** a Secure Boot key or firmware setting change; any write to a real disk outside QEMU before M6 is green; publishing a forjar release; any gate or ruleset change.

| # | Question | Recommended | Why |
| --- | --- | --- | --- |
| D1 | Loader: systemd-boot + UKI, or shim + GRUB | systemd-boot + UKI on installed hosts; shim + GRUB only for PXE firmware that cannot load a UKI | UKI is one signed file and boot counting is native in systemd-boot |
| D2 | Signing | Own key enrolled per host (MOK), signing via the distro's `sbsign` package | No third-party binary; a Rust signer can replace it later, same as restic → copia |
| D3 | PE assembly | In-house Rust module in forjar | Avoids Python `ukify`; small, testable, reproducible |
| D4 | Initramfs userland | Rust init that execs forjar, plus pinned archive tools (`nvme`, `mkfs`, `mdadm`) | Smallest attack surface that can still repair a disk |
| D5 | Kernel source | Distro archive kernel per arch, pinned by hash | Hardware support without building kernels |
| D6 | UEFI HTTP boot vs TFTP | Prefer UEFI HTTP boot where firmware has it; TFTP fallback | Faster, simpler, loads a UKI directly |

## §9 Falsifiers

Each rule goes RED on a planted defect, with mutation RED→GREEN in the PR body. Contract: `contracts/fj-metal-v1.yaml` (pv), with SHACL shapes for `machine`, `boot_image`, `boot_media`, `disk_layout` and the repair receipt.

| Rule | Planted defect | Must see | Anti-vacuity arm |
| --- | --- | --- | --- |
| 1 Parity | Flip one byte in the USB copy | `parity check` exit 1, names the transport | All 3 copies were read |
| 2 Reproducible | Embed a build timestamp | Hashes differ on 2 hosts → RED | Both builds completed |
| 3 Self-contained | Block all network except the boot server | Repair reaches the approval step | Repair log shows ≥ 1 local bundle read |
| 4 Rebuild | Restore a raw disk image | Refused: no such verb | Verb list read |
| 5 Serial pin | Target a protected serial | Refused before any write | ≥ 1 serial compared |
| 6 Failover, local | Install a kernel entry that panics | Loader falls back after N tries | Tries counter decremented |
| 6 Failover, network | Drop heartbeats | Repair OS after N boots | Server counter reached N |
| 7 No silent destroy | Healthy disk present, no spare declared | Repair reports, installs nothing | Disk health read |
| 8 Receipts | Kill power mid-transition (QEMU) | Next boot resumes from last receipt | ≥ 1 receipt existed |
| 9 Unknown | VM with no disks | Exit 2, never 0 | — |
| 10 Sovereign | Add a Python call to the build | Lint RED | Lint scanned the build graph |
| 11 Signed | Unsigned UKI with Secure Boot on | Firmware refuses; QEMU job RED | Signed image booted in the same job |
| 12 Hardware-free | Remove one transport from the QEMU matrix | Matrix gate RED: 5 of 6 | Matrix lists 6 cells |

## §10 Consumers and sources

Site deployments are expected consumers: their boot servers, USB sticks, boot counters and repair flows become instances of these resource types once M4-M8 land.

**Sources:** [Tinkerbell](https://tinkerbell.org/) · [Tinkerbell Smee](https://tinkerbell.org/docs/services/smee) · [Talos image factory](https://github.com/siderolabs/image-factory) · [systemd automatic boot assessment](https://systemd.io/AUTOMATIC_BOOT_ASSESSMENT) · [systemd-bless-boot](https://www.mankier.com/8/systemd-bless-boot.service) · [ukify(1)](https://www.man7.org/linux/man-pages/man1/ukify.1.html)
