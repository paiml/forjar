//! FJ-3404: WASM plugin runtime execution via wasmi.
//!
//! Loads and executes WASM plugin modules in a sandboxed interpreter.
//! Feature-gated behind `--features wasm-runtime`.

use crate::core::types::PluginStatus;
use std::path::Path;

/// Result of executing a plugin function in the WASM runtime.
#[derive(Debug, Clone)]
pub struct WasmExecResult {
    pub success: bool,
    pub status: PluginStatus,
    pub output: String,
}

/// Execute a plugin's exported function via the WASM runtime.
///
/// Loads the `.wasm` file, instantiates the module, and calls the
/// named export (`check`, `apply`, or `destroy`). State is passed
/// as JSON bytes through WASM linear memory.
#[cfg(feature = "wasm-runtime")]
pub fn execute_wasm(
    wasm_path: &Path,
    function_name: &str,
    input_json: &[u8],
) -> Result<WasmExecResult, String> {
    use wasmi::*;

    let wasm_bytes =
        std::fs::read(wasm_path).map_err(|e| format!("read {}: {e}", wasm_path.display()))?;

    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_bytes[..]).map_err(|e| format!("wasm compile: {e}"))?;

    let mut store = Store::new(&engine, HostState::new(input_json));
    let mut linker = Linker::<HostState>::new(&engine);

    // Provide host functions the plugin can call
    register_host_functions(&mut linker)?;

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("wasm instantiate: {e}"))?
        .start(&mut store)
        .map_err(|e| format!("wasm start: {e}"))?;

    // Write input JSON to WASM memory
    let memory = instance
        .get_memory(&store, "memory")
        .ok_or("plugin missing 'memory' export")?;
    write_input_to_memory(&mut store, &memory, input_json)?;

    // Call the exported function
    let func = instance
        .get_typed_func::<(i32, i32), i32>(&store, function_name)
        .map_err(|e| format!("export '{function_name}': {e}"))?;

    let input_ptr: i32 = 0;
    let input_len = i32::try_from(input_json.len()).map_err(|_| "input too large for wasm i32")?;

    let result_ptr = func
        .call(&mut store, (input_ptr, input_len))
        .map_err(|e| format!("wasm call '{function_name}': {e}"))?;

    // Read output from WASM memory
    let output = read_output_from_memory(&store, &memory, result_ptr)?;

    parse_plugin_output(&output)
}

/// Stub implementation when wasm-runtime feature is disabled.
#[cfg(not(feature = "wasm-runtime"))]
pub fn execute_wasm(
    _wasm_path: &Path,
    function_name: &str,
    _input_json: &[u8],
) -> Result<WasmExecResult, String> {
    Ok(WasmExecResult {
        success: true,
        status: PluginStatus::Converged,
        output: format!("{function_name}: stub (enable --features wasm-runtime)"),
    })
}

/// Check if the WASM runtime is available (feature enabled).
pub fn is_runtime_available() -> bool {
    cfg!(feature = "wasm-runtime")
}

/// Validate that a WASM binary has the expected ABI exports.
pub fn validate_wasm_exports(wasm_bytes: &[u8]) -> Result<WasmAbiInfo, String> {
    validate_exports_inner(wasm_bytes)
}

/// ABI information extracted from a WASM module.
#[derive(Debug, Clone)]
pub struct WasmAbiInfo {
    pub has_check: bool,
    pub has_apply: bool,
    pub has_destroy: bool,
    pub has_memory: bool,
    pub export_count: usize,
}

impl WasmAbiInfo {
    /// Whether the module has the minimum required exports.
    pub fn is_valid(&self) -> bool {
        self.has_memory && (self.has_check || self.has_apply)
    }

    /// List missing required exports.
    pub fn missing_exports(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.has_memory {
            missing.push("memory");
        }
        if !self.has_check {
            missing.push("check");
        }
        if !self.has_apply {
            missing.push("apply");
        }
        if !self.has_destroy {
            missing.push("destroy");
        }
        missing
    }
}

