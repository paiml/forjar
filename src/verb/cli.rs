//! `forjar verb` — the unified surface, rendered as a shipped subcommand.
//!
//! This exists so the derived tree has a real user. rmedia proved parity across
//! four surfaces while `main.rs` routed only two of them, and every test passed
//! throughout: a parity suite compares transports to each other, so it cannot
//! see that none of them is reachable. Shipping the tree turns that failure
//! into `forjar verb list` printing nothing, which a person notices.

use clap::{Args, Subcommand};

use super::registry;

/// Arguments for `forjar verb`.
#[derive(Args, Debug)]
pub struct VerbArgs {
    #[command(subcommand)]
    pub cmd: VerbCmd,
}

/// The unified verb surface.
#[derive(Subcommand, Debug)]
pub enum VerbCmd {
    /// List every verb on the unified surface
    List {
        /// Emit JSON instead of one name per line
        #[arg(long)]
        json: bool,
    },
    /// Print a verb's input and output JSON Schema
    Schema {
        /// Verb name, e.g. `plan`
        name: String,
    },
    /// Serve the verb surface over HTTP
    Serve {
        /// TCP port to listen on
        #[arg(long, default_value = "8737")]
        port: u16,
        /// Interface to bind. The surface has NO authentication, so anything
        /// other than loopback is an explicit choice.
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
    },
    /// Invoke a verb with JSON parameters
    Call {
        /// Verb name, e.g. `validate`
        name: String,
        /// Parameters as a JSON object
        #[arg(long, default_value = "{}")]
        json: String,
    },
}

/// Dispatch `forjar verb`.
pub fn dispatch_verb(cmd: VerbCmd) -> Result<(), String> {
    match cmd {
        VerbCmd::List { json } => list(json),
        VerbCmd::Schema { name } => schema(&name),
        VerbCmd::Call { name, json } => call(&name, &json),
        VerbCmd::Serve { port, bind } => super::http::serve(super::http::HttpConfig { bind, port }),
    }
}

fn list(as_json: bool) -> Result<(), String> {
    let verbs = registry::verbs();
    if as_json {
        let rows: Vec<_> = verbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "name": v.name,
                    "mcp_name": v.mcp_name(),
                    "description": v.description,
                    // Derived from `effects`, never stated separately — this is
                    // the value MCP publishes as readOnlyHint.
                    "read_only": v.effects.read_only(),
                    "timeout_ms": v.timeout_ms,
                })
            })
            .collect();
        let out = serde_json::to_string_pretty(&serde_json::json!({ "verbs": rows }))
            .map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        for v in &verbs {
            println!("{}", v.name);
        }
    }
    Ok(())
}

fn schema(name: &str) -> Result<(), String> {
    let v = registry::find(name).ok_or_else(|| unknown(name))?;
    let out = serde_json::to_string_pretty(&serde_json::json!({
        "name": v.name,
        "input_schema": (v.input_schema)(),
        "output_schema": (v.output_schema)(),
    }))
    .map_err(|e| format!("JSON error: {e}"))?;
    println!("{out}");
    Ok(())
}

fn call(name: &str, params: &str) -> Result<(), String> {
    let v = registry::find(name).ok_or_else(|| unknown(name))?;
    let value: serde_json::Value =
        serde_json::from_str(params).map_err(|e| format!("--json is not valid JSON: {e}"))?;
    let out = (v.invoke)(value)?;
    // The SAME renderer the HTTP transport uses. If these were two
    // `to_string_pretty` calls that happened to agree today, the fidelity gate
    // would be guarding a coincidence.
    println!("{}", super::render_result(&out));
    Ok(())
}

/// Name the alternatives. An unknown verb is the most likely way someone meets
/// this surface, so the error should teach the surface rather than just refuse.
fn unknown(name: &str) -> String {
    let names: Vec<&str> = registry::verbs().iter().map(|v| v.name).collect();
    format!(
        "unknown verb `{name}` — the unified surface is: {}",
        names.join(", ")
    )
}
