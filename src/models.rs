use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub rules: Vec<Rule>,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    #[serde(rename = "match")]
    pub match_pattern: String,

    #[serde(default)]
    pub regex: bool,

    #[serde(default = "default_enabled")]
    pub enabled: bool,

    pub replace: Vec<Replacement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhenCondition {
    /// Glob pattern for current working directory, e.g. "~/work/**"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// Environment variable condition: "KEY=VALUE" or just "KEY" (checks existence)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
    pub label: String,

    #[serde(rename = "with")]
    pub with_value: String,

    #[serde(default)]
    pub default: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<WhenCondition>,
}