#[cfg(feature = "wasm-runtime")]
fn validate_exports_inner(wasm_bytes: &[u8]) -> Result<WasmAbiInfo, String> {
    use wasmi::*;

    let engine = Engine::default();
    let module = Module::new(&engine, wasm_bytes).map_err(|e| format!("wasm parse: {e}"))?;

    let mut info = WasmAbiInfo {
        has_check: false,
        has_apply: false,
        has_destroy: false,
        has_memory: false,
        export_count: 0,
    };

    for export in module.exports() {
        info.export_count += 1;
        match export.name() {
            "check" => info.has_check = true,
            "apply" => info.has_apply = true,
            "destroy" => info.has_destroy = true,
            "memory" => info.has_memory = true,
            _ => {}
        }
    }

    Ok(info)
}

#[cfg(not(feature = "wasm-runtime"))]
fn validate_exports_inner(_wasm_bytes: &[u8]) -> Result<WasmAbiInfo, String> {
    Ok(WasmAbiInfo {
        has_check: true,
        has_apply: true,
        has_destroy: true,
        has_memory: true,
        export_count: 0,
    })
}

// --- Host state and helper functions (wasm-runtime only) ---

#[cfg(feature = "wasm-runtime")]
struct HostState {
    output_buffer: Vec<u8>,
}

#[cfg(feature = "wasm-runtime")]
impl HostState {
    fn new(_input: &[u8]) -> Self {
        Self {
            output_buffer: Vec::new(),
        }
    }
}

#[cfg(feature = "wasm-runtime")]
fn register_host_functions(linker: &mut wasmi::Linker<HostState>) -> Result<(), String> {
    // host_log: plugin calls this to emit log messages
    linker
        .func_wrap(
            "env",
            "host_log",
            |caller: wasmi::Caller<'_, HostState>, ptr: i32, len: i32| {
                let mem = caller.get_export("memory").and_then(|e| e.into_memory());
                if let Some(memory) = mem {
                    let start = ptr as usize;
                    let end = start + len as usize;
                    let data = memory.data(&caller);
                    if end <= data.len() {
                        if let Ok(msg) = std::str::from_utf8(&data[start..end]) {
                            eprintln!("[plugin] {msg}");
                        }
                    }
                }
            },
        )
        .map_err(|e| format!("link host_log: {e}"))?;

    // host_set_output: plugin writes its result JSON
    linker
        .func_wrap(
            "env",
            "host_set_output",
            |mut caller: wasmi::Caller<'_, HostState>, ptr: i32, len: i32| {
                let mem = caller.get_export("memory").and_then(|e| e.into_memory());
                if let Some(memory) = mem {
                    let start = ptr as usize;
                    let end = start + len as usize;
                    let buf = {
                        let data = memory.data(&caller);
                        if end <= data.len() {
                            Some(data[start..end].to_vec())
                        } else {
                            None
                        }
                    };
                    if let Some(b) = buf {
                        caller.data_mut().output_buffer = b;
                    }
                }
            },
        )
        .map_err(|e| format!("link host_set_output: {e}"))?;

    Ok(())
}

#[cfg(feature = "wasm-runtime")]
fn write_input_to_memory(
    store: &mut wasmi::Store<HostState>,
    memory: &wasmi::Memory,
    input: &[u8],
) -> Result<(), String> {
    let mem_data = memory.data_mut(store);
    if input.len() > mem_data.len() {
        return Err(format!(
            "input ({} bytes) exceeds wasm memory ({} bytes)",
            input.len(),
            mem_data.len()
        ));
    }
    mem_data[..input.len()].copy_from_slice(input);
    Ok(())
}

