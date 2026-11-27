//! jq helpers for the census: one JSON document in, one value out, failing closed.
#![allow(dead_code)]
use super::*;

pub fn jq(body: &str, prog: &str) -> String {
    let mut ch = Command::new("jq")
        .args(["-r", prog])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("jq");
    std::io::Write::write_all(&mut ch.stdin.take().expect("stdin"), body.as_bytes()).expect("feed");
    String::from_utf8_lossy(&ch.wait_with_output().expect("wait").stdout)
        .trim()
        .to_string()
}

/// Feeds jq its input. A BrokenPipe means jq exited early on malformed input, which the
/// caller's exit status judges; any other write error means the input never fully arrived.
pub fn write_fed(mut stdin: std::process::ChildStdin, body: &str) -> bool {
    match std::io::Write::write_all(&mut stdin, body.as_bytes()) {
        Ok(()) => true,
        Err(e) => e.kind() == std::io::ErrorKind::BrokenPipe,
    }
}

/// The one `reviewed_head` of a canary receipt, untrimmed. The whole input is slurped so
/// that two documents, or a trailing token, fail closed (empty) rather than join into a
/// valid-looking head; a trailing newline in the value stays in the result.
pub fn canary_head_single(body: &str) -> String {
    let prog = r#"[.[] | .quorum.canary.reviewed_head | strings] | if length == 1 then .[0] else error("not one reviewed_head") end"#;
    let mut ch = Command::new("jq")
        .args(["-j", "-s", prog])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("jq");
    // jq exits early on malformed input and closes its stdin; a broken pipe here is the
    // fail-closed path, so the write result is not an error to panic on.
    let fed = write_fed(ch.stdin.take().expect("stdin"), body);
    let out = ch.wait_with_output().expect("wait");
    if !fed || !out.status.success() {
        return String::new();
    }
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// A receipt must be exactly one JSON document. Two documents or a trailing token would
/// otherwise slip past the `has` test in `census_one_mode` (`jq -r` prints one line per
/// document, so `yes` becomes `yes\nyes`) and be judged as a plain receipt. Refuse them first.
pub fn census_one_doc(body: &str, rel: &str) -> Result<(), String> {
    let refuse = || Err(format!("{rel}: receipt is not one JSON document"));
    let mut ch = Command::new("jq")
        .args(["-j", "-s", "length"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("jq");
    let fed = write_fed(ch.stdin.take().expect("stdin"), body);
    let out = ch.wait_with_output().expect("wait");
    if fed && out.status.success() && out.stdout == b"1" {
        Ok(())
    } else {
        refuse()
    }
}
