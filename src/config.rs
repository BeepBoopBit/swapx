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

pub fn init_local_config() -> Result<PathBuf, SwapxError> {
    let path = std::env::current_dir()?.join(".swapx.yaml");
    if path.exists() {
        return Err(SwapxError::Config(
            ".swapx.yaml already exists in this directory".into(),
        ));
    }

    let example = ConfigFile {
        rules: vec![
            Rule {
                match_patterns: vec!["git checkout".into()],
                regex: false,
                enabled: true,
                dir: None,
                replace: vec![crate::models::Replacement {
                    label: "use-switch".into(),
                    with_value: "git switch".into(),
                    default: true,
                    when: None,
                }],
            },
            Rule {
                match_patterns: vec!["python ".into()],
                regex: false,
                enabled: true,
                dir: None,
                replace: vec![crate::models::Replacement {
                    label: "use-python3".into(),
                    with_value: "python3 ".into(),
                    default: true,
                    when: None,
                }],
            },
        ],
    };

    let yaml = serde_yaml_ng::to_string(&example)?;
    fs::write(&path, yaml)?;
    Ok(path)
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
