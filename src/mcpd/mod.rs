//! `forjar mcp` — the verb registry as an MCP server over stdio.
//!
//! # What replaced what
//!
//! forjar 1.x served nine hand-written MCP tools from [`crate::mcp`], each with
//! its own handler duplicating logic the CLI already had. `src/mcp/tests_parity.rs`
//! records the result: two defects where the MCP tool and the CLI command of
//! the same name disagreed about the same project, both found only by driving
//! the published binary over stdio.
//!
//! This module serves all ~155 invocable verbs and has no handler for any of
//! them. A `tools/call` becomes an argv and a process invocation of the shipped
//! binary, so the MCP answer to `plan` is by construction the answer `forjar
//! plan` gives — there is no second implementation to disagree with.
//!
//! The 1.x server is still reachable as `forjar mcp --legacy` for one release.

pub mod proto;

use crate::verb::VerbCtx;
use serde_json::Value;
use std::io::{BufRead, Write};

/// Serve MCP over stdio until end of input.
///
/// Framing is newline-delimited JSON, one request per line, as MCP's stdio
/// transport specifies.
///
/// # Errors
///
/// A string describing a fatal I/O failure. A malformed *request* is not fatal:
/// it is answered with a JSON-RPC parse error and the loop continues.
pub fn serve_stdio() -> Result<(), String> {
    // Fail before the first request rather than during it.
    let _ = crate::verb::registry().len();
    let ctx = VerbCtx::current().map_err(|e| e.to_string())?;
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    run(stdin.lock(), &mut stdout, &ctx)
}

/// The protocol loop, over any reader and writer.
///
/// # Errors
///
/// A string describing a fatal I/O failure.
pub fn run<R: BufRead, W: Write>(reader: R, writer: &mut W, ctx: &VerbCtx) -> Result<(), String> {
    for line in reader.lines() {
        let line = line.map_err(|e| format!("stdin: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(req) => proto::handle(&req, ctx),
            Err(e) => Some(serde_json::json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": { "code": -32700, "message": format!("parse error: {e}") }
            })),
        };
        if let Some(r) = response {
            writeln!(writer, "{r}").map_err(|e| format!("stdout: {e}"))?;
            writer.flush().map_err(|e| format!("flush: {e}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn ctx() -> VerbCtx {
        VerbCtx::new(PathBuf::from("/nonexistent/forjar"), PathBuf::from("."))
    }

    fn exchange(input: &str) -> Vec<Value> {
        let mut out = Vec::new();
        run(std::io::Cursor::new(input), &mut out, &ctx()).expect("loop must not fail");
        String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).expect("each line is one JSON value"))
            .collect()
    }

    #[test]
    fn one_line_in_one_line_out() {
        let r = exchange("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["id"], 1);
    }

    #[test]
    fn responses_are_newline_delimited_and_individually_parseable() {
        // A client reads one line at a time; a pretty-printed response spanning
        // several lines would deadlock it.
        let r = exchange(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n\
             {\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n",
        );
        assert_eq!(r.len(), 2);
        assert_eq!(r[0]["id"], 1);
        assert_eq!(r[1]["id"], 2);
    }

    #[test]
    fn a_malformed_line_is_a_parse_error_and_does_not_end_the_session() {
        let r = exchange("not json\n{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"ping\"}\n");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0]["error"]["code"], -32700);
        assert_eq!(r[1]["id"], 9, "the session must survive a bad line");
    }

    #[test]
    fn blank_lines_are_skipped_without_a_response() {
        let r = exchange("\n\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n\n");
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn notifications_produce_no_output_at_all() {
        let mut out = Vec::new();
        run(
            std::io::Cursor::new("{\"jsonrpc\":\"2.0\",\"method\":\"tools/list\"}\n"),
            &mut out,
            &ctx(),
        )
        .unwrap();
        assert!(out.is_empty(), "a notification must not be answered");
    }

    #[test]
    fn empty_input_is_a_clean_exit() {
        let mut out = Vec::new();
        assert!(run(std::io::Cursor::new(""), &mut out, &ctx()).is_ok());
        assert!(out.is_empty());
    }

    #[test]
    fn tools_list_over_the_loop_carries_the_whole_surface() {
        let r = exchange("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n");
        let n = r[0]["result"]["tools"].as_array().unwrap().len();
        assert_eq!(
            n,
            crate::verb::registry()
                .iter()
                .filter(|v| v.effects.is_invocable())
                .count()
        );
    }
}
