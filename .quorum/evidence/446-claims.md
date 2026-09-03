# Quorum evidence — #446 (feat/446-exec-facts-doctor) — adjudicated claims

## CONFIRMED — 7 claims survived refutation

1. [design] `forjar exec <machine> -- <cmd...>` is the one-off remote command runner the ticket asks for: the operator's argv is shell-quoted word by word and run through the same transport `apply` uses; stdout and stderr are forwarded byte for byte and the remote exit code becomes the process exit code; when stderr contains "Permission denied", ONE extra hint line follows it on stderr (agy lane's correction).
   - evidence: `src/cli/exec.rs` (`shell_quote`, `shell_join`, `cmd_exec`) dispatched from the ops list that already routes `Inventory` (`src/cli/dispatch_misc.rs:72`, `src/cli/dispatch_misc_c.rs:74` at the merge-base); the transport is `crate::transport::exec_script` unchanged. Pinned by `fj446_exec_forwards_streams_and_exit_code` (stdout "hi", stderr "err", status 3 for `sh -c 'echo hi; echo err >&2; exit 3'`) and `fj446_exec_json_carries_exit_code_and_stdout`; `shell_quote_keeps_bare_words_and_wraps_the_rest` and `shell_join_preserves_argv_boundaries` cover a word with a quote and a `$`.

2. [design] `forjar facts <machine>` runs ONE POSIX sh script (one df extension, `-i`, that GNU, BSD and busybox all carry) and parses `key=value` lines; a malformed line is skipped and an unparsable number leaves its field at the default — a uid that does not parse is `None`, rendered `uid ?`, never 0 (agy lane's correction).
   - evidence: `src/cli/facts.rs` (`facts_script`, `parse_facts`, `parse_disk`, `parse_tool`); the script passes forjar's bashrs I8 validation — the first draft used `continue` inside `case … esac` in a `while read` loop and bashrs refused it (SC2242), which the falsifier caught before any host ran it (`fj446_facts_json_reports_identity_path_and_disks` was RED with "I8 violation"); `parse_facts_reads_every_kind_of_line_and_tolerates_junk` feeds a junk line and an unparsable number.

3. [design] `forjar doctor --machine <m>` diagnoses the TARGET, read-only: reachability, facts, the remote PATH (warns when `/usr/local/bin` or `/usr/sbin` is missing — the recurring remote-PATH bug the ticket names), disk and inode pressure at documented thresholds, every `file` resource's destination directory (exists, owner:group, mode, writable by the connecting identity, sudo — the detail is always reported; the status is Fail only when it is not writable), and the executables the machine's resources need (agy lane's correction).
   - evidence: `DoctorArgs` gains `machine: Option<String>` beside `network` (`src/cli/commands/misc_ops_args.rs:8`, `src/cli/commands/misc_ops_args.rs:23` at the merge-base); the arm in `dispatch_misc_tools_b` (`src/cli/dispatch_tools.rs:68`) routes it to `src/cli/doctor_machine.rs`; thresholds `DISK_WARN_FREE_PCT=10`, `DISK_FAIL_FREE_PCT=2`, `DISK_WARN_FREE_KB=1 GiB`, `INODE_WARN_FREE_PCT=5` pinned at their boundaries by `disk_check_thresholds_at_the_boundaries` and `inode_check_warns_below_five_percent_free`; the permission detail names dir, owner:group, mode, user, uid and sudo (`dir_check_names_owner_mode_and_identity_when_unwritable`); on the host, `fj446_doctor_machine_fails_on_unwritable_destination` makes a 0555 directory the destination and asserts a non-zero exit naming it.

4. [probe] The falsifier cannot pass vacuously — under root included.
   - evidence: at the RED commit (tests only, `30f33afd`) all 8 cases fail with `error: unrecognized subcommand 'exec'` / `'facts'` (0/8); on the branch 8/8. Every case drives the built binary and asserts bytes or exit codes; `fj446_doctor_machine_is_not_a_no_op` asks for a machine that does not exist and requires an error; `fj446_doctor_machine_requires_a_config` requires the refusal without `-f`; the unwritable-destination case asserts the naming half (directory, `mode 555`, the connecting identity) under root too and skips only the exit code there (agy lane's correction — the first version returned early under root).

5. [design] No new dependency, no new transport path, no change to the apply path.
   - evidence: `git diff cba05bba --stat` touches only `src/cli/**`, `src/verb/partition.rs` and the two test files; `Cargo.toml`/`Cargo.lock` untouched; `src/core/executor` untouched.

6. [design] The verb partition stays total: `exec` is CliOnly (its value is the terminal rendering and exit code) and `facts` is Pending on the E11 facts model (#414) that will unify it.
   - evidence: `src/verb/partition.rs` rows next to `doctor` (`src/cli/mod.rs:86` registers the modules); `verb::partition::tests::the_partition_is_total` was the one red test in the 13,392-test lib suite after the feature landed and is green with the rows.

7. [probe] A "Permission denied" on the remote side points the operator at the diagnosis instead of leaving them to add debug steps to a YAML.
   - evidence: `permission_hint` in `src/cli/exec.rs` prints one stderr line naming `doctor --machine <m>`; `permission_hint_only_on_permission_denied` pins that it fires only on that text.

## REFUTED — 4 claims killed

1. [design] refuted 1/1 — "Model facts as a full per-host facts cache exposed to `when:` and templates (Ansible `setup`-style), since the ticket names Ansible fact gathering."
   - corrected: that is CRUX E11 (#414), triaged for the next pass; #446's first increment is the transport verb plus a facts report a human or a script can read (`--json`), which E11 can lift into the resolver later. Shipping the cache now would bolt a second facts source onto a resolver that has none.

2. [design] refuted 1/1 — "Let `doctor --machine` fix what it finds (`--fix`, like the local doctor's stale-lock cleanup)."
   - corrected: the ticket's scenarios are permission and PATH problems on a remote host; a fixer there would need sudo and would mutate a machine forjar has not been asked to converge. The remote doctor is read-only by construction (only `stat`, `test -w`, `command -v`, `df`); `apply` remains the only verb that writes.

3. [design] refuted 1/1 (agy lane) — "A number the target reports as nonsense can default to 0; the operator sees a 0 and moves on."
   - corrected: a uid of 0 is root, so the coercion masked exactly the permission problem the verb exists to show. `uid` is `Option<u64>` and renders `uid ?`; every other numeric key keeps its default instead of being overwritten (`a_uid_that_does_not_parse_is_unknown_not_root`).

4. [design] refuted 1/1 (agy lane) — "Facts without network addresses and a package-tool table of three entries are at the industry default."
   - corrected: Ansible `setup`, facter and grains all report addresses first; `facts` now emits `ipv4=` lines (rendered and in `--json`), and the provider→executable table covers apt, dnf, yum, zypper, pacman, apk, brew, snap, cargo, uv and pip.

## KNOWN LIMITS [A]

- `exec` is deliberately unrestricted for the connecting identity (it is the operator's own shell on the machine); it does not consult `policy:` rules and says so nowhere yet — a `policy` hook for ad-hoc exec is queued with E10/#413.
- `doctor --machine` reads destination directories only for `file` resources; package/service resources contribute their tool requirements, not paths.
- `facts` reports the login shell's PATH as the transport sees it, which is what apply's scripts see; an interactive shell's PATH may differ.
- Under root the unwritable-destination case cannot observe a non-zero exit (root writes anywhere); it asserts the naming half only.
- A standalone "permissions" fact is not reported by `facts`: permissions are meaningful against a destination, which `doctor --machine` reports per declared resource.
- Where `df -i` is unavailable the inode figure reads as 0.
