//! Tests: FJ-1437 agent registry.

#![allow(unused_imports)]
use super::agent_registry::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_empty_registry() {
        let dir = tempfile::tempdir().unwrap();
        let reg = load_agent_registry(dir.path()).unwrap();
        assert!(reg.recipes.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = AgentRegistry::default();
        reg.recipes.push(AgentRecipe {
            name: "code-helper".to_string(),
            description: "Code assistant agent".to_string(),
            category: AgentCategory::CodeAssistant,
            version: "1.0.0".to_string(),
            model: "claude-sonnet".to_string(),
            gpu_required: false,
            mcp_servers: vec!["filesystem".to_string()],
            health_check: Some("curl localhost:8080/health".to_string()),
            tags: vec!["code".to_string()],
        });
        save_agent_registry(dir.path(), &reg).unwrap();
        let loaded = load_agent_registry(dir.path()).unwrap();
        assert_eq!(loaded.recipes.len(), 1);
        assert_eq!(loaded.recipes[0].name, "code-helper");
    }

    #[test]
    fn test_search_agents() {
        let reg = AgentRegistry {
            recipes: vec![
                AgentRecipe {
                    name: "code-helper".to_string(),
                    description: "Code assistant".to_string(),
                    category: AgentCategory::CodeAssistant,
                    version: "1.0.0".to_string(),
                    model: "claude".to_string(),
                    gpu_required: false,
                    mcp_servers: vec![],
                    health_check: None,
                    tags: vec!["coding".to_string()],
                },
                AgentRecipe {
                    name: "data-bot".to_string(),
                    description: "Data analyst".to_string(),
                    category: AgentCategory::DataAnalyst,
                    version: "1.0.0".to_string(),
                    model: "claude".to_string(),
                    gpu_required: true,
                    mcp_servers: vec![],
                    health_check: None,
                    tags: vec!["data".to_string()],
                },
            ],
        };
        assert_eq!(search_agents(&reg, "code").len(), 1);
        assert_eq!(search_agents(&reg, "data").len(), 1);
        assert_eq!(search_agents(&reg, "coding").len(), 1);
    }

    #[test]
    fn test_cmd_agent_registry() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_agent_registry(dir.path(), None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_agent_category_display() {
        assert_eq!(format!("{}", AgentCategory::CodeAssistant), "code-assistant");
        assert_eq!(format!("{}", AgentCategory::SecurityAuditor), "security-auditor");
    }

    #[test]
    fn test_agent_recipe_serde() {
        let recipe = AgentRecipe {
            name: "test".to_string(),
            description: "test".to_string(),
            category: AgentCategory::Custom,
            version: "1.0.0".to_string(),
            model: "test".to_string(),
            gpu_required: false,
            mcp_servers: vec![],
            health_check: None,
            tags: vec![],
        };
        let json = serde_json::to_string(&recipe).unwrap();
        let round: AgentRecipe = serde_json::from_str(&json).unwrap();
        assert_eq!(round.name, "test");
    }
}
