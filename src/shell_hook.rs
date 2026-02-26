use crate::error::SwapxError;

const ZSH_HOOK: &str = r#"
# swapx shell integration for zsh
# Add to .zshrc: eval "$(swapx shell-hook zsh)"

__swapx_accept_line() {
    # Skip empty commands and swapx commands to prevent infinite loops
    if [[ -z "$BUFFER" ]] || [[ "$BUFFER" == swapx* ]]; then
        zle .accept-line
        return
    fi

    local original="$BUFFER"
    local transformed
    transformed=$(echo "$BUFFER" | swapx 2>/dev/null)

    if [[ $? -eq 0 ]] && [[ "$transformed" != "$BUFFER" ]]; then
        if [[ "${SWAPX_AUTO_APPLY:-0}" == "1" ]]; then
            zle -I
            echo "swapx: $BUFFER"
            echo "    → $transformed"
            BUFFER="$transformed"
        else
            zle -I
            echo "swapx: $BUFFER"
            echo "    → $transformed"
            local reply
            print -n "Apply? [Y/n] "
            read -rk1 reply
            echo ""
            # Accept on: Enter (newline/CR), empty, y, Y — reject only on explicit n/N
            if [[ "$reply" == [nN] ]]; then
                BUFFER="$original"
            else
                BUFFER="$transformed"
            fi
        fi
    fi

    zle .accept-line
}

zle -N accept-line __swapx_accept_line
"#;

const BASH_HOOK: &str = r#"
# swapx shell integration for bash
# Add to .bashrc: eval "$(swapx shell-hook bash)"

__swapx_debug_trap() {
    # Skip if in completion, prompt command, or subshell
    [[ -n "${COMP_LINE:-}" ]] && return 0
    [[ "${BASH_COMMAND}" == "${PROMPT_COMMAND:-}" ]] && return 0
    [[ "${BASH_SUBSHELL}" -gt 0 ]] && return 0

    # Skip swapx commands to prevent infinite loops
    [[ "${BASH_COMMAND}" == swapx* ]] && return 0
    [[ "${BASH_COMMAND}" == __swapx* ]] && return 0

    local transformed
    transformed=$(echo "${BASH_COMMAND}" | swapx 2>/dev/null)

    if [[ $? -eq 0 ]] && [[ "${transformed}" != "${BASH_COMMAND}" ]]; then
        if [[ "${SWAPX_AUTO_APPLY:-0}" == "1" ]]; then
            echo "swapx: ${BASH_COMMAND}"
            echo "    → ${transformed}"
            eval "${transformed}"
            return 1
        else
            echo "swapx: ${BASH_COMMAND}"
            echo "    → ${transformed}"
            read -p "Apply? [Y/n] " -n1 reply
            echo ""
            if [[ "${reply}" == "" ]] || [[ "${reply}" == "y" ]] || [[ "${reply}" == "Y" ]]; then
                eval "${transformed}"
                return 1
            fi
        fi
    fi

    return 0
}

shopt -s extdebug
trap '__swapx_debug_trap' DEBUG
"#;

const FISH_HOOK: &str = r#"
# swapx shell integration for fish
# Add to ~/.config/fish/config.fish: swapx shell-hook fish | source

function __swapx_enter
    set -l cmd (commandline)

    # Skip empty commands and swapx commands to prevent infinite loops
    if test -z "$cmd"; or string match -q 'swapx*' -- $cmd
        commandline -f execute
        return
    end

    set -l transformed (printf '%s\n' $cmd | swapx 2>/dev/null)
    set -l swapx_status $status

    if test $swapx_status -eq 0; and test "$transformed" != "$cmd"
        if test "$SWAPX_AUTO_APPLY" = 1
            echo ""
            echo "swapx: $cmd"
            echo "    → $transformed"
            commandline -r -- $transformed
        else
            echo ""
            echo "swapx: $cmd"
            echo "    → $transformed"
            read -l -n 1 -P "Apply? [Y/n] " reply
            if test -z "$reply"; or test "$reply" = y; or test "$reply" = Y
                commandline -r -- $transformed
            end
        end
    end

    commandline -f execute
end

bind \r __swapx_enter
bind \n __swapx_enter
"#;

const POWERSHELL_HOOK: &str = r#"
# swapx shell integration for PowerShell
# Add to $PROFILE: Invoke-Expression (swapx shell-hook powershell)

