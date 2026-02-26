use regex::Regex;

use crate::error::SwapxError;
use crate::models::{Rule, WhenCondition};

#[derive(Debug)]
pub struct PendingChoice {
    pub rule: Rule,
}

#[derive(Debug)]
pub struct TransformResult {
    pub command: String,
    pub changed: bool,
    pub pending_choices: Vec<PendingChoice>,
}

pub fn apply_rules(
    command: &str,
    rules: &[Rule],
    use_defaults: bool,
) -> Result<TransformResult, SwapxError> {
    let mut result = command.to_string();
    let mut changed = false;
    let mut pending_choices = Vec::new();

    for rule in rules {
        if !rule.enabled {
            continue;
        }

        let matches = if rule.regex {
            let re = Regex::new(&rule.match_pattern)?;
            re.is_match(&result)
        } else {
            result.contains(&rule.match_pattern)
        };

        if !matches {
            continue;
        }

        if rule.replace.len() == 1 {
            // Single replacement: auto-apply (check when condition if present)
            let repl = &rule.replace[0];
            let when_ok = repl.when.as_ref().map(evaluate_when).unwrap_or(true);
            if when_ok {
                result = do_replace(&result, rule, &repl.with_value)?;
                changed = true;
            }
        } else {
            // Multiple replacements: check when conditions first
            let when_matches: Vec<&crate::models::Replacement> = rule
                .replace
                .iter()
                .filter(|r| r.when.as_ref().map(evaluate_when).unwrap_or(false))
                .collect();

            if when_matches.len() == 1 {
                // Exactly one when-condition matches: auto-select it
                result = do_replace(&result, rule, &when_matches[0].with_value)?;
                changed = true;
            } else if when_matches.len() > 1 {
                // Multiple when-conditions match: ambiguous, need choice
                pending_choices.push(PendingChoice { rule: rule.clone() });
            } else if use_defaults {
                // No when-conditions matched; fall through to default logic
                if let Some(default_repl) = rule.replace.iter().find(|r| r.default) {
                    result = do_replace(&result, rule, &default_repl.with_value)?;
                    changed = true;
                } else {
                    pending_choices.push(PendingChoice { rule: rule.clone() });
                }
            } else {
                // No when match, no defaults: need interactive choice
                pending_choices.push(PendingChoice { rule: rule.clone() });
            }
        }
    }

    Ok(TransformResult {
        command: result,
        changed,
        pending_choices,
    })
}

fn do_replace(command: &str, rule: &Rule, replacement: &str) -> Result<String, SwapxError> {
    if rule.regex {
        let re = Regex::new(&rule.match_pattern)?;
        Ok(re.replace_all(command, replacement).into_owned())
    } else {
        Ok(command.replace(&rule.match_pattern, replacement))
    }
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}{}", home.display(), &path[1..]);
        }
    }
    path.to_string()
}

pub fn evaluate_when(when: &WhenCondition) -> bool {
    // Check cwd glob condition
    if let Some(ref cwd_pattern) = when.cwd {
        let expanded = expand_tilde(cwd_pattern);
        let pattern = match glob::Pattern::new(&expanded) {
            Ok(p) => p,
            Err(_) => return false,
        };
        let current_dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(_) => return false,
        };
        if !pattern.matches_path(&current_dir) {
            return false;
        }
    }

    // Check env var condition
    if let Some(ref env_cond) = when.env {
        if let Some((key, value)) = env_cond.split_once('=') {
            // KEY=VALUE form: check var exists and matches
            match std::env::var(key) {
                Ok(v) if v == value => {}
                _ => return false,
            }
        } else {
            // KEY form: just check existence
            if std::env::var(env_cond).is_err() {
                return false;
            }
        }
    }

    true
}

/// Apply a specific choice to the command for a pending rule
pub fn apply_choice(command: &str, rule: &Rule, with_value: &str) -> Result<String, SwapxError> {
    do_replace(command, rule, with_value)
}

// --- Explain support ---

#[derive(Debug)]
pub struct ExplainReplacement {
    pub label: String,
    pub with_value: String,
    pub result_command: String,
    pub is_default: bool,
    pub when_condition: Option<WhenCondition>,
    pub when_matches: bool,
}

#[derive(Debug)]
pub struct ExplainMatch {
    pub rule: Rule,
    pub is_enabled: bool,
    pub replacements: Vec<ExplainReplacement>,
}