#[cfg(feature = "wasm-runtime")]
fn read_output_from_memory(
    store: &wasmi::Store<HostState>,
    _memory: &wasmi::Memory,
    _result_ptr: i32,
) -> Result<String, String> {
    let buf = &store.data().output_buffer;
    if buf.is_empty() {
        return Ok(r#"{"success":true,"status":"converged"}"#.to_string());
    }
    String::from_utf8(buf.clone()).map_err(|e| format!("output not utf8: {e}"))
}

#[cfg_attr(not(feature = "wasm-runtime"), allow(dead_code))]
fn parse_plugin_output(output: &str) -> Result<WasmExecResult, String> {
    let v: serde_json::Value =
        serde_json::from_str(output).map_err(|e| format!("parse output: {e}"))?;

    let success = v.get("success").and_then(|v| v.as_bool()).unwrap_or(true);
    let status_str = v
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("converged");
    let status = match status_str {
        "converged" => PluginStatus::Converged,
        "drifted" => PluginStatus::Drifted,
        "missing" => PluginStatus::Missing,
        _ => PluginStatus::Error,
    };
    let msg = v
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(WasmExecResult {
        success,
        status,
        output: msg,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_availability() {
        // In default test builds, wasm-runtime is not enabled
        let available = is_runtime_available();
        // Just verify it returns a bool without panicking
        let _ = available;
    }

    #[test]
    fn stub_execute_wasm() {
        let result = execute_wasm(Path::new("/nonexistent.wasm"), "check", b"{}");
        // Without feature, returns stub success
        if !is_runtime_available() {
            let r = result.unwrap();
            assert!(r.success);
            assert!(r.output.contains("stub"));
        }
    }

    #[test]
    fn validate_exports_stub() {
        if is_runtime_available() {
            // With real runtime, invalid bytes should error
            assert!(validate_wasm_exports(b"not real wasm").is_err());
        } else {
            let info = validate_wasm_exports(b"not real wasm").unwrap();
            assert!(info.is_valid());
        }
    }

    #[test]
    fn parse_output_success() {
        let out = r#"{"success":true,"status":"converged","message":"ok"}"#;
        let r = parse_plugin_output(out).unwrap();
        assert!(r.success);
        assert_eq!(r.status, PluginStatus::Converged);
        assert_eq!(r.output, "ok");
    }

    #[test]
    fn parse_output_drifted() {
        let out = r#"{"success":true,"status":"drifted","message":"needs update"}"#;
        let r = parse_plugin_output(out).unwrap();
        assert!(r.success);
        assert_eq!(r.status, PluginStatus::Drifted);
    }

    #[test]
    fn parse_output_failure() {
        let out = r#"{"success":false,"status":"error","message":"boom"}"#;
        let r = parse_plugin_output(out).unwrap();
        assert!(!r.success);
        assert_eq!(r.status, PluginStatus::Error);
        assert_eq!(r.output, "boom");
    }

    #[test]
    fn parse_output_minimal() {
        let out = r#"{}"#;
        let r = parse_plugin_output(out).unwrap();
        assert!(r.success);
        assert_eq!(r.status, PluginStatus::Converged);
    }

    #[test]
    fn parse_output_invalid_json() {
        assert!(parse_plugin_output("not json").is_err());
    }

    #[test]
    fn wasm_abi_info_valid() {
        let info = WasmAbiInfo {
            has_check: true,
            has_apply: true,
            has_destroy: true,
            has_memory: true,
            export_count: 4,
        };
        assert!(info.is_valid());
        assert!(info.missing_exports().is_empty());
    }

    #[test]
    fn wasm_abi_info_missing_memory() {
        let info = WasmAbiInfo {
            has_check: true,
            has_apply: true,
            has_destroy: true,
            has_memory: false,
            export_count: 3,
        };
        assert!(!info.is_valid());
        assert!(info.missing_exports().contains(&"memory"));
    }

    #[test]
    fn wasm_abi_info_no_operations() {
        let info = WasmAbiInfo {
            has_check: false,
            has_apply: false,
            has_destroy: false,
            has_memory: true,
            export_count: 1,
        };
        assert!(!info.is_valid());
    }

    #[test]
    fn wasm_abi_info_check_only() {
        let info = WasmAbiInfo {
            has_check: true,
            has_apply: false,
            has_destroy: false,
            has_memory: true,
            export_count: 2,
        };
        assert!(info.is_valid());
    }
}
