# Claims — a mount's ownership options are checked state (PMAT-642)

Counts and heads live in `PMAT-642-lanes.md`.

- **C1** check_script is RED when the declared source is mounted but a declared uid, gid, file_mode, dir_mode or mode differs from the live findmnt OPTIONS
- **C2** modes compare as numbers, user and group names resolve to ids, and a key the kernel omits is compared as its kernel default, never skipped
- **C3** apply remounts on that drift with a plain umount and no lazy fallback; a busy mount fails loudly naming the path and is never mounted over
- **C4** the tests execute the generated scripts against a fake findmnt, umount and mount, and go RED when options_condition is neutralised
- **C5** a converged mount, and a declaration with no ownership option, keep the source-only behaviour

## The measured symptom

forjar#642: the check compared only `findmnt -o SOURCE`. On lambda-labs the NAS
share must move from gid=1000 to gid=courses so the `course` user can write it
(paiml/infra#1208), and 1.32.0 would have reported the resource converged while
the kernel kept gid=1000.
