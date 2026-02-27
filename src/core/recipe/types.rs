//! Recipe type definitions.

use super::super::types::Resource;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A recipe source -- where to load recipes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecipeSource {
    Local {
        path: String,
    },
    Git {
        git: String,
        #[serde(default)]
        r#ref: Option<String>,
        #[serde(default)]
        path: Option<String>,
    },
}

/// A recipe file -- declares inputs and resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeFile {
    pub recipe: RecipeMetadata,
    pub resources: IndexMap<String, Resource>,
}

/// Recipe metadata and input declarations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeMetadata {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub inputs: IndexMap<String, RecipeInput>,
    #[serde(default)]
    pub requires: Vec<RecipeRequirement>,
}

/// A recipe input declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeInput {
    #[serde(rename = "type")]
    pub input_type: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default: Option<serde_yaml_ng::Value>,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
    #[serde(default)]
    pub choices: Vec<String>,
}

/// A recipe dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeRequirement {
    pub recipe: String,
}
