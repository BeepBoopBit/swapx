mod cli;
mod config;
mod engine;
mod error;
mod executor;
mod interactive;
mod models;
mod shell_hook;

use std::io::{self, IsTerminal, Read};
use std::process;

use clap::Parser;

use cli::{Cli, Commands};
use error::SwapxError;

fn run() -> Result<i32, SwapxError> {
    let cli = Cli::parse();
    let is_tty = io::stdin().is_terminal();

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
