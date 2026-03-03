use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::SwapxError;
use crate::models::{ConfigFile, Rule};

pub fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("swapx").join("rules.yaml"))
}

pub fn global_plk_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("swapx").join("rules.plk.yaml"))
}

pub fn find_local_config() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(".swapx.yaml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn load_config_file(path: &Path) -> Result<ConfigFile, SwapxError> {
    let contents = fs::read_to_string(path)?;
    let config: ConfigFile = serde_yaml_ng::from_str(&contents)?;
    Ok(config)
}

/// Build a canonical merge key from a rule's match patterns.
/// Sorted and joined so that `["a","b"]` and `["b","a"]` produce the same key.
fn rule_merge_key(rule: &Rule) -> String {
    let mut sorted = rule.match_patterns.clone();
    sorted.sort();
    sorted.join("\0")
}

pub fn load_merged_config() -> Result<ConfigFile, SwapxError> {
    let mut rules_map: HashMap<String, Rule> = HashMap::new();

    // Load global config first
    if let Some(global_path) = global_config_path() {
        if global_path.is_file() {
            let global = load_config_file(&global_path)?;
            for rule in global.rules {
                rules_map.insert(rule_merge_key(&rule), rule);
            }
        }
    }

    // Load global plk config (overrides global rules with same match key)
    if let Some(plk_path) = global_plk_config_path() {
        if plk_path.is_file() {
            let plk = load_config_file(&plk_path)?;
            for rule in plk.rules {
                rules_map.insert(rule_merge_key(&rule), rule);
            }
        }
    }

    // Local config overrides global by match pattern
    if let Some(local_path) = find_local_config() {
        let local = load_config_file(&local_path)?;
        for rule in local.rules {
            rules_map.insert(rule_merge_key(&rule), rule);
        }
    }

    let rules: Vec<Rule> = rules_map.into_values().collect();
    Ok(ConfigFile { rules })
}

const BUILTIN_SUGGESTIONS_YAML: &str = include_str!("../suggestions/builtin.yaml");

const EXAMPLE_RULES_YAML: &str = "\
# swapx rules configuration
# Uncomment and modify the examples below, or use `swapx suggest` to auto-generate rules.
#
# rules:
#   - match: \"git checkout\"
#     replace:
#       - label: use-switch
#         with: \"git switch\"
#         default: true
#   - match: \"python \"
#     replace:
#       - label: use-python3
#         with: \"python3 \"
#         default: true
rules: []
";

pub enum InitOverwrite {
    /// Ask the user interactively per file
    Prompt,
    /// Overwrite everything without asking
    Force,
    /// Error if anything exists (for non-TTY / tests)
    Error,
}

#[derive(Debug)]
pub enum InitAction {
    Created(PathBuf),
    Replaced(PathBuf),
    Skipped(PathBuf),
}

fn write_or_prompt(
    path: &Path,
    contents: &str,
    overwrite: &InitOverwrite,
) -> Result<InitAction, SwapxError> {
    if path.exists() {
        match overwrite {
            InitOverwrite::Error => {
                return Err(SwapxError::Config(format!(
                    "{} already exists (already initialized)",
                    path.display()
                )));
            }
            InitOverwrite::Force => {
                fs::write(path, contents)?;
                return Ok(InitAction::Replaced(path.to_path_buf()));
            }
            InitOverwrite::Prompt => {
                let prompt = format!("{} already exists. Replace with defaults?", path.display());
                let confirm = dialoguer::Confirm::new()
                    .with_prompt(prompt)
                    .default(false)
                    .interact()
                    .map_err(|e| SwapxError::Config(format!("prompt failed: {}", e)))?;
                if confirm {
                    fs::write(path, contents)?;
                    return Ok(InitAction::Replaced(path.to_path_buf()));
                } else {
                    return Ok(InitAction::Skipped(path.to_path_buf()));
                }
            }
        }
    }

    fs::write(path, contents)?;
    Ok(InitAction::Created(path.to_path_buf()))
}

