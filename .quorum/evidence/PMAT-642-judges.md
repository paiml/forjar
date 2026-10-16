# Judges — a mount's ownership options are checked state (PMAT-642)

Round count and heads: see `PMAT-642-lanes.md`.

## CONFIRMED

1. [check] C1 — With the declared source mounted and the kernel reporting gid=1000 against a declared gid=997, the generated check exits non-zero, and with every declared ownership key live it exits zero. All three round-2 lanes read options_condition and agreed.
   - evidence: the red case is asserted at `src/resources/tests_mount_options.rs:93` and the green case at `src/resources/tests_mount_options.rs:103`, both by running check_script under sh against the fake findmnt built at `src/resources/tests_mount_options.rs:25`.
2. [compare] C2 — A declared mode=0755 matches a live mode=755, a group name is resolved with getent before comparing, and an omitted uid, gid or mode is compared as 0, 0 or 1777. An omitted file_mode or dir_mode never matches a declared value, because cifs always echoes both.
   - evidence: the numeric-mode case is at `src/resources/tests_mount_options.rs:110`, the omitted-key cases at `src/resources/tests_mount_options.rs:125`, `src/resources/tests_mount_options.rs:130` and `src/resources/tests_mount_options.rs:132`, and the group-name case at `src/resources/tests_mount_options.rs:139` and `src/resources/tests_mount_options.rs:141`.
3. [apply] C3 — On ownership drift, apply runs a plain umount and then mount with the declared options, with no -l anywhere. When umount fails because the path is busy, the script exits non-zero and never calls mount over the busy path.
   - evidence: the remount is asserted at `src/resources/tests_mount_options.rs:157` with no lazy detach at `src/resources/tests_mount_options.rs:162`; the busy refusal is asserted at `src/resources/tests_mount_options.rs:170`, and the absence of any mount call at `src/resources/tests_mount_options.rs:176`.
4. [test] C4 — The tests run the generated scripts, not their text. With options_condition returning None, 5 of the 8 round-1 tests went RED; with the old wildcard restored, the omitted-key test went RED; each restore went green.
   - evidence: the scripts execute through `run_in` at `src/resources/tests_mount_options.rs:63`, under sh for checks and bash for apply scripts, and the omitted-key falsifier is `src/resources/tests_mount_options.rs:122`.
5. [scope] C5 — A converged mount triggers no umount, and a declaration with no ownership key keeps the source-only check, so existing mounts that declare none are untouched.
   - evidence: the converged case is asserted at `src/resources/tests_mount_options.rs:185` and the source-only case at `src/resources/tests_mount_options.rs:147`. The branch changes mount.rs, mod.rs, the new test file and one appended roadmap entry.

## REFUTED

1. [check] R1 — The first head claimed that treating a key the kernel omits as the default was sound. In fact `[ -z "$_fj_v" ] || ...` let an omitted key match ANY declared value, so a declared uid=1000 over a tmpfs that omits uid (which means 0) read as converged: the false green #642 exists to remove.
   - corrected: lanes 1 and 2 of round 1 refuted it. Commit 0b5882e0 compares an omitted key as its kernel default through omitted_default, and `src/resources/tests_mount_options.rs:125` goes RED with the old wildcard restored and GREEN with the fix.
