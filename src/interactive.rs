use dialoguer::{Confirm, Input, Select};

use crate::config;
use crate::engine::{self, ExplainMatch, PendingChoice};
use crate::error::SwapxError;
use crate::executor;
use crate::models::{ConfigFile, Replacement, Rule};

pub fn prompt_choice(pending: &PendingChoice) -> Result<usize, SwapxError> {
    let labels: Vec<&str> = pending
        .rule
        .replace
        .iter()
        .map(|r| r.label.as_str())
        .collect();

    let default_idx = pending
        .rule
        .replace
        .iter()
        .position(|r| r.default)
        .unwrap_or(0);

    let selection = Select::new()
        .with_prompt(format!(
            "Choose replacement for '{}'",
            pending.matched_pattern
        ))
        .items(&labels)
        .default(default_idx)
        .interact()?;

    Ok(selection)
}

pub fn resolve_pending_choices(
    command: &str,
    pending_choices: Vec<PendingChoice>,
) -> Result<String, SwapxError> {
    let mut result = command.to_string();

    for pending in &pending_choices {
        let idx = prompt_choice(pending)?;
        let with_value = &pending.rule.replace[idx].with_value;
        result = engine::apply_choice(&result, &pending.matched_pattern, pending.rule.regex, with_value)?;
    }

    Ok(result)
}

pub fn interactive_mode(config: &ConfigFile) -> Result<i32, SwapxError> {
    loop {
        let input: String = Input::new().with_prompt("swapx>").interact_text()?;

        let input = input.trim();
        if input.is_empty() || input == "exit" || input == "quit" {
            break;
        }

        let transform = engine::apply_rules(input, &config.rules, false)?;
        let mut command = transform.command;

        if !transform.pending_choices.is_empty() {
            command = resolve_pending_choices(&command, transform.pending_choices)?;
        }

        if command != input {
            eprintln!("  → {}", command);
        }

        let confirm = Confirm::new()
            .with_prompt("Execute?")
            .default(true)
            .interact()?;

        if confirm {
            let code = executor::execute_command(&command)?;
            if code != 0 {
                eprintln!("Command exited with code {}", code);
            }
        }
    }

    Ok(0)
}

pub fn add_rule_wizard() -> Result<(), SwapxError> {
    let match_pattern: String = Input::new()
        .with_prompt("Match pattern (literal string or regex)")
        .interact_text()?;

    let is_regex = Confirm::new()
        .with_prompt("Is this a regex pattern?")
        .default(false)
        .interact()?;

    let mut replacements = Vec::new();

    loop {
        let label: String = Input::new()
            .with_prompt("Replacement label (e.g. 'personal')")
            .interact_text()?;

        let with_value: String = Input::new().with_prompt("Replace with").interact_text()?;

        let is_default = if replacements.is_empty() {
            Confirm::new()
                .with_prompt("Set as default?")
                .default(true)
                .interact()?
        } else {
            Confirm::new()
                .with_prompt("Set as default?")
                .default(false)
                .interact()?
        };

        replacements.push(Replacement {
            label,
            with_value,
            default: is_default,
            when: None,
        });

        let add_more = Confirm::new()
            .with_prompt("Add another replacement option?")
            .default(false)
            .interact()?;

        if !add_more {
            break;
        }
    }

    let save_local = Confirm::new()
        .with_prompt("Save to local .swapx.yaml? (No = global config)")
        .default(false)
        .interact()?;

    let rule = Rule {
        match_patterns: vec![match_pattern],
        regex: is_regex,
        enabled: true,
        dir: None,
        replace: replacements,
    };

    let path = config::save_rule(rule, save_local)?;
    eprintln!("Rule saved to {}", path.display());

    Ok(())
}

pub fn list_rules(config: &ConfigFile) {
    if config.rules.is_empty() {
        eprintln!("No rules configured.");
        eprintln!("Run `swapx init` to create a local config or `swapx add` to add a rule.");
        return;
    }

    for (i, rule) in config.rules.iter().enumerate() {
        let kind = if rule.regex { "regex" } else { "literal" };
        let disabled = if !rule.enabled { " [DISABLED]" } else { "" };
        let patterns_display: Vec<String> = rule
            .match_patterns
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect();
        eprintln!(
            "{}. [{}] match: {}{}",
            i + 1,
            kind,
            patterns_display.join(", "),
            disabled
        );
        if let Some(ref dir) = rule.dir {
            eprintln!("     dir: \"{}\"", dir);
        }
        for repl in &rule.replace {
            let default_marker = if repl.default { " (default)" } else { "" };
            let when_marker = if let Some(ref when) = repl.when {
                let mut parts = Vec::new();
                if let Some(ref cwd) = when.cwd {
                    parts.push(format!("cwd={}", cwd));
                }
                if let Some(ref env) = when.env {
                    parts.push(format!("env={}", env));
                }
                format!(" [when: {}]", parts.join(", "))
            } else {
                String::new()
            };
            eprintln!(
                "     → {}: \"{}\"{}{}",
                repl.label, repl.with_value, default_marker, when_marker
            );
        }
    }
}

pub fn display_explain(command: &str, matches: &[ExplainMatch]) {
    eprintln!("Command: {}", command);
    eprintln!();

    if matches.is_empty() {
        eprintln!("No rules match this command.");
        return;
    }

    for (i, m) in matches.iter().enumerate() {
        let kind = if m.rule.regex { "regex" } else { "literal" };
        let status = if m.is_enabled { "enabled" } else { "DISABLED" };
        let patterns_display: Vec<String> = m
            .rule
            .match_patterns
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect();
        eprintln!(
            "Rule {}: [{}] [{}] match: {} (matched: \"{}\")",
            i + 1,
            kind,
            status,
            patterns_display.join(", "),
            m.matched_pattern
        );
        if let Some(ref dir) = m.rule.dir {
            let dir_status = match m.dir_matches {
                Some(true) => "MATCHES",
                Some(false) => "no match",
                None => "",
            };
            eprintln!("  dir: \"{}\" → {}", dir, dir_status);
        }

        for repl in &m.replacements {
            let default_marker = if repl.is_default { " (default)" } else { "" };
            let when_status = if let Some(ref when) = repl.when_condition {
                let mut parts = Vec::new();
                if let Some(ref cwd) = when.cwd {
                    parts.push(format!("cwd={}", cwd));
                }
                if let Some(ref env) = when.env {
                    parts.push(format!("env={}", env));
                }
                let matched = if repl.when_matches {
                    "MATCHES"
                } else {
                    "no match"
                };
                format!(" [when: {} → {}]", parts.join(", "), matched)
            } else {
                String::new()
            };

            eprintln!(
                "  → {}: \"{}\" → \"{}\"{}{}",
                repl.label, repl.with_value, repl.result_command, default_marker, when_status
            );
        }
        eprintln!();
    }
}
