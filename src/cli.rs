use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "swapx", about = "Command rewriter CLI tool", version)]
pub struct Cli {
    /// Show the transformed command without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Pass a command string directly (preserves stdin as TTY for interactive prompts)
    #[arg(long)]
    pub cmd: Option<String>,

    /// Output pending choices as tab-separated lines (exit 20 if choices pending, 0 otherwise)
    #[arg(long)]
    pub list_choices: bool,

    /// Apply comma-separated 0-based choice indices for pending rules (implies dry-run)
    #[arg(long)]
    pub choice: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a new replacement rule
    Add,

    /// List all active rules
    List,

    /// Initialize global swapx config and install builtin suggestion packs
    Init {
        /// Overwrite existing files without prompting
        #[arg(long)]
        force: bool,
    },

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

    /// Reset all swapx configuration (global, local, and suggestion packs)
    Reset,

    /// Scan system and suggest rules based on installed tools and project files
    Suggest {
        /// Show suggestions without saving
        #[arg(long)]
        check: bool,
        /// Accept all suggestions with defaults, no prompts
        #[arg(long)]
        auto: bool,
    },

    /// Pass-through: any command not matching a built-in subcommand
    #[command(external_subcommand)]
    External(Vec<String>),
}