if (Get-Module -Name PSReadLine) {
    Set-PSReadLineKeyHandler -Key Enter -ScriptBlock {
        $line = $null
        $cursor = $null
        [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)

        # Skip empty commands and swapx commands to prevent infinite loops
        if ([string]::IsNullOrWhiteSpace($line) -or $line -match '^swapx') {
            [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
            return
        }

        try {
            $transformed = $line | swapx 2>$null
            if ($LASTEXITCODE -eq 0 -and $transformed -ne $line) {
                if ($env:SWAPX_AUTO_APPLY -eq '1') {
                    Write-Host ""
                    Write-Host "swapx: $line"
                    Write-Host "    → $transformed"
                    [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
                    [Microsoft.PowerShell.PSConsoleReadLine]::Insert($transformed)
                } else {
                    Write-Host ""
                    Write-Host "swapx: $line"
                    Write-Host "    → $transformed"
                    $reply = Read-Host "Apply? [Y/n]"
                    if ($reply -eq '' -or $reply -eq 'y' -or $reply -eq 'Y') {
                        [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
                        [Microsoft.PowerShell.PSConsoleReadLine]::Insert($transformed)
                    }
                }
            }
        } catch {
            # swapx not found or errored — fall through to normal execution
        }

        [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
    }
}
"#;

const NUSHELL_HOOK: &str = r#"
# swapx shell integration for nushell
#
# Step 1: Save the function — run this command, or add it to your env.nu:
#   swapx shell-hook nu | save -f ~/.config/nushell/swapx.nu
#
# Step 2: Source it in your config.nu:
#   source ~/.config/nushell/swapx.nu
#
# Step 3: Add this keybinding to your config.nu inside $env.config.keybindings:
#   {
#     name: swapx_enter
#     modifier: none
#     keycode: enter
#     mode: [emacs vi_insert vi_normal]
#     event: [
#       { edit: Clear }
#       { send: ExecuteHostCommand, cmd: "__swapx_handler" }
#     ]
#   }

def __swapx_handler [] {
    let cmd = (commandline)

    # Skip empty commands and swapx commands to prevent infinite loops
    if ($cmd | str trim | is-empty) or ($cmd | str starts-with "swapx") {
        commandline edit --replace $cmd
        commandline execute
        return
    }

    let result = try { $cmd | ^swapx } catch { "" }
    let transformed = ($result | str trim)

    if ($transformed | is-empty) or ($transformed == $cmd) {
        commandline edit --replace $cmd
        commandline execute
        return
    }

    let auto_apply = ($env | get -i SWAPX_AUTO_APPLY | default "0")
    if $auto_apply == "1" {
        print $"\nswapx: ($cmd)"
        print $"    → ($transformed)"
        commandline edit --replace $transformed
    } else {
        print $"\nswapx: ($cmd)"
        print $"    → ($transformed)"
        let reply = (input "Apply? [Y/n] ")
        if ($reply | is-empty) or $reply == "y" or $reply == "Y" {
            commandline edit --replace $transformed
        } else {
            commandline edit --replace $cmd
        }
    }

    commandline execute
}
"#;

pub fn detect_shell() -> Option<String> {
    // Check for nushell first via its unique env var
    if std::env::var("NU_VERSION").is_ok() {
        return Some("nu".to_string());
    }

    // Check $SHELL (works on Unix and some Windows setups like Git Bash)
    if let Ok(s) = std::env::var("SHELL") {
        // Handle both Unix (/) and Windows (\) path separators
        let name = s.rsplit(['/', '\\']).next()?;
        return match name {
            "zsh" | "bash" | "fish" | "nu" => Some(name.to_string()),
            _ => None,
        };
    }

    // On Windows, detect PowerShell via PSModulePath when $SHELL is not set
    #[cfg(windows)]
    if std::env::var("PSModulePath").is_ok() {
        return Some("powershell".to_string());
    }

    None
}

pub fn generate_hook(shell: &str) -> Result<String, SwapxError> {
    match shell {
        "zsh" => Ok(ZSH_HOOK.to_string()),
        "bash" => Ok(BASH_HOOK.to_string()),
        "fish" => Ok(FISH_HOOK.to_string()),
        "powershell" | "pwsh" => Ok(POWERSHELL_HOOK.to_string()),
        "nu" | "nushell" => Ok(NUSHELL_HOOK.to_string()),
        _ => Err(SwapxError::Config(format!(
            "Unsupported shell: \"{}\". Supported shells: zsh, bash, fish, powershell, nu",
            shell
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_hook_zsh() {
        let hook = generate_hook("zsh").unwrap();
        assert!(hook.contains("__swapx_accept_line"));
        assert!(hook.contains("zle -N accept-line"));
    }

    #[test]
    fn test_generate_hook_bash() {
        let hook = generate_hook("bash").unwrap();
        assert!(hook.contains("__swapx_debug_trap"));
        assert!(hook.contains("shopt -s extdebug"));
    }

    #[test]
    fn test_generate_hook_fish() {
        let hook = generate_hook("fish").unwrap();
        assert!(hook.contains("__swapx_enter"));
        assert!(hook.contains("bind \\r __swapx_enter"));
        assert!(hook.contains("commandline"));
    }

    #[test]
    fn test_generate_hook_powershell() {
        let hook = generate_hook("powershell").unwrap();
        assert!(hook.contains("Set-PSReadLineKeyHandler"));
        assert!(hook.contains("GetBufferState"));
    }

    #[test]
    fn test_generate_hook_pwsh_alias() {
        let powershell_hook = generate_hook("powershell").unwrap();
        let pwsh_hook = generate_hook("pwsh").unwrap();
        assert_eq!(powershell_hook, pwsh_hook);
    }

    #[test]
    fn test_generate_hook_nu() {
        let hook = generate_hook("nu").unwrap();
        assert!(hook.contains("__swapx_handler"));
        assert!(hook.contains("commandline edit --replace"));
    }

    #[test]
    fn test_generate_hook_nushell_alias() {
        let nu_hook = generate_hook("nu").unwrap();
        let nushell_hook = generate_hook("nushell").unwrap();
        assert_eq!(nu_hook, nushell_hook);
    }

    #[test]
    fn test_generate_hook_unsupported() {
        let err = generate_hook("tcsh").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("tcsh"));
        assert!(msg.contains("zsh"));
        assert!(msg.contains("bash"));
        assert!(msg.contains("fish"));
        assert!(msg.contains("powershell"));
        assert!(msg.contains("nu"));
    }
}
