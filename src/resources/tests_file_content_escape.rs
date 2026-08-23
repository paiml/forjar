//! C8 (GH #296): `file` content must never escape its transport and become shell.
//!
//! # The defect these tests pin
//!
//! `push_file_content_lines` wrote inline content into a heredoc with a FIXED
//! delimiter, under a comment asserting the very thing that was false:
//!
//! ```text
//! // Heredoc body is literal (quoted FORJAR_EOF), so content is not shell.
//! cat > '<path>' <<'FORJAR_EOF'
//! <content>
//! FORJAR_EOF
//! ```
//!
//! A heredoc body is literal only up to the first line that equals the
//! delimiter. A managed file whose content contains a `FORJAR_EOF` line closes
//! the heredoc early; every following line of the CONTENT is then read by bash
//! as SHELL and executed on the target machine, while the file on disk keeps
//! only the truncated prefix. If the payload reopens a heredoc to swallow the
//! generator's own trailing delimiter, the script still exits 0 — so apply
//! reports the resource converged, with the wrong bytes on disk and an
//! arbitrary command already run.
//!
//! # Why these tests execute the script
//!
//! Asserting on the generated TEXT is what let this survive: the text always
//! contained `FORJAR_EOF` and always looked right. These tests generate the
//! apply script, run it through the REAL local transport (`bash`, script on
//! stdin — exactly how forjar runs it), and then assert on the filesystem:
//! the managed file is byte-exact, and a canary path that only an escaping
//! payload could create does not exist. A generator can only pass by actually
//! being right.
//!
//! The fix is to give the content no delimiter to escape: it is base64-encoded
//! and decoded on the target, the same transport the `source:` field has always
//! used. Byte-exactness comes free — a heredoc can never reproduce content that
//! does not end in a newline, because it appends one.

use crate::core::types::{MachineTarget, Resource, ResourceType};
use crate::resources::file::apply_script;

/// One case's private scratch dir: the managed file, plus the canary path an
/// escaping payload is told to create.
struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("tempdir"),
        }
    }

    /// Path of the file the `file` resource manages.
    fn target(&self) -> String {
        self.dir.path().join("managed.conf").display().to_string()
    }

    /// Path no correct run ever creates. Its existence is proof that content
    /// crossed the data/code boundary and executed.
    fn canary(&self) -> String {
        self.dir.path().join("PWNED").display().to_string()
    }

    fn canary_exists(&self) -> bool {
        self.dir.path().join("PWNED").exists()
    }
}

fn file_resource(path: &str, content: &str) -> Resource {
    Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m1".to_string()),
        path: Some(path.to_string()),
        state: Some("file".to_string()),
        content: Some(content.to_string()),
        ..Default::default()
    }
}

/// Generate the apply script and run it the way forjar runs it: `bash`, script
/// on stdin, via the real local transport. Returns the exit code.
fn apply(sb: &Sandbox, content: &str) -> i32 {
    let r = file_resource(&sb.target(), content);
    let script = apply_script(&r);
    let out = crate::transport::local::exec_local(&script, None).expect("bash is available");
    out.exit_code
}

/// The whole contract in one assertion: apply converges, the managed file holds
/// the declared bytes and nothing else, and no command the content asked for
/// was executed.
fn assert_written_verbatim(sb: &Sandbox, content: &str) {
    let code = apply(sb, content);
    assert!(
        !sb.canary_exists(),
        "content ESCAPED and executed as shell on the target: {} was created by content {:?}",
        sb.canary(),
        content
    );
    assert_eq!(code, 0, "apply script did not converge for {content:?}");
    let on_disk = std::fs::read(sb.target()).unwrap_or_default();
    assert_eq!(
        String::from_utf8_lossy(&on_disk),
        content,
        "file on disk is not byte-exact ({} bytes written, {} declared)",
        on_disk.len(),
        content.len()
    );
}

// --- The escape itself -----------------------------------------------------

#[test]
fn content_equal_to_the_delimiter_is_written_not_interpreted() {
    // The whole file is one `FORJAR_EOF` line: the heredoc body closes before
    // it has taken a single byte, and the generator's own trailer is then run
    // as a command.
    let sb = Sandbox::new();
    assert_written_verbatim(&sb, "FORJAR_EOF");
}

#[test]
fn delimiter_mid_file_does_not_truncate_the_file() {
    let sb = Sandbox::new();
    assert_written_verbatim(&sb, "alpha\nFORJAR_EOF\nbeta\n");
}

#[test]
fn delimiter_plus_trailing_shell_does_not_execute_and_does_not_report_converged() {
    // The canonical exploit. The payload closes the heredoc, runs a command,
    // then REOPENS a heredoc so the generator's own trailing `FORJAR_EOF` is
    // consumed as data — the script exits 0 and apply calls it converged.
    let sb = Sandbox::new();
    let content = format!(
        "harmless=1\nFORJAR_EOF\ntouch {}\ncat > /dev/null <<'FORJAR_EOF'\nswallowed",
        sb.canary()
    );
    assert_written_verbatim(&sb, &content);
}

