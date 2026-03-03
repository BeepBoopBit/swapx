use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config;
use crate::error::SwapxError;
use crate::models::{Replacement, Rule};

// ─── Data structures ───

#[derive(Debug, Deserialize)]
pub struct SuggestionFile {
    pub suggestions: Vec<SuggestionDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SuggestionDef {
    pub name: String,
    pub description: String,
    pub detect: DetectCondition,
    #[serde(default)]
    pub prompts: Vec<PromptDef>,
    pub rules: Vec<SuggestionRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DetectCondition {
    pub bin: Option<String>,
    pub file: Option<String>,
    pub project: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptDef {
    pub var: String,
    pub message: String,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub detect: Option<String>,
    #[serde(default)]
    pub filter: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SuggestionRule {
    #[serde(rename = "match")]
    pub match_pattern: String,
    pub replace: Vec<SuggestionReplacement>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SuggestionReplacement {
    pub label: String,
    #[serde(rename = "with")]
    pub with_value: String,
    #[serde(default)]
    pub default: bool,
}

// ─── Detection logic ───

pub fn bin_exists(name: &str) -> bool {
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

pub fn file_exists(pattern: &str) -> bool {
    if let Ok(matches) = glob::glob(pattern) {
        for entry in matches {
            if entry.is_ok() {
                return true;
            }
        }
    }
    false
}

pub fn project_detected(project_type: &str) -> bool {
    match project_type {
        "node" => Path::new("package.json").exists(),
        _ => false,
    }
}

pub fn detect_matches(cond: &DetectCondition) -> Option<String> {
    let mut reasons = Vec::new();

    if let Some(ref bin) = cond.bin {
        if !bin_exists(bin) {
            return None;
        }
        reasons.push(format!("{} found on PATH", bin));
    }

    if let Some(ref file) = cond.file {
        if !file_exists(file) {
            return None;
        }
        reasons.push(format!("{} exists", file));
    }

    if let Some(ref project) = cond.project {
        if !project_detected(project) {
            return None;
        }
        reasons.push(format!("{} project detected", project));
    }

    if reasons.is_empty() {
        return None;
    }

    Some(reasons.join(", "))
}

// ─── Template engine ───

pub fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

// ─── Prompt resolution ───

pub fn read_package_json_scripts() -> Result<Vec<String>, SwapxError> {
    let content = fs::read_to_string("package.json")?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    let mut scripts = Vec::new();
    if let Some(obj) = json.get("scripts").and_then(|s| s.as_object()) {
        for key in obj.keys() {
            scripts.push(key.clone());
        }
        scripts.sort();
    }
    Ok(scripts)
}

fn detect_options(source: &str, filter: &Option<Vec<String>>) -> Vec<String> {
    let options = match source {
        "package.json.scripts" => read_package_json_scripts().unwrap_or_default(),
        _ => Vec::new(),
    };

    if let Some(ref filter_list) = filter {
        options
            .into_iter()
            .filter(|o| filter_list.iter().any(|f| o.contains(f)))
            .collect()
    } else {
        options
    }
}

fn prompt_select(message: &str, options: &[String]) -> Result<String, SwapxError> {
    let selection = dialoguer::Select::new()
        .with_prompt(message)
        .items(options)
        .default(0)
        .interact()?;
    Ok(options[selection].clone())
}

fn prompt_input(message: &str, default: Option<&str>) -> Result<String, SwapxError> {
    let mut input = dialoguer::Input::<String>::new().with_prompt(message);
    if let Some(d) = default {
        input = input.default(d.to_string());
    }
    Ok(input.interact_text()?)
}

pub fn resolve_prompts(
    prompts: &[PromptDef],
    auto: bool,
    vars: &mut HashMap<String, String>,
) -> Result<(), SwapxError> {
    for prompt in prompts {
        let value = if let Some(ref detect_source) = prompt.detect {
            let options = detect_options(detect_source, &prompt.filter);
            if auto {
                // In auto mode, use first matching option or default
                options
                    .first()
                    .cloned()
                    .or_else(|| {
                        prompt
                            .default
                            .as_ref()
                            .map(|d| render_template(d, vars))
                    })
                    .unwrap_or_default()
            } else if options.is_empty() {
                // No detected options, fall back to text input
                let default = prompt.default.as_ref().map(|d| render_template(d, vars));
                prompt_input(&prompt.message, default.as_deref())?
            } else {
                prompt_select(&prompt.message, &options)?
            }
        } else if auto {
            prompt
                .default
                .as_ref()
                .map(|d| render_template(d, vars))
                .unwrap_or_default()
        } else {
            let default = prompt.default.as_ref().map(|d| render_template(d, vars));
            prompt_input(&prompt.message, default.as_deref())?
        };

        vars.insert(prompt.var.clone(), value);
    }
    Ok(())
}

// ─── Built-in suggestions ───

const BUILTIN_YAML: &str = include_str!("../suggestions/builtin.yaml");

pub fn builtin_suggestions() -> Vec<SuggestionDef> {
    let file: SuggestionFile =
        serde_yaml_ng::from_str(BUILTIN_YAML).expect("builtin suggestions YAML is invalid");
    file.suggestions
}

// ─── Suggestion pack loading ───

pub fn load_suggestion_packs() -> Vec<SuggestionDef> {
    let mut suggestions = Vec::new();

    let suggestions_dir = match dirs::config_dir() {
        Some(d) => d.join("swapx").join("suggestions.d"),
        None => return suggestions,
    };

    if !suggestions_dir.is_dir() {
        return suggestions;
    }

    let pattern = suggestions_dir.join("*.yaml").to_string_lossy().to_string();
    if let Ok(entries) = glob::glob(&pattern) {
        for entry in entries.flatten() {
            match load_suggestion_file(&entry) {
                Ok(file) => suggestions.extend(file.suggestions),
                Err(e) => {
                    eprintln!(
                        "swapx: warning: failed to load suggestion pack {}: {}",
                        entry.display(),
                        e
                    );
                }
            }
        }
    }

    suggestions
}

fn load_suggestion_file(path: &Path) -> Result<SuggestionFile, SwapxError> {
    let contents = fs::read_to_string(path)?;
    let file: SuggestionFile = serde_yaml_ng::from_str(&contents)?;
    Ok(file)
}

// ─── Rule generation ───

pub fn suggestion_to_rules(
    def: &SuggestionDef,
    vars: &HashMap<String, String>,
    dir_scope: Option<String>,
) -> Vec<Rule> {
    def.rules
        .iter()
        .map(|sr| {
            let match_pattern = render_template(&sr.match_pattern, vars);
            let replacements = sr
                .replace
                .iter()
                .map(|rep| Replacement {
                    label: render_template(&rep.label, vars),
                    with_value: render_template(&rep.with_value, vars),
                    default: rep.default,
                    when: None,
                })
                .collect();

            Rule {
                match_patterns: vec![match_pattern],
                regex: false,
                enabled: true,
                dir: dir_scope.clone(),
                replace: replacements,
            }
        })
        .collect()
}

// ─── Main flow ───

pub fn run_suggest(check: bool, auto: bool) -> Result<(), SwapxError> {
    // 1. Gather built-in + pack suggestions
    let mut all_suggestions = builtin_suggestions();
    all_suggestions.extend(load_suggestion_packs());

    // 2. Run detection, filter to applicable
    let mut applicable: Vec<(SuggestionDef, String)> = Vec::new();
    for suggestion in &all_suggestions {
        if let Some(reason) = detect_matches(&suggestion.detect) {
            applicable.push((suggestion.clone(), reason));
        }
    }

    // 3. Display found suggestions
    if applicable.is_empty() {
        eprintln!("No suggestions found for your system.");
        return Ok(());
    }

    eprintln!("Found {} suggestion(s):\n", applicable.len());
    for (i, (suggestion, reason)) in applicable.iter().enumerate() {
        eprintln!(
            "  {}. {} — {}",
            i + 1,
            suggestion.name,
            suggestion.description
        );
        eprintln!("     detected: {}", reason);
    }
    eprintln!();

    // 4. If --check: display count and exit
    if check {
        eprintln!(
            "{} suggestion(s) available. Run `swapx suggest` to apply.",
            applicable.len()
        );
        return Ok(());
    }

    // 5. Ask: all / pick / none (or accept all if --auto)
    let accepted: Vec<(SuggestionDef, String)> = if auto {
        applicable
    } else {
        let selection = dialoguer::Select::new()
            .with_prompt("What would you like to do?")
            .items(&["Accept all", "Pick individually", "Cancel"])
            .default(0)
            .interact()?;

        match selection {
            0 => applicable,
            1 => {
                let mut picked = Vec::new();
                for (suggestion, reason) in applicable {
                    let accept = dialoguer::Confirm::new()
                        .with_prompt(format!(
                            "Accept \"{}\" ({})?",
                            suggestion.name, suggestion.description
                        ))
                        .default(true)
                        .interact()?;
                    if accept {
                        picked.push((suggestion, reason));
                    }
                }
                picked
            }
            _ => {
                eprintln!("Cancelled.");
                return Ok(());
            }
        }
    };

    if accepted.is_empty() {
        eprintln!("No suggestions accepted.");
        return Ok(());
    }

    // 6. For accepted: resolve prompts, generate rules
    let mut all_rules: Vec<Rule> = Vec::new();
    for (suggestion, _reason) in &accepted {
        let mut vars = HashMap::new();
        resolve_prompts(&suggestion.prompts, auto, &mut vars)?;
        let rules = suggestion_to_rules(suggestion, &vars, None);
        all_rules.extend(rules);
    }

    // 7. Ask save location (local vs global), or global if --auto
    let local = if auto {
        false
    } else {
        let selection = dialoguer::Select::new()
            .with_prompt("Save rules to")
            .items(&["Global (~/.config/swapx/rules.yaml)", "Local (.swapx.yaml)"])
            .default(0)
            .interact()?;
        selection == 1
    };

    // 8. Save via config::save_rule()
    let mut saved_path = PathBuf::new();
    for rule in all_rules {
        saved_path = config::save_rule(rule, local)?;
    }

    eprintln!(
        "Saved {} rule(s) to {}",
        accepted.len(),
        saved_path.display()
    );

    Ok(())
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_render_template() {
        let mut vars = HashMap::new();
        vars.insert("script".to_string(), "dev".to_string());
        vars.insert("process_name".to_string(), "my-app".to_string());

        assert_eq!(
            render_template("pnpm run {{script}}", &vars),
            "pnpm run dev"
        );
        assert_eq!(
            render_template(
                "uc start --name {{process_name}} -- pnpm run {{script}}",
                &vars
            ),
            "uc start --name my-app -- pnpm run dev"
        );
        // No variables
        assert_eq!(render_template("hello world", &vars), "hello world");
        // Unknown variable stays
        assert_eq!(render_template("{{unknown}}", &vars), "{{unknown}}");
    }

    #[test]
    fn test_detect_condition_bin() {
        // `sh` should exist on any unix system
        let cond = DetectCondition {
            bin: Some("sh".into()),
            file: None,
            project: None,
        };
        let result = detect_matches(&cond);
        assert!(result.is_some());
        assert!(result.unwrap().contains("sh found on PATH"));
    }

    #[test]
    fn test_detect_condition_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();

        let cond = DetectCondition {
            bin: None,
            file: Some(file_path.to_string_lossy().to_string()),
            project: None,
        };
        assert!(detect_matches(&cond).is_some());

        let cond_missing = DetectCondition {
            bin: None,
            file: Some(dir.path().join("nonexistent.txt").to_string_lossy().to_string()),
            project: None,
        };
        assert!(detect_matches(&cond_missing).is_none());
    }

    #[test]
    fn test_detect_condition_project_node() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let cond = DetectCondition {
            bin: None,
            file: None,
            project: Some("node".into()),
        };
        let result = detect_matches(&cond);
        assert!(result.is_some());
        assert!(result.unwrap().contains("node project detected"));

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_suggestion_to_rules() {
        let def = SuggestionDef {
            name: "test".into(),
            description: "test desc".into(),
            detect: DetectCondition {
                bin: None,
                file: None,
                project: None,
            },
            prompts: vec![],
            rules: vec![SuggestionRule {
                match_pattern: "pnpm run {{script}}".into(),
                replace: vec![
                    SuggestionReplacement {
                        label: "run with uc".into(),
                        with_value: "uc start -- pnpm run {{script}}".into(),
                        default: false,
                    },
                    SuggestionReplacement {
                        label: "run directly".into(),
                        with_value: "pnpm run {{script}}".into(),
                        default: true,
                    },
                ],
            }],
        };

        let mut vars = HashMap::new();
        vars.insert("script".to_string(), "dev".to_string());

        let rules = suggestion_to_rules(&def, &vars, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].match_patterns, vec!["pnpm run dev"]);
        assert_eq!(rules[0].replace.len(), 2);
        assert_eq!(rules[0].replace[0].label, "run with uc");
        assert_eq!(rules[0].replace[0].with_value, "uc start -- pnpm run dev");
        assert_eq!(rules[0].replace[1].label, "run directly");
        assert_eq!(rules[0].replace[1].with_value, "pnpm run dev");
        assert!(rules[0].replace[1].default);
    }

    #[test]
    fn test_builtin_suggestions_valid() {
        let builtins = builtin_suggestions();
        assert!(!builtins.is_empty());
        for s in &builtins {
            assert!(!s.name.is_empty(), "name should not be empty");
            assert!(!s.description.is_empty(), "description should not be empty");
            assert!(
                s.detect.bin.is_some() || s.detect.file.is_some() || s.detect.project.is_some(),
                "detect should have at least one condition"
            );
            assert!(!s.rules.is_empty(), "should have at least one rule");
            for rule in &s.rules {
                assert!(!rule.match_pattern.is_empty());
                assert!(!rule.replace.is_empty());
            }
        }
    }

    #[test]
    fn test_load_suggestion_pack_yaml() {
        let yaml = r#"
suggestions:
  - name: test-pack
    description: "test suggestion"
    detect:
      bin: test-bin
    rules:
      - match: "test-cmd"
        replace:
          - label: "use alt"
            with: "alt-cmd"
            default: true
          - label: "keep"
            with: "test-cmd"
"#;
        let file: SuggestionFile = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(file.suggestions.len(), 1);
        assert_eq!(file.suggestions[0].name, "test-pack");
        assert_eq!(file.suggestions[0].rules.len(), 1);
        assert_eq!(file.suggestions[0].rules[0].replace.len(), 2);
    }

    #[test]
    fn test_read_package_json_scripts() {
        let dir = TempDir::new().unwrap();
        let pkg = r#"{
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start"
  }
}"#;
        fs::write(dir.path().join("package.json"), pkg).unwrap();

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let scripts = read_package_json_scripts().unwrap();
        assert_eq!(scripts, vec!["build", "dev", "start"]);

        env::set_current_dir(original_dir).unwrap();
    }
}
