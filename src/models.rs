use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub rules: Vec<Rule>,
}

fn default_enabled() -> bool {
    true
}

/// Deserialize a string-or-list into Vec<String>.
/// Accepts both `match: "foo"` and `match: ["foo", "bar"]`.
fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        Single(String),
        Multiple(Vec<String>),
    }

    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::Single(s) => Ok(vec![s]),
        StringOrVec::Multiple(v) => Ok(v),
    }
}

/// Serialize Vec<String> as a plain string when len==1, list otherwise.
/// This preserves round-trip format for single-pattern rules.
fn serialize_string_or_vec<S>(patterns: &Vec<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if patterns.len() == 1 {
        serializer.serialize_str(&patterns[0])
    } else {
        patterns.serialize(serializer)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    #[serde(
        rename = "match",
        deserialize_with = "deserialize_string_or_vec",
        serialize_with = "serialize_string_or_vec"
    )]
    pub match_patterns: Vec<String>,

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
