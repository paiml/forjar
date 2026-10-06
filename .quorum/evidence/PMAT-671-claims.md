# Claims — forjar#671: the nightly aarch64 legs can run the cross they install

Briefed to every lane at head af12be69 (61948daf merged with main ce0a7cc2;
the diff against the merge base is byte-identical, git patch-id 216d52fd).

- C1: nightly.yml's "Install cross (aarch64 legs only)" step now writes `$CARGO_HOME/bin` to `$GITHUB_PATH`, so the next step's shell resolves the `cross` that `cargo install` put in the private CARGO_HOME.
- C2: the #611 falsifier no longer reads one step by name. It discovers every step of every workflow whose `run:` says `cargo install cross`, and the discovery must find both the release.yml and the nightly.yml step.
- C3: for each discovered step and each aarch64 target (gnu and musl), the test runs the step's own script with a fake `cargo` that installs where the real one does, replays `$GITHUB_PATH` the way the runner does, and asks the next step's shell for `cross`.
- C4: with nightly.yml reverted to main the test goes red by name for both targets; restored, it is green.