#[test]
fn delimiter_plus_trailing_shell_with_command_substitution() {
    // Same escape, different payload shape: `$(...)` in a position that is
    // shell only if the escape happened.
    let sb = Sandbox::new();
    let content = format!(
        "x\nFORJAR_EOF\necho $(touch {}) >/dev/null\ncat > /dev/null <<'FORJAR_EOF'\nswallowed",
        sb.canary()
    );
    assert_written_verbatim(&sb, &content);
}

// --- Delimiter look-alikes: whitespace and line-ending variants ------------

#[test]
fn delimiter_with_trailing_space_is_preserved_verbatim() {
    let sb = Sandbox::new();
    assert_written_verbatim(&sb, "alpha\nFORJAR_EOF \nbeta\n");
}

#[test]
fn delimiter_with_trailing_tab_is_preserved_verbatim() {
    let sb = Sandbox::new();
    assert_written_verbatim(&sb, "alpha\nFORJAR_EOF\t\nbeta\n");
}

#[test]
fn delimiter_with_leading_whitespace_is_preserved_verbatim() {
    let sb = Sandbox::new();
    assert_written_verbatim(&sb, "alpha\n  FORJAR_EOF\n\tFORJAR_EOF\nbeta\n");
}

#[test]
fn crlf_delimiter_line_is_preserved_verbatim() {
    // CRLF content is a real case (Windows-authored config). The carriage
    // return must survive the round trip byte for byte.
    let sb = Sandbox::new();
    assert_written_verbatim(&sb, "alpha\r\nFORJAR_EOF\r\nbeta\r\n");
}

#[test]
fn crlf_payload_with_trailing_shell_does_not_execute() {
    let sb = Sandbox::new();
    let content = format!(
        "x\r\nFORJAR_EOF\r\ntouch {}\r\ncat > /dev/null <<'FORJAR_EOF'\r\nswallowed",
        sb.canary()
    );
    assert_written_verbatim(&sb, &content);
}

// --- Every delimiter forjar might pick -------------------------------------

#[test]
fn content_containing_every_delimiter_forjar_might_pick() {
    // A file that names every marker in the codegen vocabulary, plus a family
    // of numbered derivatives, and then tries to execute. No choice of a FIXED
    // or content-DERIVED delimiter drawn from this vocabulary can be absent
    // from this content.
    let sb = Sandbox::new();
    let mut content = String::new();
    for marker in ["FORJAR_EOF", "FORJAR_SUDO", "FORJAR_B64", "EOF", "EOT"] {
        content.push_str(&format!("{marker}\n"));
        for n in 0..8 {
            content.push_str(&format!("{marker}_{n}\n"));
        }
    }
    content.push_str(&format!("touch {}\n", sb.canary()));
    assert_written_verbatim(&sb, &content);
}

// --- Byte-exactness a heredoc cannot deliver -------------------------------

#[test]
fn content_without_a_trailing_newline_is_not_padded() {
    // A heredoc body is a sequence of newline-terminated lines, so it can only
    // ever produce content ending in `\n`. Declared content that does not end
    // in one must not gain one.
    let sb = Sandbox::new();
    assert_written_verbatim(&sb, "no-trailing-newline");
}

#[test]
fn content_with_a_trailing_newline_is_not_doubled() {
    let sb = Sandbox::new();
    assert_written_verbatim(&sb, "key=value\n");
}

#[test]
fn empty_content_writes_an_empty_file() {
    let sb = Sandbox::new();
    assert_written_verbatim(&sb, "");
}

// --- Data stays data -------------------------------------------------------

#[test]
fn shell_metacharacters_are_written_literally_not_expanded() {
    let sb = Sandbox::new();
    assert_written_verbatim(&sb, "$HOME ${PATH} $(whoami) `id` \"q\" 'q' \\ ; | &\n");
}

#[test]
fn unicode_and_tabs_survive_byte_for_byte() {
    let sb = Sandbox::new();
    assert_written_verbatim(&sb, "café ☕\tπ = 3.14159\n日本語\n");
}

// --- For ANY content whatsoever --------------------------------------------

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(48))]

    /// The invariant, over content assembled from the fragments most likely to
    /// break a delimited encoding: markers, bare newlines, CR, whitespace and
    /// shell metacharacters.
    #[test]
    fn arbitrary_content_round_trips_byte_exact(
        parts in proptest::collection::vec(
            proptest::sample::select(vec![
                "FORJAR_EOF", "FORJAR_SUDO", "FORJAR_B64", "EOF", "\n", "\r\n", "\r",
                " ", "\t", "a", "$(id)", "'", "\"", "\\", ";", "#", "café",
            ]),
            0..24,
        )
    ) {
        let content: String = parts.concat();
        let sb = Sandbox::new();
        let r = file_resource(&sb.target(), &content);
        let script = apply_script(&r);
        let out = crate::transport::local::exec_local(&script, None).expect("bash");
        proptest::prop_assert!(!sb.canary_exists());
        proptest::prop_assert_eq!(out.exit_code, 0, "apply failed for {:?}", content);
        let on_disk = std::fs::read(sb.target()).unwrap_or_default();
        proptest::prop_assert_eq!(
            String::from_utf8_lossy(&on_disk).to_string(),
            content
        );
    }
}
