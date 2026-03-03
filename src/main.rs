mod cli;
mod config;
mod engine;
mod error;
mod executor;
mod interactive;
mod models;
mod shell_hook;
mod suggest;

use std::io::{self, IsTerminal, Read};
use std::process;

use clap::Parser;

use cli::{Cli, Commands};
use error::SwapxError;

/// Exit code signaling the user already made an interactive selection via dialoguer.
/// Shell hooks see this and auto-apply without double-prompting.
const EXIT_INTERACTIVE_CHOICE: i32 = 10;

/// Exit code signaling that there are pending choices the caller must resolve.
/// Used by --list-choices when unresolved multi-replacement rules exist.
const EXIT_PENDING_CHOICES: i32 = 20;

fn handle_list_choices(cmd_str: &str) -> Result<i32, SwapxError> {
    let config = config::load_merged_config()?;
    // use_defaults=false so we get pending choices instead of auto-resolving defaults
    let transform = engine::apply_rules(cmd_str, &config.rules, false)?;

    if transform.pending_choices.is_empty() {
        // No pending choices: output fully transformed command
        println!("{}", transform.command);
        Ok(0)
    } else {
        // Output partially transformed command on line 1
        println!("{}", transform.command);
        // Lines 2+: one per pending choice
        for pc in &transform.pending_choices {
            let default_index = pc
                .rule
                .replace
                .iter()
                .position(|r| r.default)
                .map(|i| i as i32)
                .unwrap_or(-1);
            let labels: Vec<&str> = pc.rule.replace.iter().map(|r| r.label.as_str()).collect();
            // Format: MATCH_PATTERN\tDEFAULT_INDEX\tLABEL1\tLABEL2\t...
            print!("{}\t{}", pc.matched_pattern, default_index);
            for label in &labels {
                print!("\t{}", label);
            }
            println!();
        }
        Ok(EXIT_PENDING_CHOICES)
    }
}

fn parse_choice_indices(choice_str: &str) -> Result<Vec<usize>, SwapxError> {
    choice_str
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<usize>()
                .map_err(|_| SwapxError::Config(format!("invalid choice index: {:?}", s.trim())))
        })
        .collect()
}

fn handle_choice(cmd_str: &str, choice_str: &str) -> Result<i32, SwapxError> {
    let config = config::load_merged_config()?;
    // use_defaults=false so we get the same pending choices as --list-choices
    let transform = engine::apply_rules(cmd_str, &config.rules, false)?;
    let indices = parse_choice_indices(choice_str)?;

    if indices.len() != transform.pending_choices.len() {
        return Err(SwapxError::Config(format!(
            "expected {} choice index(es) but got {}",
            transform.pending_choices.len(),
            indices.len()
        )));
    }

    let mut result = transform.command;
    for (pc, &idx) in transform.pending_choices.iter().zip(indices.iter()) {
        if idx >= pc.rule.replace.len() {
            return Err(SwapxError::Config(format!(
                "choice index {} out of range for rule {:?} (has {} option(s))",
                idx,
                pc.matched_pattern,
                pc.rule.replace.len()
            )));
        }
        result = engine::apply_choice(
            &result,
            &pc.matched_pattern,
            pc.rule.regex,
            &pc.rule.replace[idx].with_value,
        )?;
    }

    println!("{}", result);
    Ok(0)
}

fn handle_cmd_flag(cmd_str: &str, dry_run: bool, is_tty: bool) -> Result<i32, SwapxError> {
    let config = config::load_merged_config()?;
    let use_defaults = !is_tty;
    let transform = engine::apply_rules(cmd_str, &config.rules, use_defaults)?;

    let mut final_command = transform.command;
    let mut interactive_selection = false;

    if !transform.pending_choices.is_empty() {
        if is_tty {
            final_command =
                interactive::resolve_pending_choices(&final_command, transform.pending_choices)?;
            interactive_selection = true;
        } else {
            // Non-interactive with unresolved choices: pass through as-is
            println!("{}", final_command);
            return Ok(0);
        }
    }

    if dry_run {
        println!("{}", final_command);
        if interactive_selection {
            Ok(EXIT_INTERACTIVE_CHOICE)
        } else {
            Ok(0)
        }
    } else {
        if transform.changed || final_command != cmd_str {
            eprintln!("swapx: → {}", final_command);
        }
        executor::execute_command(&final_command)
    }
}

