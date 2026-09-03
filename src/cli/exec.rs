//! `forjar exec` — one command, one machine, no YAML (#446).
//!
//! The ticket's smallest ask: while provisioning a host you need to know
//! something about it RIGHT NOW — `ls -la` on a destination, `id -un`, `df -h`
//! — and writing a `forjar.yaml` task to ask one question is friction that
//! sends the operator back to a raw `ssh` session, outside forjar's transport,
//! its identity and its logging.
//!
//! Everything here goes through [`crate::transport::exec_script`], the same
//! funnel `apply` uses, so `exec` inherits the transport selection (local,
//! SSH, container, pepita) and the I8 bashrs gate rather than re-implementing
//! any of it.

use crate::core::types::{ForjarConfig, Machine};
use std::path::Path;

/// Characters that need no quoting in any POSIX shell.
fn is_bare_safe(c: char) -> bool {
    c.is_ascii_alphanumeric() || "._-/=:@,+".contains(c)
}

/// Quote ONE word so the remote shell sees it byte-for-byte.
///
/// The single-quote case is the one that matters: `'` cannot appear inside a
/// single-quoted string, so it is closed, escaped and reopened
/// (`'\''`). `$`, backticks, spaces and newlines are inert inside single
/// quotes, which is why nothing else needs special handling — an argument like
/// `$HOME` is passed as the literal five characters, not expanded here.
pub(crate) fn shell_quote(word: &str) -> String {
    if !word.is_empty() && word.chars().all(is_bare_safe) {
        return word.to_string();
    }
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// Join the operator's argv into one command line for the remote shell.
pub(crate) fn shell_join(words: &[String]) -> String {
    words
        .iter()
        .map(|w| shell_quote(w))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Look a machine up by name, or say what the config actually declares.
///
/// A typo'd machine name is the most common way these verbs are misused, and
/// "machine not found" alone forces the operator to go read the YAML. The file
/// and the inventory are both in the message because both are what they would
/// have looked up.
pub(crate) fn resolve_machine<'a>(
    config: &'a ForjarConfig,
    name: &str,
    file: &Path,
) -> Result<&'a Machine, String> {
    config.machines.get(name).ok_or_else(|| {
        let known: Vec<&str> = config.machines.keys().map(String::as_str).collect();
        format!(
            "machine '{name}' is not in {} (machines: {})",
            file.display(),
            known.join(", ")
        )
    })
}

/// The hint the ticket asks for by name.
///
/// A `Permission denied` is precisely the failure whose diagnosis needs the
/// owner, the mode and the identity forjar connects as — the three facts
/// `doctor --machine` reports. Pointing at it here is what turns a dead end
/// into a next step.
pub(crate) fn permission_hint(machine: &str, stderr: &str) -> Option<String> {
    if !stderr.contains("Permission denied") {
        return None;
    }
    Some(format!(
        "hint: forjar doctor --machine {machine} reports the owner and mode of the destination and the identity forjar connects as"
    ))
}

/// The `--json` record of one run. Every field is a MEASUREMENT.
pub(crate) fn exec_json(machine: &str, out: &crate::transport::ExecOutput) -> String {
    serde_json::json!({
        "machine": machine,
        "exit_code": out.exit_code,
        "stdout": out.stdout,
        "stderr": out.stderr,
    })
    .to_string()
}

/// Forward the remote streams to ours, unaltered.
fn forward(machine: &str, out: &crate::transport::ExecOutput, json: bool) {
    if json {
        println!("{}", exec_json(machine, out));
        return;
    }
    print!("{}", out.stdout);
    eprint!("{}", out.stderr);
    if let Some(hint) = permission_hint(machine, &out.stderr) {
        eprintln!("{hint}");
    }
}

/// Leave with the REMOTE process's exit code.
///
/// forjar's own taxonomy ([`crate::core::error::ErrorClass`]) describes
/// forjar's failures; this verb reports someone else's. `exec … -- false` must
/// exit 1 because `false` did, exactly as `ssh host false` does, so the code
/// is written to the process here rather than mapped through a class that
/// would flatten every remote code onto 1.
fn exit_with(code: i32) -> Result<(), String> {
    if code == 0 {
        return Ok(());
    }
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(code);
}

/// #446: run one command on one machine.
pub(crate) fn cmd_exec(
    file: &Path,
    machine_name: &str,
    command: &[String],
    json: bool,
) -> Result<(), String> {
    if command.is_empty() {
        return Err(
            "no command given — use `forjar exec <machine> -- <command> [args...]`".to_string(),
        );
    }
    let config = super::helpers::parse_and_validate(file)?;
    let machine = resolve_machine(&config, machine_name, file)?;
    let script = shell_join(command);
    let out = crate::transport::exec_script(machine, &script)?;
    forward(machine_name, &out, json);
    exit_with(out.exit_code)
}
