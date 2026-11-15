use super::*;

fn req() -> JitRunnerRequest {
    JitRunnerRequest {
        labels: vec!["forjar-pool".into(), "linux-x64".into()],
        ephemeral: true,
        token_ttl_secs: MAX_TOKEN_TTL_SECS,
        token_source: "RUNNER_JIT_TOKEN".into(),
    }
}

fn with_labels(labels: &[&str]) -> JitRunnerRequest {
    JitRunnerRequest {
        labels: labels.iter().map(|s| (*s).to_string()).collect(),
        ..req()
    }
}

fn with_source(s: &str) -> JitRunnerRequest {
    JitRunnerRequest {
        token_source: s.into(),
        ..req()
    }
}

#[test]
fn accepts_a_valid_ephemeral_request() {
    let c = validate(&req()).expect("valid");
    assert_eq!(c.labels().len(), 2);
    assert_eq!(c.token_ttl_secs(), 3600);
    assert_eq!(c.token_source(), "RUNNER_JIT_TOKEN");
}

#[test]
fn refuses_zero_ttl() {
    let r = JitRunnerRequest {
        token_ttl_secs: 0,
        ..req()
    };
    assert_eq!(validate(&r), Err(JitConfigError::ZeroTtl));
}

#[test]
fn accepts_ttl_of_one_second() {
    let r = JitRunnerRequest {
        token_ttl_secs: 1,
        ..req()
    };
    assert_eq!(validate(&r).expect("valid").token_ttl_secs(), 1);
}

#[test]
fn refuses_ttl_over_one_hour() {
    let r = JitRunnerRequest {
        token_ttl_secs: 3601,
        ..req()
    };
    assert_eq!(validate(&r), Err(JitConfigError::TtlTooLong { secs: 3601 }));
}

#[test]
fn refuses_non_ephemeral_runner() {
    let r = JitRunnerRequest {
        ephemeral: false,
        ..req()
    };
    assert_eq!(validate(&r), Err(JitConfigError::NotEphemeral));
}

#[test]
fn refuses_empty_label_set() {
    assert_eq!(
        validate(&with_labels(&[])),
        Err(JitConfigError::EmptyLabelSet)
    );
}

#[test]
fn refuses_blank_label_first_or_second() {
    assert_eq!(
        validate(&with_labels(&["  "])),
        Err(JitConfigError::BlankLabel)
    );
    assert_eq!(
        validate(&with_labels(&[""])),
        Err(JitConfigError::BlankLabel)
    );
    assert_eq!(
        validate(&with_labels(&["ok", ""])),
        Err(JitConfigError::BlankLabel)
    );
}

#[test]
fn checks_every_label_not_only_the_first() {
    assert_eq!(
        validate(&with_labels(&["ok", "a,b"])),
        Err(JitConfigError::LabelHasComma {
            label: "a,b".into()
        })
    );
}

#[test]
fn refuses_comma_in_label() {
    assert_eq!(
        validate(&with_labels(&["a,b"])),
        Err(JitConfigError::LabelHasComma {
            label: "a,b".into()
        })
    );
}

#[test]
fn refuses_whitespace_and_control_in_label() {
    for bad in [
        " lead", "trail ", "a b", "a\tb", "a\nb", "a\u{7}b", "a\u{a0}b",
    ] {
        assert_eq!(
            validate(&with_labels(&[bad])),
            Err(JitConfigError::LabelHasWhitespaceOrControl { label: bad.into() }),
            "{bad:?}"
        );
    }
}

#[test]
fn refuses_shell_metacharacters_in_label() {
    for bad in [
        "a;b", "a|b", "a&b", "a$b", "a`b", "a'b", "a\"b", "a\\b", "a<b", "a>b", "a(b)", "a{b}",
        "a*", "a?", "a!b", "a#b", "~a", "a=b", "a/b",
    ] {
        assert_eq!(
            validate(&with_labels(&[bad])),
            Err(JitConfigError::LabelForbiddenChar { label: bad.into() }),
            "{bad:?}"
        );
    }
}

#[test]
fn refuses_label_over_256_bytes_but_accepts_exactly_256() {
    let ok = "a".repeat(256);
    assert!(validate(&with_labels(&[&ok])).is_ok());
    let long = "a".repeat(257);
    assert_eq!(
        validate(&with_labels(&[&long])),
        Err(JitConfigError::LabelTooLong { len: 257 })
    );
}

#[test]
fn refuses_duplicate_labels_case_insensitively() {
    assert_eq!(
        validate(&with_labels(&["Linux", "x64", "linux"])),
        Err(JitConfigError::DuplicateLabel {
            label: "linux".into()
        })
    );
    assert_eq!(
        validate(&with_labels(&["a", "a"])),
        Err(JitConfigError::DuplicateLabel { label: "a".into() })
    );
}

#[test]
fn accepts_capability_labels_with_digits_and_dots() {
    // A pure string rule cannot tell `cuda-12` from `node-12`, so none is guessed.
    let r = with_labels(&[
        "cuda-12",
        "ubuntu-24.04",
        "gpu-2",
        "build-host",
        "self_hosted:x",
    ]);
    assert_eq!(validate(&r).expect("valid").labels().len(), 5);
}

#[test]
fn refuses_empty_token_source() {
    for s in ["", "   "] {
        assert_eq!(
            validate(&with_source(s)),
            Err(JitConfigError::MissingTokenSource),
            "{s:?}"
        );
    }
}

#[test]
fn refuses_token_source_naming_a_path() {
    for bad in [
        "/run/secrets/tok",
        "./tok",
        "~/tok",
        ".tok",
        "~tok",
        "a\\b",
        "file:tok",
        "FILE:tok",
        "File:tok",
    ] {
        assert_eq!(
            validate(&with_source(bad)),
            Err(JitConfigError::TokenSourceIsPath),
            "{bad}"
        );
    }
}

#[test]
fn accepts_env_var_names() {
    for ok in ["A", "_", "_A1", "RUNNER_JIT_TOKEN", &"A".repeat(64)] {
        assert!(validate(&with_source(ok)).is_ok(), "{ok}");
    }
}

#[test]
fn refuses_token_shaped_source() {
    let fake = format!("ghs_{}", "x".repeat(30));
    assert_eq!(
        validate(&with_source(&fake)),
        Err(JitConfigError::TokenSourceNotEnvName)
    );
}

#[test]
fn refuses_other_non_name_sources() {
    for bad in [
        "$(curl x)",
        "tok;rm",
        "A B",
        "lower",
        "Mixed",
        "1ABC",
        "A-B",
        "A=B",
        "ÄB",
    ] {
        assert_eq!(
            validate(&with_source(bad)),
            Err(JitConfigError::TokenSourceNotEnvName),
            "{bad}"
        );
    }
}

#[test]
fn refuses_over_length_source() {
    let long = "A".repeat(65);
    assert_eq!(
        validate(&with_source(&long)),
        Err(JitConfigError::TokenSourceTooLong { len: 65 })
    );
}

#[test]
fn token_source_errors_never_echo_the_value() {
    let fake = format!("ghs_{}", "x".repeat(30));
    for s in [
        fake.clone(),
        format!("/{fake}"),
        fake.to_uppercase() + &"Z".repeat(80),
    ] {
        let e = validate(&with_source(&s)).expect_err("refused");
        assert!(!format!("{e}").contains("xxxx"), "{e}");
        assert!(!format!("{e:?}").contains("xxxx"), "{e:?}");
        assert!(!format!("{e}").contains("XXXX"), "{e}");
    }
}
