# Claims — forjar#590: --refresh consults a templated completion_check

Briefed to every lane at head e18cd9ec (one commit on main ce0a7cc2).

- C1: the check was consulted and passed; the command ran because the entry `--refresh` recorded was hashed over the RAW declaration while the planner compares against `hash_desired_state` over the RESOLVED one, so a templated resource planned as `~ update`.
- C2: `record_converged` now resolves each candidate once, with the config's own secrets as the planner does (FJ-154), runs the check on that resolved resource and hashes that same resource; an unresolvable candidate is not recorded.
- C3: `falsification_590_refresh_templated_check` drives the real binary over a fresh state dir, a failed lock entry and `-r guard`, with an untemplated control, and a marker file proves whether the command ran; an unlatched entry must also survive the next plain apply.
- C4: with only the hash change reverted the three templated tests fail and the control passes; with only the secrets change reverted all four pass.
