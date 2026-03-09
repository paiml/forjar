//! Additional coverage tests for agent_registry.rs — cmd output, category filter.

use super::agent_registry::*;

fn make_registry_with_entries() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let reg = AgentRegistry {
        recipes: vec![
            AgentRecipe {
                name: "code-bot".to_string(),
                description: "Code assistant agent".to_string(),
                category: AgentCategory::CodeAssistant,
                version: "1.0.0".to_string(),
                model: "claude-sonnet".to_string(),
                gpu_required: false,
                mcp_servers: vec!["filesystem".to_string()],
                health_check: Some("curl localhost:8080/health".to_string()),
                tags: vec!["code".to_string(), "dev".to_string()],
            },
            AgentRecipe {
                name: "data-analyst".to_string(),
                description: "Data pipeline agent".to_string(),
                category: AgentCategory::DataAnalyst,
                version: "2.0.0".to_string(),
                model: "claude-opus".to_string(),
                gpu_required: true,
                mcp_servers: vec!["sql".to_string()],
                health_check: None,
                tags: vec!["data".to_string()],
            },
            AgentRecipe {
                name: "sec-audit".to_string(),
                description: "Security auditor agent".to_string(),
                category: AgentCategory::SecurityAuditor,
                version: "1.0.0".to_string(),
                model: "claude-sonnet".to_string(),
                gpu_required: false,
                mcp_servers: vec![],
                health_check: None,
                tags: vec!["security".to_string()],
            },
        ],
    };
    save_agent_registry(dir.path(), &reg).unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

// ── cmd_agent_registry with entries ──────────────────────────────────

#[test]
fn cmd_registry_text_output() {
    let (_dir, path) = make_registry_with_entries();
    assert!(cmd_agent_registry(&path, None, false).is_ok());
}

#[test]
fn cmd_registry_json_output() {
    let (_dir, path) = make_registry_with_entries();
    assert!(cmd_agent_registry(&path, None, true).is_ok());
}

// ── category filter ──────────────────────────────────────────────────

#[test]
fn cmd_filter_by_category() {
    let (_dir, path) = make_registry_with_entries();
    assert!(cmd_agent_registry(&path, Some("code-assistant"), false).is_ok());
}

#[test]
fn cmd_filter_by_data_analyst() {
    let (_dir, path) = make_registry_with_entries();
    assert!(cmd_agent_registry(&path, Some("data-analyst"), true).is_ok());
}

#[test]
fn cmd_filter_no_match() {
    let (_dir, path) = make_registry_with_entries();
    assert!(cmd_agent_registry(&path, Some("nonexistent"), false).is_ok());
}

// ── AgentCategory display all variants ───────────────────────────────

#[test]
fn category_display_all() {
    assert_eq!(format!("{}", AgentCategory::DataAnalyst), "data-analyst");
    assert_eq!(format!("{}", AgentCategory::CustomerSupport), "customer-support");
    assert_eq!(format!("{}", AgentCategory::Custom), "custom");
}

// ── search_agents edge cases ─────────────────────────────────────────

#[test]
fn search_empty_registry() {
    let reg = AgentRegistry::default();
    assert!(search_agents(&reg, "test").is_empty());
}

#[test]
fn search_by_description() {
    let (_dir, path) = make_registry_with_entries();
    let reg = load_agent_registry(&path).unwrap();
    let results = search_agents(&reg, "pipeline");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "data-analyst");
}

#[test]
fn search_case_insensitive() {
    let (_dir, path) = make_registry_with_entries();
    let reg = load_agent_registry(&path).unwrap();
    let results = search_agents(&reg, "CODE");
    assert!(!results.is_empty());
}