fn run() -> Result<i32, SwapxError> {
    let cli = Cli::parse();
    let is_tty = io::stdin().is_terminal();

    // Validate --list-choices and --choice mutual exclusion and --cmd requirement
    if cli.list_choices && cli.choice.is_some() {
        return Err(SwapxError::Config(
            "--list-choices and --choice are mutually exclusive".into(),
        ));
    }
    if cli.list_choices && cli.cmd.is_none() {
        return Err(SwapxError::Config("--list-choices requires --cmd".into()));
    }
    if cli.choice.is_some() && cli.cmd.is_none() {
        return Err(SwapxError::Config("--choice requires --cmd".into()));
    }
    if (cli.list_choices || cli.choice.is_some()) && cli.command.is_some() {
        return Err(SwapxError::Config(
            "--list-choices and --choice cannot be combined with a subcommand".into(),
        ));
    }

    // Handle --cmd flag before subcommands
    if let Some(ref cmd_str) = cli.cmd {
        if cli.command.is_some() {
            return Err(SwapxError::Config(
                "--cmd cannot be combined with a subcommand".into(),
            ));
        }
        if cli.list_choices {
            return handle_list_choices(cmd_str);
        }
        if let Some(ref choice_str) = cli.choice {
            return handle_choice(cmd_str, choice_str);
        }
        return handle_cmd_flag(cmd_str, cli.dry_run, is_tty);
    }

    match cli.command {
        None => {
            if is_tty {
                // Interactive mode
                let config = config::load_merged_config()?;
                interactive::interactive_mode(&config)
            } else {
                // Pipe mode: read from stdin, transform, print
                let mut input = String::new();
                io::stdin().read_to_string(&mut input)?;
                let config = config::load_merged_config()?;

                for line in input.lines() {
                    let transform = engine::apply_rules(line, &config.rules, true)?;
                    println!("{}", transform.command);
                }
                Ok(0)
            }
        }

        Some(Commands::Add) => {
            interactive::add_rule_wizard()?;
            Ok(0)
        }

        Some(Commands::List) => {
            let config = config::load_merged_config()?;
            interactive::list_rules(&config);
            Ok(0)
        }

        Some(Commands::Init) => {
            let path = config::init_local_config()?;
            eprintln!("Created {}", path.display());
            Ok(0)
        }

        Some(Commands::Enable { pattern }) => {
            let path = config::toggle_rule(&pattern, true)?;
            eprintln!("Enabled rule \"{}\" in {}", pattern, path.display());
            Ok(0)
        }

        Some(Commands::Disable { pattern }) => {
            let path = config::toggle_rule(&pattern, false)?;
            eprintln!("Disabled rule \"{}\" in {}", pattern, path.display());
            Ok(0)
        }

        Some(Commands::ShellHook { shell }) => {
            let shell_name = shell
                .or_else(shell_hook::detect_shell)
                .ok_or_else(|| {
                    SwapxError::Config(
                        "Could not detect shell. Please specify: swapx shell-hook [zsh|bash|fish|powershell|nu]".into(),
                    )
                })?;
            let hook = shell_hook::generate_hook(&shell_name)?;
            print!("{}", hook);
            Ok(0)
        }

        Some(Commands::Suggest { check, auto }) => {
            suggest::run_suggest(check, auto)?;
            Ok(0)
        }

        Some(Commands::Explain { command }) => {
            let command_str = command.join(" ");
            let config = config::load_merged_config()?;
            let matches = engine::explain_rules(&command_str, &config.rules)?;
            interactive::display_explain(&command_str, &matches);
            Ok(0)
        }

        Some(Commands::External(args)) => {
            let command_str = shell_words_join(&args);
            let config = config::load_merged_config()?;
            let use_defaults = !is_tty;
            let transform = engine::apply_rules(&command_str, &config.rules, use_defaults)?;

            let mut final_command = transform.command;

            if !transform.pending_choices.is_empty() {
                if is_tty {
                    final_command = interactive::resolve_pending_choices(
                        &final_command,
                        transform.pending_choices,
                    )?;
                } else {
                    // Non-interactive with unresolved choices: output as-is
                    println!("{}", final_command);
                    return Ok(0);
                }
            }

            if cli.dry_run {
                println!("{}", final_command);
                Ok(0)
            } else {
                if transform.changed || final_command != command_str {
                    eprintln!("swapx: → {}", final_command);
                }
                executor::execute_command(&final_command)
            }
        }
    }
}

/// Join args into a shell command string, quoting as needed
fn shell_words_join(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg.contains(' ')
                || arg.contains('\'')
                || arg.contains('"')
                || arg.contains('\\')
                || arg.contains('$')
                || arg.contains('`')
                || arg.contains('|')
                || arg.contains('&')
                || arg.contains(';')
                || arg.contains('(')
                || arg.contains(')')
                || arg.contains('<')
                || arg.contains('>')
                || arg.contains('*')
                || arg.contains('?')
                || arg.contains('[')
                || arg.contains(']')
                || arg.contains('{')
                || arg.contains('}')
                || arg.contains('~')
                || arg.contains('#')
                || arg.contains('!')
                || arg.is_empty()
            {
                format!("'{}'", arg.replace('\'', "'\\''"))
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("swapx: error: {}", e);
            process::exit(1);
        }
    }
}