pub fn init_config(overwrite: InitOverwrite) -> Result<Vec<InitAction>, SwapxError> {
    let config_dir = global_config_dir()
        .ok_or_else(|| SwapxError::Config("Cannot determine config directory".into()))?;
    init_config_at(&config_dir, overwrite)
}

fn init_config_at(
    config_dir: &Path,
    overwrite: InitOverwrite,
) -> Result<Vec<InitAction>, SwapxError> {
    let mut actions = Vec::new();

    // Ensure config dir exists
    fs::create_dir_all(config_dir)?;

    // Write rules.yaml
    let rules_path = config_dir.join("rules.yaml");
    actions.push(write_or_prompt(
        &rules_path,
        EXAMPLE_RULES_YAML,
        &overwrite,
    )?);

    // Ensure suggestions.d/ exists
    let suggestions_dir = config_dir.join("suggestions.d");
    fs::create_dir_all(&suggestions_dir)?;

    // Write suggestions.d/builtin.yaml
    let builtin_path = suggestions_dir.join("builtin.yaml");
    actions.push(write_or_prompt(
        &builtin_path,
        BUILTIN_SUGGESTIONS_YAML,
        &overwrite,
    )?);

    Ok(actions)
}

pub fn toggle_rule(pattern: &str, enabled: bool) -> Result<PathBuf, SwapxError> {
    // Try local config first, then global
    let paths: Vec<Option<PathBuf>> = vec![find_local_config(), global_config_path()];

    for path in paths.into_iter().flatten() {
        if !path.is_file() {
            continue;
        }
        let mut config = load_config_file(&path)?;
        if let Some(rule) = config
            .rules
            .iter_mut()
            .find(|r| r.match_patterns.iter().any(|p| p == pattern))
        {
            rule.enabled = enabled;
            let yaml = serde_yaml_ng::to_string(&config)?;
            fs::write(&path, yaml)?;
            return Ok(path);
        }
    }

    Err(SwapxError::Config(format!(
        "No rule found with match pattern: \"{}\"",
        pattern
    )))
}

pub fn global_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("swapx"))
}

pub fn reset_all() -> Result<Vec<PathBuf>, SwapxError> {
    let global = global_config_dir();
    let local = find_local_config();
    reset_all_at(global.as_deref(), local.as_deref())
}

fn reset_all_at(
    config_dir: Option<&Path>,
    local_config: Option<&Path>,
) -> Result<Vec<PathBuf>, SwapxError> {
    let mut deleted = Vec::new();

    if let Some(p) = config_dir {
        if p.is_dir() {
            for entry in fs::read_dir(p)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
                eprintln!("Deleted {}", path.display());
                deleted.push(path);
            }
        }
    }

    if let Some(p) = local_config {
        if p.is_file() {
            fs::remove_file(p)?;
            eprintln!("Deleted {}", p.display());
            deleted.push(p.to_path_buf());
        }
    }

    Ok(deleted)
}

