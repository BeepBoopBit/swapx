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

pub fn init_config() -> Result<Vec<PathBuf>, SwapxError> {
    let config_dir = global_config_dir()
        .ok_or_else(|| SwapxError::Config("Cannot determine config directory".into()))?;

    if config_dir.is_dir() {
        return Err(SwapxError::Config(
            format!("{} already exists (already initialized)", config_dir.display()),
        ));
    }

    let mut created = Vec::new();

    // Create ~/.config/swapx/
    fs::create_dir_all(&config_dir)?;
    created.push(config_dir.clone());

    // Write ~/.config/swapx/rules.yaml with commented-out examples
    let rules_path = config_dir.join("rules.yaml");
    fs::write(&rules_path, EXAMPLE_RULES_YAML)?;
    created.push(rules_path);

    // Create ~/.config/swapx/suggestions.d/
    let suggestions_dir = config_dir.join("suggestions.d");
    fs::create_dir_all(&suggestions_dir)?;
    created.push(suggestions_dir.clone());

    // Copy builtin suggestions to ~/.config/swapx/suggestions.d/builtin.yaml
    let builtin_path = suggestions_dir.join("builtin.yaml");
    fs::write(&builtin_path, BUILTIN_SUGGESTIONS_YAML)?;
    created.push(builtin_path);

    Ok(created)
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
    let mut deleted = Vec::new();

    // Delete the entire ~/.config/swapx/ directory (all rules, suggestions, etc.)
    if let Some(p) = global_config_dir() {
        if p.is_dir() {
            for entry in fs::read_dir(&p)? {
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

    // Local config (walk up from CWD)
    if let Some(p) = find_local_config() {
        if p.is_file() {
            fs::remove_file(&p)?;
            eprintln!("Deleted {}", p.display());
            deleted.push(p);
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
    use std::env;

    #[test]
    fn reset_all_deletes_existing_files() {
        // Set up a temp dir as XDG_CONFIG_HOME so global paths resolve there
        let tmp = tempfile::TempDir::new().unwrap();
        let config_dir = tmp.path().join("swapx");
        fs::create_dir_all(&config_dir).unwrap();

        // Create various rule files (not just hardcoded names)
        let rules_path = config_dir.join("rules.yaml");
        fs::write(&rules_path, "rules: []\n").unwrap();

        let plk_path = config_dir.join("rules.plk.yaml");
        fs::write(&plk_path, "rules: []\n").unwrap();

        let custom_path = config_dir.join("my-custom-rules.yaml");
        fs::write(&custom_path, "rules: []\n").unwrap();

        // Create suggestions.d with a file inside
        let suggestions = config_dir.join("suggestions.d");
        fs::create_dir_all(&suggestions).unwrap();
        fs::write(suggestions.join("test.yaml"), "suggestions: []\n").unwrap();

        // Create a local config in a temp working dir
        let work_dir = tempfile::TempDir::new().unwrap();
        let local_path = work_dir.path().join(".swapx.yaml");
        fs::write(&local_path, "rules: []\n").unwrap();

        // Override env vars and cwd so our functions find these paths
        env::set_var("XDG_CONFIG_HOME", tmp.path());
        let orig_dir = env::current_dir().unwrap();
        env::set_current_dir(work_dir.path()).unwrap();

        let deleted = reset_all().unwrap();

        // Restore cwd
        env::set_current_dir(&orig_dir).unwrap();

        // 3 files + 1 dir in config_dir + 1 local config = 5
        assert_eq!(deleted.len(), 5);
        assert!(!rules_path.exists());
        assert!(!plk_path.exists());
        assert!(!custom_path.exists());
        assert!(!suggestions.exists());
        assert!(!local_path.exists());
        // The swapx dir itself should still exist (empty)
        assert!(config_dir.exists());
    }

    #[test]
    fn reset_all_returns_empty_when_nothing_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        env::set_var("XDG_CONFIG_HOME", tmp.path());

        let work_dir = tempfile::TempDir::new().unwrap();
        let orig_dir = env::current_dir().unwrap();
        env::set_current_dir(work_dir.path()).unwrap();

        let deleted = reset_all().unwrap();

        env::set_current_dir(&orig_dir).unwrap();

        assert!(deleted.is_empty());
    }

    #[test]
    fn init_config_creates_expected_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        env::set_var("XDG_CONFIG_HOME", tmp.path());

        let created = init_config().unwrap();

        let config_dir = tmp.path().join("swapx");
        assert!(config_dir.is_dir());
        assert!(config_dir.join("rules.yaml").is_file());
        assert!(config_dir.join("suggestions.d").is_dir());
        assert!(config_dir.join("suggestions.d").join("builtin.yaml").is_file());

        // Should have created 4 entries: dir, rules.yaml, suggestions.d, builtin.yaml
        assert_eq!(created.len(), 4);

        // rules.yaml should have empty rules list
        let rules_contents = fs::read_to_string(config_dir.join("rules.yaml")).unwrap();
        assert!(rules_contents.contains("rules: []"));

        // builtin.yaml should contain valid suggestion YAML
        let builtin_contents =
            fs::read_to_string(config_dir.join("suggestions.d").join("builtin.yaml")).unwrap();
        assert!(builtin_contents.contains("suggestions:"));

        // Verify the builtin YAML is parseable
        let file: crate::suggest::SuggestionFile =
            serde_yaml_ng::from_str(&builtin_contents).unwrap();
        assert!(!file.suggestions.is_empty());
    }

    #[test]
    fn init_config_fails_if_already_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        env::set_var("XDG_CONFIG_HOME", tmp.path());

        let config_dir = tmp.path().join("swapx");
        fs::create_dir_all(&config_dir).unwrap();

        let result = init_config();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already exists"));
    }
}
