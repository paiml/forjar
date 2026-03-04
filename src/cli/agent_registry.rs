//! FJ-1437: OpenClaw agent recipe registry.
//!
//! Curated library of agent deployment recipes:
//! code assistant, data analyst, security auditor, customer support.
//! Versioned, signed, composable.

use std::path::Path;

/// An agent recipe entry in the registry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentRecipe {
    pub name: String,
    pub description: String,
    pub category: AgentCategory,
    pub version: String,
    pub model: String,
    pub gpu_required: bool,
    pub mcp_servers: Vec<String>,
    pub health_check: Option<String>,
    pub tags: Vec<String>,
}

/// Agent recipe category.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AgentCategory {
    CodeAssistant,
    DataAnalyst,
    SecurityAuditor,
    CustomerSupport,
    Custom,
}

impl std::fmt::Display for AgentCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentCategory::CodeAssistant => write!(f, "code-assistant"),
            AgentCategory::DataAnalyst => write!(f, "data-analyst"),
            AgentCategory::SecurityAuditor => write!(f, "security-auditor"),
            AgentCategory::CustomerSupport => write!(f, "customer-support"),
            AgentCategory::Custom => write!(f, "custom"),
        }
    }
}

/// Agent registry.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentRegistry {
    pub recipes: Vec<AgentRecipe>,
}

/// Agent registry report.
#[derive(Debug, serde::Serialize)]
pub struct AgentRegistryReport {
    pub recipes: Vec<AgentRecipe>,
    pub total: usize,
    pub categories: Vec<String>,
}

/// Load agent registry from directory.
pub fn load_agent_registry(dir: &Path) -> Result<AgentRegistry, String> {
    let index = dir.join("agents.json");
    if !index.exists() {
        return Ok(AgentRegistry::default());
    }
    let data = std::fs::read_to_string(&index)
        .map_err(|e| format!("read: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("parse: {e}"))
}

/// Save agent registry.
#[allow(dead_code)]
pub fn save_agent_registry(dir: &Path, registry: &AgentRegistry) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("mkdir: {e}"))?;
    let data = serde_json::to_string_pretty(registry)
        .map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(dir.join("agents.json"), data)
        .map_err(|e| format!("write: {e}"))
}

/// List agent recipes.
pub fn cmd_agent_registry(
    dir: &Path,
    category: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let registry = load_agent_registry(dir)?;
    let filtered = filter_by_category(&registry, category);
    let categories = collect_categories(&filtered);

    let report = AgentRegistryReport {
        total: filtered.len(),
        categories,
        recipes: filtered,
    };

    if json {
        let out =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        print_agent_report(&report);
    }
    Ok(())
}

fn filter_by_category(registry: &AgentRegistry, category: Option<&str>) -> Vec<AgentRecipe> {
    match category {
        Some(cat) => registry
            .recipes
            .iter()
            .filter(|r| format!("{}", r.category) == cat)
            .cloned()
            .collect(),
        None => registry.recipes.clone(),
    }
}

fn collect_categories(recipes: &[AgentRecipe]) -> Vec<String> {
    let mut cats: Vec<String> = recipes
        .iter()
        .map(|r| format!("{}", r.category))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    cats.sort();
    cats
}

fn print_agent_report(report: &AgentRegistryReport) {
    println!("Agent Recipe Registry");
    println!("=====================");
    println!("Recipes: {} | Categories: {}", report.total, report.categories.join(", "));
    println!();
    for r in &report.recipes {
        let gpu = if r.gpu_required { " [GPU]" } else { "" };
        println!("  {} v{} ({}){}  — {}", r.name, r.version, r.category, gpu, r.description);
    }
}

/// Search agent recipes by name or tag.
#[allow(dead_code)]
pub fn search_agents<'a>(registry: &'a AgentRegistry, query: &str) -> Vec<&'a AgentRecipe> {
    let q = query.to_lowercase();
    registry
        .recipes
        .iter()
        .filter(|r| {
            r.name.to_lowercase().contains(&q)
                || r.description.to_lowercase().contains(&q)
                || r.tags.iter().any(|t| t.to_lowercase().contains(&q))
        })
        .collect()
}
