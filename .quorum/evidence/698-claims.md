# Claims — forjar#698 (phase 1): the I8 gate on bashrs 7.4.2, false SC1066 inside double quotes

Briefed to every lane with the ticket and the full diff against main
9a37a2ff; head 6ba722fb. The PR carries `Refs PMAT-698`, not a closing
keyword: the SC2075 fixture (bashrs#441) the ticket also names is a later
phase and the ticket stays open.

- C1: the lock resolves bashrs 7.4.2 (`cargo tree -i bashrs --depth 0`), admitted by the existing `bashrs = "7.4"`; no manifest change.
- C2: on 7.4.2 the I8 gate (`validate_script`) passes `ids="$ids $WANT=$id"`, `echo "setting $p=$v now"`, and both in one script.
- C3: the control still rejects a real `$foo=bar`, and by SC1066, so the pass was not bought by dropping the rule.
- C4: the falsifier discriminates: with only the lock moved back to bashrs 7.4.1, the three pass-cases go red on SC1066 and the control stays green.
- C5: the other lock edges (windows-sys, getrandom) are cargo's own resolution of `cargo update -p bashrs --precise 7.4.2`, not a hand edit.
