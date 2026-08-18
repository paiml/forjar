//! The single dispatch point every transport goes through.
//!
//! # Why this runs the binary rather than calling `cli::dispatch` in-process
//!
//! forjar's verbs report by printing. `cli::dispatch` returns `Result<(),
//! String>` and writes its actual answer to stdout, so an in-process caller
//! would have to capture file descriptor 1 — which this crate cannot do,
//! because `unsafe_code = "forbid"` and every stdout-capture crate needs it.
//!
//! Re-executing the current binary is not a workaround for that; it is the
//! stronger design. paiml/rmedia#247 shipped a four-way transport-parity suite
//! that stayed green for the whole period its MCP and HTTP servers had no
//! caller from `main`: the transports agreed with each other perfectly and were
//! unreachable from the process entry point. Here a transport call *is* a
//! process invocation of the shipped binary, so "the transport agrees with the
//! CLI" and "the transport is reachable" are the same fact, and neither can be
//! true in a build where the CLI is broken.

use super::derive;
use super::error::VerbError;
use super::spec::VerbSpec;
use super::{argv, validate};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

/// Where and how a verb runs.
#[derive(Debug, Clone)]
pub struct VerbCtx {
    /// The forjar binary to invoke. Defaults to the running executable.
    pub binary: PathBuf,
    /// The working directory for the invocation.
    pub cwd: PathBuf,
}

impl VerbCtx {
    /// A context that re-executes the current binary in the current directory.
    ///
    /// # Errors
    ///
    /// [`VerbError::Spawn`] if the current executable or directory cannot be
    /// determined.
    pub fn current() -> Result<Self, VerbError> {
        Ok(VerbCtx {
            binary: std::env::current_exe()
                .map_err(|e| VerbError::Spawn(format!("current_exe: {e}")))?,
            cwd: std::env::current_dir()
                .map_err(|e| VerbError::Spawn(format!("current_dir: {e}")))?,
        })
    }

    /// Point the context at an explicit binary and directory.
    #[must_use]
    pub fn new(binary: PathBuf, cwd: PathBuf) -> Self {
        VerbCtx { binary, cwd }
    }
}

/// Invoke `verb` with `params`.
///
/// Validates the params against the verb's derived schema *before* running
/// anything, so a malformed call never reaches a host.
///
/// # Errors
///
/// - [`VerbError::UnknownVerb`] — no such verb.
/// - [`VerbError::NotInvocable`] — the verb serves a transport.
/// - [`VerbError::InvalidParams`] — params failed validation.
/// - [`VerbError::Spawn`] — the binary could not be run.
///
/// A verb that runs and exits non-zero is **not** an error here: the envelope
/// carries `ok: false` and the exit code, because the exchange succeeded and
/// the caller needs the output.
pub fn dispatch(verb: &str, params: &Value, ctx: &VerbCtx) -> Result<Value, VerbError> {
    let spec = derive::find(verb).ok_or_else(|| VerbError::UnknownVerb(verb.to_string()))?;
    dispatch_spec(spec, params, ctx)
}

/// [`dispatch`] against an already-resolved spec, avoiding a second derivation.
///
/// # Errors
///
/// As [`dispatch`].
pub fn dispatch_spec(spec: &VerbSpec, params: &Value, ctx: &VerbCtx) -> Result<Value, VerbError> {
    if !spec.effects.is_invocable() {
        return Err(VerbError::NotInvocable(spec.name.clone()));
    }
    validate::check(spec, params)?;
    let args = argv::build(spec, params)?;

    let out = Command::new(&ctx.binary)
        .args(&args)
        .current_dir(&ctx.cwd)
        .output()
        .map_err(|e| VerbError::Spawn(format!("{}: {e}", ctx.binary.display())))?;

    Ok(envelope(&spec.name, &out))
}

/// Build the result envelope from a finished process.
fn envelope(verb: &str, out: &std::process::Output) -> Value {
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let code = out.status.code().unwrap_or(-1);
    let mut env = json!({
        "verb": verb,
        "ok": code == 0,
        "exit_code": code,
        "stdout": stdout,
        "stderr": stderr,
    });
    // `--json` verbs already emit a document; surface it parsed so a client
    // need not double-decode. Absent when stdout is prose, which is honest.
    if let Ok(v) = serde_json::from_str::<Value>(env["stdout"].as_str().unwrap_or("")) {
        if v.is_object() || v.is_array() {
            env["json"] = v;
        }
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> VerbCtx {
        VerbCtx::new(PathBuf::from("/nonexistent/forjar"), PathBuf::from("."))
    }

    #[test]
    fn an_unknown_verb_is_rejected_before_anything_runs() {
        let e = dispatch("no-such-verb", &json!({}), &ctx()).unwrap_err();
        assert_eq!(e.kind(), "unknown_verb");
        assert_eq!(e.http_status(), 404);
    }

    #[test]
    fn transport_verbs_are_refused() {
        // Serving `mcp` from inside the MCP server is unbounded recursion.
        for v in ["mcp", "serve", "lsp"] {
            let e = dispatch(v, &json!({}), &ctx()).unwrap_err();
            assert_eq!(e.kind(), "not_invocable", "{v}");
            assert_eq!(e.http_status(), 403, "{v}");
        }
    }

    #[test]
    fn params_are_validated_before_the_binary_is_touched() {
        // The context points at a path that does not exist. If validation ran
        // after the spawn, this would be a Spawn error, not InvalidParams.
        let e = dispatch("plan", &json!({"nonexistent_param": 1}), &ctx()).unwrap_err();
        assert_eq!(e.kind(), "invalid_params");
    }

    #[test]
    fn a_missing_binary_is_a_spawn_error() {
        let e = dispatch("plan", &json!({}), &ctx()).unwrap_err();
        assert_eq!(e.kind(), "spawn");
        assert_eq!(e.http_status(), 500);
    }

    #[test]
    fn the_envelope_carries_the_five_mandatory_fields() {
        let out = std::process::Output {
            status: std::process::ExitStatus::default(),
            stdout: b"hello".to_vec(),
            stderr: Vec::new(),
        };
        let e = envelope("plan", &out);
        for k in ["verb", "ok", "exit_code", "stdout", "stderr"] {
            assert!(e.get(k).is_some(), "envelope missing {k}");
        }
        assert_eq!(e["verb"], "plan");
        assert_eq!(e["stdout"], "hello");
        assert!(e.get("json").is_none(), "prose stdout must not parse");
    }

    #[test]
    fn json_stdout_is_surfaced_parsed() {
        let out = std::process::Output {
            status: std::process::ExitStatus::default(),
            stdout: br#"{"changes": 3}"#.to_vec(),
            stderr: Vec::new(),
        };
        let e = envelope("plan", &out);
        assert_eq!(e["json"]["changes"], 3);
        // The raw text stays, so a client that wants bytes still has them.
        assert_eq!(e["stdout"], r#"{"changes": 3}"#);
    }

    #[test]
    fn a_bare_scalar_on_stdout_is_not_treated_as_a_json_document() {
        // `serde_json` happily parses `42` and `"hi"`. Promoting those to a
        // `json` field would make a verb that prints a number look structured.
        for raw in [&b"42"[..], &b"\"hi\""[..], &b"true"[..]] {
            let out = std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: raw.to_vec(),
                stderr: Vec::new(),
            };
            assert!(
                envelope("x", &out).get("json").is_none(),
                "scalar {raw:?} must not become a json document"
            );
        }
    }
}
