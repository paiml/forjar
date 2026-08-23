//! C8, second instance (GH #296): SSH authorized_keys could close their own
//! heredoc and execute as shell — in a `$SUDO` context.
//!
//! `user.rs` built the authorized_keys write as:
//!
//!     cat > /tmp/forjar-authkeys <<'FORJAR_EOF'
//!     <keys joined by newline>
//!     FORJAR_EOF
//!     $SUDO mv /tmp/forjar-authkeys '<home>'/.ssh/authorized_keys
//!     $SUDO chmod 600 ...
//!     $SUDO chown -R ...
//!
//! A key entry containing a line equal to `FORJAR_EOF` closes the heredoc early.
//! Everything after it is parsed as shell by the target — and the very next
//! lines in this template are `$SUDO`, so the injected commands land beside a
//! privilege escalation the operator already granted. The file that does get
//! written is silently truncated to whatever preceded the delimiter, so the
//! machine also ends up with the WRONG authorized_keys and no error.
//!
//! The `file.rs` fix (C8, first instance) did not cover this. That is the point
//! of these tests: the defect was a CLASS — content interpolated into a fixed
//! heredoc delimiter — and fixing the reported instance left the security-
//! critical one in place.
//!
//! Every assertion here is on the CONTENT the script deploys, never on the
//! script's text. Asserting on text is what let the original defect live: the
//! old tests looked for the delimiter and the key material, and both were
//! present even in the scripts that had already escaped.

use crate::core::shell_escape::decode_written_file;
use crate::core::types::{MachineTarget, Resource, ResourceType};
use crate::resources::user::apply_script;

const AUTHKEYS_PATH: &str = "/tmp/forjar-authkeys";

fn user_with_keys(keys: &[&str]) -> Resource {
    Resource {
        resource_type: ResourceType::User,
        machine: MachineTarget::Single("m1".to_string()),
        name: Some("noah".to_string()),
        home: Some("/home/noah".to_string()),
        ssh_authorized_keys: keys.iter().map(|k| (*k).to_string()).collect(),
        ..Default::default()
    }
}

/// The bytes the generated script actually deploys to the authorized_keys temp
/// file, decoded from the transport rather than scraped out of the script text.
fn deployed(r: &Resource) -> Vec<u8> {
    let script = apply_script(r);
    decode_written_file(&script, AUTHKEYS_PATH)
        .unwrap_or_else(|| panic!("no authorized_keys write found in script:\n{script}"))
}

#[test]
fn a_key_containing_the_delimiter_does_not_escape() {
    let hostile = "ssh-ed25519 AAAAC3Nza ok\nFORJAR_EOF\ntouch /tmp/PWNED\ncat > /dev/null <<'FORJAR_EOF'\nswallowed";
    let r = user_with_keys(&[hostile]);
    let script = apply_script(&r);

    // No heredoc means no delimiter to hit. This is the structural guarantee;
    // the content assertions below are what prove it is also correct.
    assert!(
        !script.contains("FORJAR_EOF"),
        "authorized_keys still written through a fixed-delimiter heredoc:\n{script}"
    );
    assert_eq!(
        deployed(&r),
        hostile.as_bytes(),
        "the delimiter-bearing key was not deployed byte-exactly"
    );
}

#[test]
fn the_injected_command_is_not_present_as_shell() {
    let hostile = "k\nFORJAR_EOF\ntouch /tmp/PWNED-AUTHKEYS\n";
    let r = user_with_keys(&[hostile]);
    let script = apply_script(&r);

    // `touch /tmp/PWNED-AUTHKEYS` must appear only inside the encoded payload,
    // never as a line the target's shell would execute.
    let executable_lines: Vec<&str> = script
        .lines()
        .filter(|l| !l.contains("base64 -d"))
        .collect();
    assert!(
        !executable_lines.iter().any(|l| l.contains("PWNED")),
        "injected command reached an executable line:\n{}",
        executable_lines.join("\n")
    );
}

#[test]
fn multiple_keys_round_trip_including_a_hostile_one() {
    let a = "ssh-ed25519 AAAAfirst first@host";
    let b = "ssh-ed25519 AAAAsecond second@host";
    let hostile = "FORJAR_EOF";
    let r = user_with_keys(&[a, hostile, b]);

    // The join order and separator are part of the contract: a truncating
    // escape would silently drop `b`, which is exactly the data-loss half of
    // this defect and is invisible without asserting on the full payload.
    assert_eq!(
        deployed(&r),
        format!("{a}\n{hostile}\n{b}").as_bytes(),
        "keys after a delimiter-valued entry were lost"
    );
}

#[test]
fn a_key_that_is_exactly_the_delimiter_survives() {
    let r = user_with_keys(&["FORJAR_EOF"]);
    assert_eq!(deployed(&r), b"FORJAR_EOF");
}

#[test]
fn crlf_line_endings_do_not_escape() {
    // A CRLF delimiter line defeats a naive `line == delimiter` guard while
    // still terminating a heredoc on some shells.
    let hostile = "x\r\nFORJAR_EOF\r\ntouch /tmp/PWNED-CRLF\r\n";
    let r = user_with_keys(&[hostile]);
    assert!(!apply_script(&r).contains("FORJAR_EOF\r"));
    assert_eq!(deployed(&r), hostile.as_bytes());
}

#[test]
fn ordinary_keys_are_unchanged_by_the_fix() {
    // The control. A fix that mangles normal input is not a fix.
    let a = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJzXucj3 lambda-to-osx";
    let r = user_with_keys(&[a]);
    assert_eq!(deployed(&r), a.as_bytes());
}
