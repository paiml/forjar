//! forjar#649: the linked bashrs must not read `disk-watch` as an OpenAI key.
//!
//! bashrs 6.68.0 matched the bare substring `sk-`, so `disk-watch`, `ask-lane`
//! and `desk-x` were refused by the I8 gate, but only inside a quoted string
//! (an unquoted `systemctl enable ci-disk-watch.timer` was never flagged, so a
//! test using it passes on 6.68.0 too). The pin moved to 7.4.2 (bashrs#351
//! token boundary); the quoted forms below are red on 6.68.0 and green on 7.4.2.

use bashrs::linter::lint_shell;

fn sec005(script: &str) -> Vec<String> {
    lint_shell(script)
        .diagnostics
        .iter()
        .filter(|d| d.code == "SEC005")
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn words_ending_in_sk_are_not_api_keys() {
    for word in ["disk-watch", "ask-lane", "desk-lamp"] {
        for s in [
            format!("#!/bin/sh\nX=\"ci-{word}.timer\"\n"),
            format!("#!/bin/sh\nU=\"https://x/{word}\"\n"),
        ] {
            assert!(
                sec005(&s).is_empty(),
                "{word} flagged in {s:?}: {:?}",
                sec005(&s)
            );
        }
    }
}

#[test]
fn a_real_key_shaped_literal_is_still_flagged() {
    let key = format!("sk-{}", "A1b2C3d4E5f6G7h8I9j0K1l2M3n4O5p6Q7r8S9t0U1v2");
    let s = format!("#!/bin/sh\nKEY=\"{key}\"\n");
    assert!(
        !sec005(&s).is_empty(),
        "over-correction: real key shape not flagged"
    );
}
