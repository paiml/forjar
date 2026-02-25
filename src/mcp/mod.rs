//! FJ-063: MCP integration via pforge.
//!
//! Exposes forjar operations as MCP tools: validate, plan, drift,
//! lint, graph, show, status. Uses pforge-runtime HandlerRegistry for
//! O(1) dispatch and pforge McpServer for protocol handling.

use pforge_config::{ForgeConfig, ForgeMetadata, OptimizationLevel, ParamSchema, TransportType};
use pforge_runtime::{Handler, HandlerRegistry, McpServer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::core::{codegen, parser, planner, resolver, state};
use crate::tripwire::drift;

// ── Input / Output types ────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateInput {
    /// Path to forjar.yaml
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ValidateOutput {
    pub valid: bool,
    pub resource_count: usize,
    pub machine_count: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlanInput {
    /// Path to forjar.yaml
    pub path: String,
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to specific resource
    pub resource: Option<String>,
    /// Filter by tag
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PlanOutput {
    pub changes: Vec<PlannedChangeOutput>,
    pub to_create: u32,
    pub to_update: u32,
    pub to_destroy: u32,
    pub unchanged: u32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PlannedChangeOutput {
    pub resource_id: String,
    pub machine: String,
    pub action: String,
    pub description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DriftInput {
    /// Path to forjar.yaml
    pub path: String,
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to specific machine
    pub machine: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DriftOutput {
    pub drifted: bool,
    pub findings: Vec<DriftFindingOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DriftFindingOutput {
    pub resource: String,
    pub expected_hash: String,
    pub actual_hash: String,
    pub detail: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LintInput {
    /// Path to forjar.yaml
    pub path: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct LintOutput {
    pub warnings: Vec<String>,
    pub warning_count: usize,
    pub error_count: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphInput {
    /// Path to forjar.yaml
    pub path: String,
    /// Output format: "mermaid" (default) or "dot"
    pub format: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GraphOutput {
    pub graph: String,
    pub format: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowInput {
    /// Path to forjar.yaml
    pub path: String,
    /// Show specific resource only
    pub resource: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ShowOutput {
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatusInput {
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to specific machine
    pub machine: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StatusOutput {
    pub machines: Vec<MachineStatusOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MachineStatusOutput {
    pub name: String,
    pub resource_count: usize,
}

// ── Handlers ────────────────────────────────────────────────────────

pub struct ValidateHandler;
pub struct PlanHandler;
pub struct DriftHandler;
pub struct LintHandler;
pub struct GraphHandler;
pub struct ShowHandler;
pub struct StatusHandler;

#[async_trait::async_trait]
impl Handler for ValidateHandler {
    type Input = ValidateInput;
    type Output = ValidateOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);
        match parser::parse_and_validate(&path) {
            Ok(config) => Ok(ValidateOutput {
                valid: true,
                resource_count: config.resources.len(),
                machine_count: config.machines.len(),
                errors: vec![],
            }),
            Err(e) => Ok(ValidateOutput {
                valid: false,
                resource_count: 0,
                machine_count: 0,
                errors: e.lines().map(|l| l.to_string()).collect(),
            }),
        }
    }
}

#[async_trait::async_trait]
impl Handler for PlanHandler {
    type Input = PlanInput;
    type Output = PlanOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);
        let state_dir = PathBuf::from(input.state_dir.as_deref().unwrap_or("state"));

        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        let order =
            resolver::build_execution_order(&config).map_err(pforge_runtime::Error::Handler)?;

        // Load locks for all machines
        let mut locks = std::collections::HashMap::new();
        for machine_name in config.machines.keys() {
            if let Ok(Some(lock)) = state::load_lock(&state_dir, machine_name) {
                locks.insert(machine_name.clone(), lock);
            }
        }

        let exec_plan = planner::plan(&config, &order, &locks, input.tag.as_deref());

        let mut changes: Vec<PlannedChangeOutput> = exec_plan
            .changes
            .iter()
            .map(|c| PlannedChangeOutput {
                resource_id: c.resource_id.clone(),
                machine: c.machine.clone(),
                action: c.action.to_string(),
                description: c.description.clone(),
            })
            .collect();

        // Apply resource filter if specified
        if let Some(ref r) = input.resource {
            changes.retain(|c| c.resource_id == *r);
        }

        Ok(PlanOutput {
            to_create: exec_plan.to_create,
            to_update: exec_plan.to_update,
            to_destroy: exec_plan.to_destroy,
            unchanged: exec_plan.unchanged,
            changes,
        })
    }
}

#[async_trait::async_trait]
impl Handler for DriftHandler {
    type Input = DriftInput;
    type Output = DriftOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);
        let state_dir = PathBuf::from(input.state_dir.as_deref().unwrap_or("state"));

        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        let mut findings = Vec::new();

        for machine_name in config.machines.keys() {
            if let Some(ref m) = input.machine {
                if machine_name != m {
                    continue;
                }
            }

            if let Ok(Some(lock_data)) = state::load_lock(&state_dir, machine_name) {
                let drift_findings = drift::detect_drift(&lock_data);
                for f in drift_findings {
                    findings.push(DriftFindingOutput {
                        resource: f.resource_id.clone(),
                        expected_hash: f.expected_hash.clone(),
                        actual_hash: f.actual_hash.clone(),
                        detail: f.detail.clone(),
                    });
                }
            }
        }

        let drifted = !findings.is_empty();
        Ok(DriftOutput { drifted, findings })
    }
}

#[async_trait::async_trait]
impl Handler for LintHandler {
    type Input = LintInput;
    type Output = LintOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);

        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        let mut warnings = Vec::new();
        let mut error_count = 0;

        // Check for unused machines
        let mut used_machines = std::collections::HashSet::new();
        for resource in config.resources.values() {
            for m in resource.machine.to_vec() {
                used_machines.insert(m);
            }
        }
        for name in config.machines.keys() {
            if !used_machines.contains(name) {
                warnings.push(format!(
                    "Machine '{}' is defined but not referenced by any resource",
                    name
                ));
            }
        }

        // bashrs script lint
        for (id, resource) in &config.resources {
            for (kind, result) in [
                ("check", codegen::check_script(resource)),
                ("apply", codegen::apply_script(resource)),
                ("state_query", codegen::state_query_script(resource)),
            ] {
                if let Ok(script) = result {
                    let lint_result = crate::core::purifier::lint_script(&script);
                    for d in &lint_result.diagnostics {
                        use bashrs::linter::Severity;
                        match d.severity {
                            Severity::Error => {
                                error_count += 1;
                                warnings.push(format!(
                                    "[ERROR] {}.{}: [{}] {}",
                                    id, kind, d.code, d.message
                                ));
                            }
                            _ => {
                                warnings.push(format!(
                                    "[WARN] {}.{}: [{}] {}",
                                    id, kind, d.code, d.message
                                ));
                            }
                        }
                    }
                }
            }
        }

        let warning_count = warnings.len();
        Ok(LintOutput {
            warnings,
            warning_count,
            error_count,
        })
    }
}

#[async_trait::async_trait]
impl Handler for GraphHandler {
    type Input = GraphInput;
    type Output = GraphOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);
        let fmt = input.format.as_deref().unwrap_or("mermaid");

        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        let mut graph = String::new();
        match fmt {
            "dot" => {
                graph.push_str("digraph forjar {\n");
                graph.push_str("  rankdir=LR;\n");
                for (id, resource) in &config.resources {
                    let label = format!("{}\\n({})", id, resource.resource_type);
                    graph.push_str(&format!("  \"{}\" [label=\"{}\"];\n", id, label));
                    for dep in &resource.depends_on {
                        graph.push_str(&format!("  \"{}\" -> \"{}\";\n", dep, id));
                    }
                }
                graph.push_str("}\n");
            }
            _ => {
                graph.push_str("graph LR\n");
                for (id, resource) in &config.resources {
                    graph.push_str(&format!(
                        "  {}[\"{}\\n({})\"]\n",
                        id, id, resource.resource_type
                    ));
                    for dep in &resource.depends_on {
                        graph.push_str(&format!("  {} --> {}\n", dep, id));
                    }
                }
            }
        }

        Ok(GraphOutput {
            graph,
            format: fmt.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl Handler for ShowHandler {
    type Input = ShowInput;
    type Output = ShowOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);

        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        let config_value = if let Some(ref r) = input.resource {
            if let Some(resource) = config.resources.get(r) {
                // Resolve templates for this resource
                let resolved = resolver::resolve_resource_templates(
                    resource,
                    &config.params,
                    &config.machines,
                )
                .unwrap_or_else(|_| resource.clone());
                serde_json::to_value(&resolved)
                    .map_err(|e| pforge_runtime::Error::Handler(e.to_string()))?
            } else {
                return Err(pforge_runtime::Error::Handler(format!(
                    "Resource '{}' not found",
                    r
                )));
            }
        } else {
            serde_json::to_value(&config)
                .map_err(|e| pforge_runtime::Error::Handler(e.to_string()))?
        };

        Ok(ShowOutput {
            config: config_value,
        })
    }
}

#[async_trait::async_trait]
impl Handler for StatusHandler {
    type Input = StatusInput;
    type Output = StatusOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let state_dir = PathBuf::from(input.state_dir.as_deref().unwrap_or("state"));

        let mut machines = Vec::new();

        if state_dir.exists() {
            let entries = std::fs::read_dir(&state_dir)
                .map_err(|e| pforge_runtime::Error::Handler(e.to_string()))?;

            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    if let Some(ref m) = input.machine {
                        if &name != m {
                            continue;
                        }
                    }

                    let resource_count = state::load_lock(&state_dir, &name)
                        .ok()
                        .flatten()
                        .map(|l| l.resources.len())
                        .unwrap_or(0);

                    machines.push(MachineStatusOutput {
                        name,
                        resource_count,
                    });
                }
            }
        }

        Ok(StatusOutput { machines })
    }
}

// ── Registry + Server ───────────────────────────────────────────────

/// Build a HandlerRegistry with all forjar MCP tools.
pub fn build_registry() -> HandlerRegistry {
    let mut registry = HandlerRegistry::new();
    registry.register("forjar_validate", ValidateHandler);
    registry.register("forjar_plan", PlanHandler);
    registry.register("forjar_drift", DriftHandler);
    registry.register("forjar_lint", LintHandler);
    registry.register("forjar_graph", GraphHandler);
    registry.register("forjar_show", ShowHandler);
    registry.register("forjar_status", StatusHandler);
    registry
}

/// Build the ForgeConfig for the forjar MCP server.
fn build_forge_config() -> ForgeConfig {
    use pforge_config::{HandlerRef, ToolDef};
    use rustc_hash::FxHashMap;

    let empty_params = ParamSchema {
        fields: FxHashMap::default(),
    };

    ForgeConfig {
        forge: ForgeMetadata {
            name: "forjar-mcp".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            transport: TransportType::Stdio,
            optimization: OptimizationLevel::Release,
        },
        tools: vec![
            ToolDef::Native {
                name: "forjar_validate".to_string(),
                description: "Validate a forjar.yaml configuration file".to_string(),
                handler: HandlerRef {
                    path: "handlers::validate".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(30000),
            },
            ToolDef::Native {
                name: "forjar_plan".to_string(),
                description: "Show execution plan for infrastructure changes".to_string(),
                handler: HandlerRef {
                    path: "handlers::plan".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(60000),
            },
            ToolDef::Native {
                name: "forjar_drift".to_string(),
                description: "Detect configuration drift from desired state".to_string(),
                handler: HandlerRef {
                    path: "handlers::drift".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(60000),
            },
            ToolDef::Native {
                name: "forjar_lint".to_string(),
                description: "Lint forjar config for best practices and shell safety".to_string(),
                handler: HandlerRef {
                    path: "handlers::lint".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(30000),
            },
            ToolDef::Native {
                name: "forjar_graph".to_string(),
                description: "Generate resource dependency graph (Mermaid/DOT)".to_string(),
                handler: HandlerRef {
                    path: "handlers::graph".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(10000),
            },
            ToolDef::Native {
                name: "forjar_show".to_string(),
                description: "Show fully resolved config with templates expanded".to_string(),
                handler: HandlerRef {
                    path: "handlers::show".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(30000),
            },
            ToolDef::Native {
                name: "forjar_status".to_string(),
                description: "Show current state from lock files".to_string(),
                handler: HandlerRef {
                    path: "handlers::status".to_string(),
                    inline: None,
                },
                params: empty_params,
                timeout_ms: Some(10000),
            },
        ],
        resources: vec![],
        prompts: vec![],
        state: None,
    }
}

/// Start the forjar MCP server (stdio transport).
pub async fn serve() -> Result<(), String> {
    let config = build_forge_config();
    let server = McpServer::new(config);

    // Register forjar handlers into the server's registry
    let registry = server.registry();
    {
        let mut reg = registry.write().await;
        reg.register("forjar_validate", ValidateHandler);
        reg.register("forjar_plan", PlanHandler);
        reg.register("forjar_drift", DriftHandler);
        reg.register("forjar_lint", LintHandler);
        reg.register("forjar_graph", GraphHandler);
        reg.register("forjar_show", ShowHandler);
        reg.register("forjar_status", StatusHandler);
    }

    server
        .run()
        .await
        .map_err(|e| format!("MCP server error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj063_build_registry_has_all_tools() {
        let registry = build_registry();
        assert_eq!(registry.len(), 7);
        assert!(registry.has_handler("forjar_validate"));
        assert!(registry.has_handler("forjar_plan"));
        assert!(registry.has_handler("forjar_drift"));
        assert!(registry.has_handler("forjar_lint"));
        assert!(registry.has_handler("forjar_graph"));
        assert!(registry.has_handler("forjar_show"));
        assert!(registry.has_handler("forjar_status"));
    }

    #[test]
    fn test_fj063_build_registry_no_unknown_tools() {
        let registry = build_registry();
        assert!(!registry.has_handler("forjar_apply"));
        assert!(!registry.has_handler("nonexistent"));
    }

    #[test]
    fn test_fj063_forge_config_metadata() {
        let config = build_forge_config();
        assert_eq!(config.forge.name, "forjar-mcp");
        assert_eq!(config.tools.len(), 7);
    }

    #[test]
    fn test_fj063_forge_config_tool_names() {
        let config = build_forge_config();
        let names: Vec<&str> = config.tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"forjar_validate"));
        assert!(names.contains(&"forjar_plan"));
        assert!(names.contains(&"forjar_drift"));
        assert!(names.contains(&"forjar_lint"));
        assert!(names.contains(&"forjar_graph"));
        assert!(names.contains(&"forjar_show"));
        assert!(names.contains(&"forjar_status"));
    }

    #[tokio::test]
    async fn test_fj063_validate_handler_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-pkg:\n    type: package\n    provider: apt\n    packages: [curl]\n",
        )
        .unwrap();

        let handler = ValidateHandler;
        let input = ValidateInput {
            path: config_path.to_str().unwrap().to_string(),
        };
        let output = handler.handle(input).await.unwrap();
        assert!(output.valid);
        assert_eq!(output.resource_count, 1);
        assert_eq!(output.machine_count, 1);
        assert!(output.errors.is_empty());
    }

    #[tokio::test]
    async fn test_fj063_validate_handler_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, "resources: []").unwrap();

        let handler = ValidateHandler;
        let input = ValidateInput {
            path: config_path.to_str().unwrap().to_string(),
        };
        let output = handler.handle(input).await.unwrap();
        assert!(!output.valid);
        assert!(!output.errors.is_empty());
    }

    #[tokio::test]
    async fn test_fj063_validate_handler_missing_file() {
        let handler = ValidateHandler;
        let input = ValidateInput {
            path: "/nonexistent/forjar.yaml".to_string(),
        };
        let output = handler.handle(input).await.unwrap();
        assert!(!output.valid);
    }

    #[tokio::test]
    async fn test_fj063_graph_handler_mermaid() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  base-dir:\n    type: file\n    path: /opt/app\n    state: directory\n  app-config:\n    type: file\n    path: /opt/app/config.yml\n    content: \"key: value\"\n    depends_on: [base-dir]\n",
        )
        .unwrap();

        let handler = GraphHandler;
        let input = GraphInput {
            path: config_path.to_str().unwrap().to_string(),
            format: None,
        };
        let output = handler.handle(input).await.unwrap();
        assert_eq!(output.format, "mermaid");
        assert!(output.graph.contains("graph LR"));
        assert!(output.graph.contains("base-dir"));
        assert!(output.graph.contains("app-config"));
    }

    #[tokio::test]
    async fn test_fj063_graph_handler_dot() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    provider: apt\n    packages: [nginx]\n",
        )
        .unwrap();

        let handler = GraphHandler;
        let input = GraphInput {
            path: config_path.to_str().unwrap().to_string(),
            format: Some("dot".to_string()),
        };
        let output = handler.handle(input).await.unwrap();
        assert_eq!(output.format, "dot");
        assert!(output.graph.contains("digraph forjar"));
        assert!(output.graph.contains("pkg"));
    }

    #[tokio::test]
    async fn test_fj063_lint_handler_unused_machine() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n  unused-box:\n    hostname: unused\n    addr: 10.0.0.99\nresources:\n  test-pkg:\n    type: package\n    provider: apt\n    packages: [curl]\n",
        )
        .unwrap();

        let handler = LintHandler;
        let input = LintInput {
            path: config_path.to_str().unwrap().to_string(),
        };
        let output = handler.handle(input).await.unwrap();
        assert!(output.warnings.iter().any(|w| w.contains("unused-box")));
    }

    #[tokio::test]
    async fn test_fj063_show_handler_single_resource() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-file:\n    type: file\n    path: /tmp/test.txt\n    content: hello\n",
        )
        .unwrap();

        let handler = ShowHandler;
        let input = ShowInput {
            path: config_path.to_str().unwrap().to_string(),
            resource: Some("test-file".to_string()),
        };
        let output = handler.handle(input).await.unwrap();
        assert!(output.config.is_object());
    }

    #[tokio::test]
    async fn test_fj063_show_handler_missing_resource() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-file:\n    type: file\n    path: /tmp/test.txt\n    content: hello\n",
        )
        .unwrap();

        let handler = ShowHandler;
        let input = ShowInput {
            path: config_path.to_str().unwrap().to_string(),
            resource: Some("nonexistent".to_string()),
        };
        let result = handler.handle(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fj063_show_handler_full_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-file:\n    type: file\n    path: /tmp/test.txt\n    content: hello\n",
        )
        .unwrap();

        let handler = ShowHandler;
        let input = ShowInput {
            path: config_path.to_str().unwrap().to_string(),
            resource: None,
        };
        let output = handler.handle(input).await.unwrap();
        assert!(output.config.is_object());
    }

    #[tokio::test]
    async fn test_fj063_status_handler_empty() {
        let dir = tempfile::tempdir().unwrap();
        let handler = StatusHandler;
        let input = StatusInput {
            state_dir: Some(dir.path().to_str().unwrap().to_string()),
            machine: None,
        };
        let output = handler.handle(input).await.unwrap();
        assert!(output.machines.is_empty());
    }

    #[tokio::test]
    async fn test_fj063_status_handler_nonexistent_dir() {
        let handler = StatusHandler;
        let input = StatusInput {
            state_dir: Some("/nonexistent/state/dir".to_string()),
            machine: None,
        };
        let output = handler.handle(input).await.unwrap();
        assert!(output.machines.is_empty());
    }

    #[tokio::test]
    async fn test_fj063_registry_dispatch_validate() {
        let registry = build_registry();
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-pkg:\n    type: package\n    provider: apt\n    packages: [git]\n",
        )
        .unwrap();

        let input = serde_json::json!({
            "path": config_path.to_str().unwrap()
        });
        let result = registry
            .dispatch("forjar_validate", &serde_json::to_vec(&input).unwrap())
            .await;
        assert!(result.is_ok());
        let output: ValidateOutput = serde_json::from_slice(&result.unwrap()).unwrap();
        assert!(output.valid);
    }

    #[tokio::test]
    async fn test_fj063_registry_dispatch_unknown_tool() {
        let registry = build_registry();
        let result = registry.dispatch("nonexistent", b"{}").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fj063_plan_handler_with_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  pkg-a:\n    type: package\n    provider: apt\n    packages: [curl]\n  pkg-b:\n    type: package\n    provider: apt\n    packages: [wget]\n",
        )
        .unwrap();

        let handler = PlanHandler;
        let input = PlanInput {
            path: config_path.to_str().unwrap().to_string(),
            state_dir: Some(state_dir.to_str().unwrap().to_string()),
            resource: Some("pkg-a".to_string()),
            tag: None,
        };
        let output = handler.handle(input).await.unwrap();
        assert_eq!(output.changes.len(), 1);
        assert_eq!(output.changes[0].resource_id, "pkg-a");
    }

    #[tokio::test]
    async fn test_fj063_plan_handler_all_resources() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  pkg-a:\n    type: package\n    provider: apt\n    packages: [curl]\n  pkg-b:\n    type: package\n    provider: apt\n    packages: [wget]\n",
        )
        .unwrap();

        let handler = PlanHandler;
        let input = PlanInput {
            path: config_path.to_str().unwrap().to_string(),
            state_dir: Some(state_dir.to_str().unwrap().to_string()),
            resource: None,
            tag: None,
        };
        let output = handler.handle(input).await.unwrap();
        assert!(output.to_create >= 2);
    }

    #[tokio::test]
    async fn test_fj063_drift_handler_no_state() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-pkg:\n    type: package\n    provider: apt\n    packages: [curl]\n",
        )
        .unwrap();

        let handler = DriftHandler;
        let input = DriftInput {
            path: config_path.to_str().unwrap().to_string(),
            state_dir: Some(state_dir.to_str().unwrap().to_string()),
            machine: None,
        };
        let output = handler.handle(input).await.unwrap();
        assert!(!output.drifted);
    }

    #[tokio::test]
    async fn test_fj063_drift_handler_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  box-a:\n    hostname: a\n    addr: 10.0.0.1\n  box-b:\n    hostname: b\n    addr: 10.0.0.2\nresources:\n  test-pkg:\n    type: package\n    provider: apt\n    packages: [curl]\n",
        )
        .unwrap();

        let handler = DriftHandler;
        let input = DriftInput {
            path: config_path.to_str().unwrap().to_string(),
            state_dir: Some(state_dir.to_str().unwrap().to_string()),
            machine: Some("box-a".to_string()),
        };
        let output = handler.handle(input).await.unwrap();
        assert!(!output.drifted);
    }

    #[tokio::test]
    async fn test_fj063_lint_handler_clean_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  app-dir:\n    type: file\n    machine: local\n    path: /opt/app\n    state: directory\n",
        )
        .unwrap();

        let handler = LintHandler;
        let input = LintInput {
            path: config_path.to_str().unwrap().to_string(),
        };
        let output = handler.handle(input).await.unwrap();
        // No unused-machine warnings (structural lint is clean)
        let structural_warnings: Vec<_> = output
            .warnings
            .iter()
            .filter(|w| w.contains("Machine") || w.contains("[ERROR]"))
            .collect();
        assert!(
            structural_warnings.is_empty(),
            "expected no structural warnings, got: {:?}",
            structural_warnings
        );
        assert_eq!(output.error_count, 0);
    }

    #[tokio::test]
    async fn test_fj063_validate_handler_multiple_resources() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  web-pkg:\n    type: package\n    provider: apt\n    packages: [nginx]\n  web-conf:\n    type: file\n    path: /etc/nginx/nginx.conf\n    content: \"worker_processes 4;\"\n    depends_on: [web-pkg]\n  web-svc:\n    type: service\n    name: nginx\n    depends_on: [web-conf]\n",
        )
        .unwrap();

        let handler = ValidateHandler;
        let input = ValidateInput {
            path: config_path.to_str().unwrap().to_string(),
        };
        let output = handler.handle(input).await.unwrap();
        assert!(output.valid);
        assert_eq!(output.resource_count, 3);
        assert_eq!(output.machine_count, 1);
        assert!(output.errors.is_empty());
    }

    #[tokio::test]
    async fn test_fj063_graph_handler_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  step-a:\n    type: package\n    provider: apt\n    packages: [curl]\n  step-b:\n    type: file\n    path: /tmp/b.txt\n    content: hello\n    depends_on: [step-a]\n  step-c:\n    type: service\n    name: myapp\n    depends_on: [step-b]\n",
        )
        .unwrap();

        let handler = GraphHandler;
        let input = GraphInput {
            path: config_path.to_str().unwrap().to_string(),
            format: None,
        };
        let output = handler.handle(input).await.unwrap();
        assert_eq!(output.format, "mermaid");
        // Verify all three resources appear
        assert!(output.graph.contains("step-a"));
        assert!(output.graph.contains("step-b"));
        assert!(output.graph.contains("step-c"));
        // Verify the dependency chain edges: a->b and b->c
        assert!(output.graph.contains("step-a --> step-b"));
        assert!(output.graph.contains("step-b --> step-c"));
    }

    #[tokio::test]
    async fn test_fj063_status_handler_with_state() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // StatusHandler reads .json files from state_dir
        let state_json = serde_json::json!({
            "version": "1.0",
            "machine": "local",
            "resources": {
                "test-pkg": {
                    "resource_type": "Package",
                    "status": "Converged",
                    "hash": "abc123",
                    "details": {}
                }
            }
        });
        std::fs::write(
            state_dir.join("local.json"),
            serde_json::to_string_pretty(&state_json).unwrap(),
        )
        .unwrap();

        let handler = StatusHandler;
        let input = StatusInput {
            state_dir: Some(state_dir.to_str().unwrap().to_string()),
            machine: None,
        };
        let output = handler.handle(input).await.unwrap();
        assert_eq!(output.machines.len(), 1);
        assert_eq!(output.machines[0].name, "local");
    }

    #[tokio::test]
    async fn test_fj063_plan_handler_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, "this is not valid yaml: [[[").unwrap();

        let handler = PlanHandler;
        let input = PlanInput {
            path: config_path.to_str().unwrap().to_string(),
            state_dir: None,
            resource: None,
            tag: None,
        };
        let result = handler.handle(input).await;
        assert!(result.is_err(), "expected error for invalid config");
    }
}
