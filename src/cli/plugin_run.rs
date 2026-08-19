//! FJ-3404 / Refs #210: `forjar plugin run` — execute a plugin operation.
//!
//! Split out of `plugin.rs` so neither file carries the whole subcommand set.

use super::plugin::validate_plugin_name;
use std::path::Path;

/// Refs #210: refuse a run that cannot execute, instead of faking one.
///
/// Without the `wasm-runtime` feature the "runtime" is a stub that returns a
/// canned success, so `plugin run … --operation apply` reported
/// `success: true` / `status: Converged` for a plugin it never loaded. That is
/// a green certifying convergence that never happened. `state-encrypt` already
/// refuses when built without `encryption`; this does the same, and still
/// emits parseable JSON under `--json` so a machine consumer sees the reason.
pub(crate) fn refuse_stub_runtime(name: &str, operation: &str, json: bool) -> String {
    let message = format!(
        "plugin {operation} not executed: this forjar was built without the \
         `wasm-runtime` feature, so no WASM module can run. \
         Rebuild with `cargo build --features wasm-runtime`."
    );
    if json {
        println!(
            "{}",
            serde_json::json!({
                "plugin": name,
                "operation": operation,
                "success": false,
                "message": message,
                "status": "Unsupported",
                "runtime": "stub",
            })
        );
    }
    message
}

/// Render a completed dispatch in text or JSON.
fn print_run_result(result: &crate::core::plugin_dispatch::PluginDispatchResult, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "plugin": result.plugin_name,
                "operation": result.operation,
                "success": result.success,
                "message": result.message,
                "status": format!("{:?}", result.status),
                "runtime": "wasmi",
            })
        );
        return;
    }
    println!("Plugin:    {}", result.plugin_name);
    println!("Operation: {}", result.operation);
    println!("Runtime:   wasmi");
    println!("Status:    {:?}", result.status);
    println!("Success:   {}", result.success);
    if !result.message.is_empty() {
        println!("Message:   {}", result.message);
    }
}

/// Map an operation name to its dispatcher.
type DispatchFn =
    fn(&Path, &str, &serde_json::Value) -> crate::core::plugin_dispatch::PluginDispatchResult;

fn dispatcher_for(operation: &str) -> Result<DispatchFn, String> {
    match operation {
        "check" => Ok(crate::core::plugin_dispatch::dispatch_check),
        "apply" => Ok(crate::core::plugin_dispatch::dispatch_apply),
        "destroy" => Ok(crate::core::plugin_dispatch::dispatch_destroy),
        _ => Err(format!(
            "invalid operation '{operation}': use check, apply, or destroy"
        )),
    }
}

/// FJ-3404: Execute a plugin operation via the WASM runtime.
pub(crate) fn cmd_plugin_run(
    name: &str,
    operation: &str,
    plugin_dir: &Path,
    config: &str,
    json: bool,
) -> Result<(), String> {
    let dispatch_fn = dispatcher_for(operation)?;
    validate_plugin_name(name)?;
    let config_json: serde_json::Value =
        serde_json::from_str(config).map_err(|e| format!("parse config: {e}"))?;

    if !crate::core::plugin_runtime::is_runtime_available() {
        return Err(refuse_stub_runtime(name, operation, json));
    }

    let resolved = crate::core::plugin_dispatch::dispatch_check(plugin_dir, name, &config_json);
    if !resolved.success {
        return Err(format!("plugin resolve failed: {}", resolved.message));
    }
    let result = dispatch_fn(plugin_dir, name, &config_json);
    print_run_result(&result, json);
    if result.success {
        Ok(())
    } else {
        Err(format!("plugin {operation} failed: {}", result.message))
    }
}
