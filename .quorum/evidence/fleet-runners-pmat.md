# PMAT-547 — instruments, and what each said

    cargo test --no-fail-fast   9 workflow-reading suites at HEAD: 110 passed, 0 failed
    cargo clippy --tests        clean
    cargo fmt --all             clean
    python3 yaml.safe_load      all 20 workflow files parse, before and after
    actionlint                  base 0bb6cb57: 22 findings · this branch: 15
    bash -n                     the new glibc step parses, on both the x86_64
                                and the aarch64 substitution
    the glibc step, run         x86_64: `builder: ldd (Ubuntu GLIBC
                                2.35-0ubuntu3.11) 2.35`, `binary demands at most
                                GLIBC_2.34`; aarch64: `builder: not applicable —
                                … was cross-built`, same demanded floor
    gh api .../actions/jobs     the measurement the ticket rests on: runner_name
                                read back per job rather than taken from the
                                workflow file
    gh api orgs/paiml/actions/runners
                                25 runners, every one Linux; 0 macOS, 0 Windows
    analyze_vacuous_tests       below

The 15 actionlint findings that remain are pre-existing shellcheck infos inside
`run:` blocks — `SC2012`, `SC2035`, `SC2016`, `SC2013`, `SC2086`, `SC2010` — and
none is in a block this branch wrote. The drop from 22 is the unknown-label
noise that `.github/actionlint.yaml` removes; without that file every
`clean-room` in the repository reports as an unknown label and the real findings
are buried.

VACUOUS SCAN. `pmat analyze vacuous-tests` reports 432 of 19720 `#[test]` fns
cannot fail (2.2%) across 2177 parsed files, plus 3 that skip silently. Neither
`falsification_every_ci_job_runs_on_the_fleet` nor
`falsification_hosted_jobs_do_not_cache_target` appears. Necessary and not
sufficient, and this ticket is a good argument for saying so out loud twice
over: the scan cannot see that a parser silently returns nothing for the object
form of `runs-on`, and it cannot see that a suite's denominator has gone to
zero — which is exactly the second failure mode that
`the_scan_reaches_the_real_coverage_job` exists to catch and did catch, without
help from any scan.

TOOL DEFECTS FOUND. The paiml-implement delegate hit its 30-turn cap before
writing a receipt, for the thirteenth time this session; filed as
paiml-implement#141. The three lane JSONs were read directly from the out_dir.
