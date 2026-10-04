# Claims — forjar 1.33.0-rc.1 release PR, #647 + #648 on main (PMAT-651)

Counts and heads live in `PMAT-651-lanes.md`.

- **C1** check_script is RED when more than one filesystem (autofs excluded) is mounted at the target, even when the top one matches the declared source and options
- **C2** for an x-systemd.automount entry, apply writes fstab, runs systemctl daemon-reload BEFORE detaching, detaches the whole stack with a plain umount, and mounts by touching the path, never with its own mount -t
- **C3** under an automount, a busy stack fails loudly and is never lazily detached, on the options-drift path and on the wrong-source path alike
- **C4** a plain mount (no x-systemd.automount) keeps mount -t and never calls systemctl
- **C5** query_latency_under_50ms asserts the best of 5 samples against the 50ms bound, so one cold sample under load cannot turn it red
- **C6** the diff against main is exactly #647, #648, the PMAT-648 and PMAT-651 roadmap rows and the 1.33.0-rc.1 bump, with no workflow change

## The measured symptom

forjar#648: on lambda-labs the NAS share is `noauto,x-systemd.automount`.
1.33.0-rc.1's #642 remount ran its own `mount -t cifs`; touching the trigger
fired systemd's automount from a unit that still held the OLD options, and
forjar's mount landed on top. The check read the top mount only and called the
stack converged. forjar#647: a single cold FTS5 timing sample went red under
load and blocked the rc clean-room.
