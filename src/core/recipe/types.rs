//! Recipe type definitions.

use super::super::types::Resource;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A recipe source -- where to load recipes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecipeSource {
    /// Load recipe from a local filesystem path.
    Local {
        /// Local file path.
        path: String,
    },
    /// Load recipe from a Git repository.
    Git {
        /// Git repository URL.
        git: String,
        /// Git ref (branch, tag, commit).
        #[serde(default)]
        r#ref: Option<String>,
        /// Subdirectory path within the repo.
        #[serde(default)]
        path: Option<String>,
    },
}

/// A recipe file -- declares inputs and resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeFile {
    /// Recipe metadata and input declarations.
    pub recipe: RecipeMetadata,
    /// Resources declared by this recipe.
    pub resources: IndexMap<String, Resource>,
}

/// Recipe metadata and input declarations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeMetadata {
    /// Recipe name.
    pub name: String,
    /// Recipe version.
    #[serde(default)]
    pub version: Option<String>,
    /// Recipe description.
    #[serde(default)]
    pub description: Option<String>,
    /// Declared input parameters.
    #[serde(default)]
    pub inputs: IndexMap<String, RecipeInput>,
    /// Required dependency recipes.
    #[serde(default)]
    pub requires: Vec<RecipeRequirement>,
}

/// A recipe input declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeInput {
    /// Input value type (string, int, bool, etc.).
    #[serde(rename = "type")]
    pub input_type: String,
    /// Input description.
    #[serde(default)]
    pub description: Option<String>,
    /// Default value if not provided.
    #[serde(default)]
    pub default: Option<serde_yaml_ng::Value>,
    /// Minimum value (for numeric inputs).
    #[serde(default)]
    pub min: Option<i64>,
    /// Maximum value (for numeric inputs).
    #[serde(default)]
    pub max: Option<i64>,
    /// Allowed values (for enum inputs).
    #[serde(default)]
    pub choices: Vec<String>,
}

/// A recipe dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeRequirement {
    /// Name of the required recipe.
    pub recipe: String,
}
