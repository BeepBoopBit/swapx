use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "swapx", about = "Command rewriter CLI tool")]
pub struct Cli {
    /// Show the transformed command without executing
    #[arg(long)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a new replacement rule
    Add,

    /// List all active rules
    List,

    /// Create a .swapx.yaml in the current directory
    Init,

    /// Enable a rule by its match pattern
    Enable {
        /// The match pattern of the rule to enable
        pattern: String,
    },

    /// Disable a rule by its match pattern
    Disable {
        /// The match pattern of the rule to disable
        pattern: String,
    },

    /// Generate shell hook script for transparent command interception
    ShellHook {
        /// Shell type: "zsh", "bash", "fish", "powershell", or "nu" (auto-detected if omitted)
        shell: Option<String>,
    },

    /// Explain which rules match a command and what they'd produce
    Explain {
        /// The command to explain
        #[arg(trailing_var_arg = true, num_args = 1..)]
        command: Vec<String>,
    },

    /// Pass-through: any command not matching a built-in subcommand
    #[command(external_subcommand)]
    External(Vec<String>),
}
