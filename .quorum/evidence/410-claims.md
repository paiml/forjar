# Claims — forjar#410: seal a sandbox's output in-process

Briefed to every lane at head 01f5e86b, base ce0a7cc2.

- C1: `seal_output` hashes `$out` with the store's own content hash and renames it to `<store>/<hex>/content`, then re-hashes the moved entry.
- C2: a symlink or special file anywhere under `$out` is refused before anything is hashed or moved, because the directory hash skips symlinks.
- C3: an entry already at the address is re-hashed: equal leaves `$out` alone, unequal refuses and overwrites nothing.
- C4: plan steps 8 and 9 carry no shell command, name neither `forjar-hash-dir` nor a literal `/HASH/` path, and say NOT EXECUTED.
- C5: `execute_sandbox_plan` still refuses by name, now naming seccomp-bpf as the one missing mechanism.