pub fn explain_rules(command: &str, rules: &[Rule]) -> Result<Vec<ExplainMatch>, SwapxError> {
    let mut matches = Vec::new();

    for rule in rules {
        let does_match = if rule.regex {
            let re = Regex::new(&rule.match_pattern)?;
            re.is_match(command)
        } else {
            command.contains(&rule.match_pattern)
        };

        if !does_match {
            continue;
        }

        let mut replacements = Vec::new();
        for repl in &rule.replace {
            let result_command = do_replace(command, rule, &repl.with_value)?;
            let when_matches = repl.when.as_ref().map(evaluate_when).unwrap_or(false);

            replacements.push(ExplainReplacement {
                label: repl.label.clone(),
                with_value: repl.with_value.clone(),
                result_command,
                is_default: repl.default,
                when_condition: repl.when.clone(),
                when_matches,
            });
        }

        matches.push(ExplainMatch {
            rule: rule.clone(),
            is_enabled: rule.enabled,
            replacements,
        });
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Replacement, Rule};

    fn make_rule(
        pattern: &str,
        regex: bool,
        enabled: bool,
        replacements: Vec<Replacement>,
    ) -> Rule {
        Rule {
            match_pattern: pattern.into(),
            regex,
            enabled,
            replace: replacements,
        }
    }

    fn make_repl(label: &str, with_value: &str, default: bool) -> Replacement {
        Replacement {
            label: label.into(),
            with_value: with_value.into(),
            default,
            when: None,
        }
    }

    #[test]
    fn literal_single_replacement() {
        let rules = vec![make_rule(
            "git@github.com:",
            false,
            true,
            vec![make_repl("personal", "git@github-personal:", false)],
        )];

        let result = apply_rules("git clone git@github.com:user/repo.git", &rules, false).unwrap();
        assert!(result.changed);
        assert_eq!(
            result.command,
            "git clone git@github-personal:user/repo.git"
        );
        assert!(result.pending_choices.is_empty());
    }

    #[test]
    fn regex_numbered_capture_groups() {
        let rules = vec![make_rule(
            r"docker run -p (\d+):(\d+)",
            true,
            true,
            vec![make_repl("swap-ports", "docker run -p $2:$1", false)],
        )];

        let result = apply_rules("docker run -p 8080:3000 myimage", &rules, false).unwrap();
        assert!(result.changed);
        assert_eq!(result.command, "docker run -p 3000:8080 myimage");
    }

    #[test]
    fn regex_named_capture_groups() {
        let rules = vec![make_rule(
            r"(?P<user>\w+)@(?P<host>\w+)",
            true,
            true,
            vec![make_repl("reverse", "${host}@${user}", false)],
        )];

        let result = apply_rules("ssh alice@server", &rules, false).unwrap();
        assert!(result.changed);
        assert_eq!(result.command, "ssh server@alice");
    }

    #[test]
    fn disabled_rule_skipping() {
        let rules = vec![make_rule(
            "git@github.com:",
            false,
            false, // disabled
            vec![make_repl("personal", "git@github-personal:", false)],
        )];

        let result = apply_rules("git clone git@github.com:user/repo.git", &rules, false).unwrap();
        assert!(!result.changed);
        assert_eq!(result.command, "git clone git@github.com:user/repo.git");
    }

    #[test]
    fn multi_replacement_with_default() {
        let rules = vec![make_rule(
            "git@github.com:",
            false,
            true,
            vec![
                make_repl("personal", "git@github-personal:", true),
                make_repl("work", "git@github-work:", false),
            ],
        )];

        // With use_defaults=true, should auto-apply the default
        let result = apply_rules("git clone git@github.com:user/repo.git", &rules, true).unwrap();
        assert!(result.changed);
        assert_eq!(
            result.command,
            "git clone git@github-personal:user/repo.git"
        );
        assert!(result.pending_choices.is_empty());
    }

    #[test]
    fn multi_replacement_without_default_is_pending() {
        let rules = vec![make_rule(
            "git@github.com:",
            false,
            true,
            vec![
                make_repl("personal", "git@github-personal:", false),
                make_repl("work", "git@github-work:", false),
            ],
        )];

        let result = apply_rules("git clone git@github.com:user/repo.git", &rules, false).unwrap();
        assert!(!result.changed);
        assert_eq!(result.pending_choices.len(), 1);
    }

    #[test]
    fn no_match_passthrough() {
        let rules = vec![make_rule(
            "git@github.com:",
            false,
            true,
            vec![make_repl("personal", "git@github-personal:", false)],
        )];

        let result = apply_rules("echo hello world", &rules, false).unwrap();
        assert!(!result.changed);
        assert_eq!(result.command, "echo hello world");
        assert!(result.pending_choices.is_empty());
    }

    #[test]
    fn docker_port_swap_example() {
        // Matches the init config example
        let rules = vec![make_rule(
            r"docker run -p (\d+):(\d+)",
            true,
            true,
            vec![make_repl("swap-ports", "docker run -p $2:$1", false)],
        )];

        let result = apply_rules("docker run -p 80:443 nginx", &rules, false).unwrap();
        assert!(result.changed);
        assert_eq!(result.command, "docker run -p 443:80 nginx");
    }
}