pub fn save_rule(rule: Rule, local: bool) -> Result<PathBuf, SwapxError> {
    let path = if local {
        find_local_config()
            .or_else(|| Some(std::env::current_dir().ok()?.join(".swapx.yaml")))
            .ok_or_else(|| SwapxError::Config("Cannot determine local config path".into()))?
    } else {
        let p = global_config_path()
            .ok_or_else(|| SwapxError::Config("Cannot determine global config path".into()))?;
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        p
    };

    let mut config = if path.is_file() {
        load_config_file(&path)?
    } else {
        ConfigFile { rules: vec![] }
    };

    let new_key = rule_merge_key(&rule);

    // Replace existing rule with same merge key, or append
    if let Some(existing) = config
        .rules
        .iter_mut()
        .find(|r| rule_merge_key(r) == new_key)
    {
        *existing = rule;
    } else {
        config.rules.push(rule);
    }

    let yaml = serde_yaml_ng::to_string(&config)?;
    fs::write(&path, yaml)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_all_deletes_existing_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_dir = tmp.path().join("swapx");
        fs::create_dir_all(&config_dir).unwrap();

        let rules_path = config_dir.join("rules.yaml");
        fs::write(&rules_path, "rules: []\n").unwrap();

        let plk_path = config_dir.join("rules.plk.yaml");
        fs::write(&plk_path, "rules: []\n").unwrap();

        let custom_path = config_dir.join("my-custom-rules.yaml");
        fs::write(&custom_path, "rules: []\n").unwrap();

        let suggestions = config_dir.join("suggestions.d");
        fs::create_dir_all(&suggestions).unwrap();
        fs::write(suggestions.join("test.yaml"), "suggestions: []\n").unwrap();

        let local_path = tmp.path().join(".swapx.yaml");
        fs::write(&local_path, "rules: []\n").unwrap();

        let deleted = reset_all_at(Some(&config_dir), Some(&local_path)).unwrap();

        // 3 files + 1 dir in config_dir + 1 local config = 5
        assert_eq!(deleted.len(), 5);
        assert!(!rules_path.exists());
        assert!(!plk_path.exists());
        assert!(!custom_path.exists());
        assert!(!suggestions.exists());
        assert!(!local_path.exists());
        assert!(config_dir.exists());
    }

    #[test]
    fn reset_all_returns_empty_when_nothing_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_dir = tmp.path().join("swapx");
        let local_path = tmp.path().join(".swapx.yaml");

        let deleted = reset_all_at(Some(&config_dir), Some(&local_path)).unwrap();

        assert!(deleted.is_empty());
    }

    #[test]
    fn init_config_creates_expected_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_dir = tmp.path().join("swapx");

        let actions = init_config_at(&config_dir, InitOverwrite::Error).unwrap();

        assert!(config_dir.is_dir());
        assert!(config_dir.join("rules.yaml").is_file());
        assert!(config_dir.join("suggestions.d").is_dir());
        assert!(config_dir
            .join("suggestions.d")
            .join("builtin.yaml")
            .is_file());

        assert_eq!(actions.len(), 2);
        assert!(matches!(&actions[0], InitAction::Created(_)));
        assert!(matches!(&actions[1], InitAction::Created(_)));

        let rules_contents = fs::read_to_string(config_dir.join("rules.yaml")).unwrap();
        assert!(rules_contents.contains("rules: []"));

        let builtin_contents =
            fs::read_to_string(config_dir.join("suggestions.d").join("builtin.yaml")).unwrap();
        assert!(builtin_contents.contains("suggestions:"));

        let file: crate::suggest::SuggestionFile =
            serde_yaml_ng::from_str(&builtin_contents).unwrap();
        assert!(!file.suggestions.is_empty());
    }

    #[test]
    fn init_config_fails_if_already_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_dir = tmp.path().join("swapx");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("rules.yaml"), "rules: []\n").unwrap();

        let result = init_config_at(&config_dir, InitOverwrite::Error);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already exists"));
    }

    #[test]
    fn init_config_force_replaces_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_dir = tmp.path().join("swapx");
        let suggestions_dir = config_dir.join("suggestions.d");
        fs::create_dir_all(&suggestions_dir).unwrap();

        fs::write(config_dir.join("rules.yaml"), "rules: [custom]\n").unwrap();
        fs::write(suggestions_dir.join("builtin.yaml"), "old content\n").unwrap();

        let actions = init_config_at(&config_dir, InitOverwrite::Force).unwrap();

        assert_eq!(actions.len(), 2);
        assert!(matches!(&actions[0], InitAction::Replaced(_)));
        assert!(matches!(&actions[1], InitAction::Replaced(_)));

        let rules_contents = fs::read_to_string(config_dir.join("rules.yaml")).unwrap();
        assert!(rules_contents.contains("rules: []"));

        let builtin_contents = fs::read_to_string(suggestions_dir.join("builtin.yaml")).unwrap();
        assert!(builtin_contents.contains("suggestions:"));
    }
}
