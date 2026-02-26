use std::process::Command;

use crate::error::SwapxError;

/// Resolve the shell binary and arguments needed to execute a command string.
fn resolve_shell_command(cmd_str: &str) -> (String, Vec<String>) {
    #[cfg(unix)]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        (shell, vec!["-c".into(), cmd_str.into()])
    }

    #[cfg(windows)]
    {
        // Nushell
        if std::env::var("NU_VERSION").is_ok() {
            return ("nu".into(), vec!["-c".into(), cmd_str.into()]);
        }
        // Git Bash or similar (SHELL is set)
        if let Ok(shell) = std::env::var("SHELL") {
            return (shell, vec!["-c".into(), cmd_str.into()]);
        }
        // PowerShell — prefer pwsh (7+, cross-platform) over powershell.exe (5.1)
        if std::env::var("PSModulePath").is_ok() {
            if which_exists("pwsh") {
                return ("pwsh".into(), vec!["-Command".into(), cmd_str.into()]);
            }
            return (
                "powershell.exe".into(),
                vec!["-Command".into(), cmd_str.into()],
            );
        }
        // Fallback to cmd.exe
        ("cmd.exe".into(), vec!["/C".into(), cmd_str.into()])
    }
}

/// Check if a command exists on PATH (Windows only).
#[cfg(windows)]
fn which_exists(name: &str) -> bool {
    Command::new("where")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn execute_command(cmd_str: &str) -> Result<i32, SwapxError> {
    let (program, args) = resolve_shell_command(cmd_str);

    let status = Command::new(&program).args(&args).status()?;

    Ok(status.code().unwrap_or(1))
}
