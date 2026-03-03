use crate::error::SwapxError;

const ZSH_HOOK: &str = r##"
# swapx shell integration for zsh
# Add to .zshrc: eval "$(swapx shell-hook zsh)"

__swapx_accept_line() {
    # Skip empty commands and swapx commands to prevent infinite loops
    if [[ -z "$BUFFER" ]] || [[ "$BUFFER" == swapx* ]]; then
        zle .accept-line
        return
    fi

    local original="$BUFFER"
    local result swapx_exit
    result=$(swapx --dry-run --cmd "$BUFFER" --list-choices)
    swapx_exit=$?

    if [[ $swapx_exit -eq 20 ]]; then
        # Pending choices: show numbered menu for each
        zle -I
        local -a lines
        lines=("${(@f)result}")
        local choice_indices=""
        local i
        for (( i=2; i<=${#lines[@]}; i++ )); do
            local line="${lines[$i]}"
            local match_pattern="${line%%	*}"
            local rest="${line#*	}"
            local default_idx="${rest%%	*}"
            rest="${rest#*	}"
            local -a labels
            labels=("${(@s/	/)rest}")

            echo "Choose replacement for '${match_pattern}':"
            local j
            for (( j=1; j<=${#labels[@]}; j++ )); do
                local suffix=""
                if [[ $(( j - 1 )) -eq $default_idx ]]; then
                    suffix=" (default)"
                fi
                echo "  $j) ${labels[$j]}${suffix}"
            done

            local default_display=""
            if [[ $default_idx -ge 0 ]]; then
                default_display=$(( default_idx + 1 ))
            fi

            local reply
            if [[ -n "$default_display" ]]; then
                print -n "#? [$default_display] "
            else
                print -n "#? "
            fi
            read -r reply </dev/tty

            if [[ -z "$reply" ]] && [[ -n "$default_display" ]]; then
                reply="$default_display"
            fi

            # Validate reply is a positive integer; cancel on invalid input
            if ! [[ "$reply" =~ ^[0-9]+$ ]] || [[ "$reply" -eq 0 ]] || [[ "$reply" -gt ${#labels[@]} ]]; then
                BUFFER=""
                zle .accept-line
                return
            fi

            local zero_based=$(( reply - 1 ))
            if [[ -n "$choice_indices" ]]; then
                choice_indices="${choice_indices},${zero_based}"
            else
                choice_indices="${zero_based}"
            fi
        done

        local transformed
        transformed=$(swapx --dry-run --cmd "$BUFFER" --choice "$choice_indices")
        BUFFER="$transformed"
    elif [[ $swapx_exit -eq 0 ]] && [[ "$result" != "$BUFFER" ]]; then
        if [[ "${SWAPX_AUTO_APPLY:-0}" == "1" ]]; then
            zle -I
            echo "swapx: $BUFFER"
            echo "    → $result"
            BUFFER="$result"
        else
            zle -I
            echo "swapx: $BUFFER"
            echo "    → $result"
            local reply
            print -n "Apply? [Y/n] "
            read -rk1 reply
            echo ""
            if [[ "$reply" == [nN] ]]; then
                BUFFER="$original"
            else
                BUFFER="$result"
            fi
        fi
    fi

    zle .accept-line
}

zle -N accept-line __swapx_accept_line
"##;

const BASH_HOOK: &str = r##"
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

    local result swapx_exit
    result=$(swapx --dry-run --cmd "${BASH_COMMAND}" --list-choices)
    swapx_exit=$?

    if [[ $swapx_exit -eq 20 ]]; then
        # Pending choices: show numbered menu for each
        local IFS=$'\n'
        local -a lines
        mapfile -t lines <<< "$result"
        unset IFS
        local choice_indices=""
        local i
        for (( i=1; i<${#lines[@]}; i++ )); do
            local line="${lines[$i]}"
            local match_pattern="${line%%	*}"
            local rest="${line#*	}"
            local default_idx="${rest%%	*}"
            rest="${rest#*	}"

            local IFS=$'\t'
            local -a labels
            read -ra labels <<< "$rest"
            unset IFS

            echo "Choose replacement for '${match_pattern}':"
            local j
            for (( j=0; j<${#labels[@]}; j++ )); do
                local suffix=""
                if [[ $j -eq $default_idx ]]; then
                    suffix=" (default)"
                fi
                echo "  $(( j + 1 ))) ${labels[$j]}${suffix}"
            done

            local default_display=""
            if [[ $default_idx -ge 0 ]]; then
                default_display=$(( default_idx + 1 ))
            fi

            local reply
            if [[ -n "$default_display" ]]; then
                read -p "#? [$default_display] " reply </dev/tty
            else
                read -p "#? " reply </dev/tty
            fi

            if [[ -z "$reply" ]] && [[ -n "$default_display" ]]; then
                reply="$default_display"
            fi

            # Validate reply is a positive integer; cancel on invalid input
            if ! [[ "$reply" =~ ^[0-9]+$ ]] || [[ "$reply" -eq 0 ]] || [[ "$reply" -gt ${#labels[@]} ]]; then
                return 1
            fi

            local zero_based=$(( reply - 1 ))
            if [[ -n "$choice_indices" ]]; then
                choice_indices="${choice_indices},${zero_based}"
            else
                choice_indices="${zero_based}"
            fi
        done

        local transformed
        transformed=$(swapx --dry-run --cmd "${BASH_COMMAND}" --choice "$choice_indices")
        eval "${transformed}"
        return 1
    elif [[ $swapx_exit -eq 0 ]] && [[ "${result}" != "${BASH_COMMAND}" ]]; then
        if [[ "${SWAPX_AUTO_APPLY:-0}" == "1" ]]; then
            echo "swapx: ${BASH_COMMAND}"
            echo "    → ${result}"
            eval "${result}"
            return 1
        else
            echo "swapx: ${BASH_COMMAND}"
            echo "    → ${result}"
            read -p "Apply? [Y/n] " -n1 reply
            echo ""
            if [[ "${reply}" == "" ]] || [[ "${reply}" == "y" ]] || [[ "${reply}" == "Y" ]]; then
                eval "${result}"
                return 1
            fi
        fi
    fi

    return 0
}

shopt -s extdebug
trap '__swapx_debug_trap' DEBUG
"##;

const FISH_HOOK: &str = r##"
# swapx shell integration for fish
# Add to ~/.config/fish/config.fish: swapx shell-hook fish | source

function __swapx_enter
    set -l cmd (commandline)

    # Skip empty commands and swapx commands to prevent infinite loops
    if test -z "$cmd"; or string match -q 'swapx*' -- $cmd
        commandline -f execute
        return
    end

    set -l result (swapx --dry-run --cmd "$cmd" --list-choices)
    set -l swapx_status $status

    if test $swapx_status -eq 20
        # Pending choices: show numbered menu for each
        echo ""
        set -l choice_indices ""
        set -l line_count (count $result)
        for i in (seq 2 $line_count)
            set -l line $result[$i]
            set -l fields (string split \t -- $line)
            set -l match_pattern $fields[1]
            set -l default_idx $fields[2]
            set -l labels $fields[3..-1]

            echo "Choose replacement for '$match_pattern':"
            for j in (seq 1 (count $labels))
                set -l suffix ""
                if test (math "$j - 1") -eq "$default_idx"
                    set suffix " (default)"
                end
                echo "  $j) $labels[$j]$suffix"
            end

            set -l default_display ""
            if test "$default_idx" -ge 0
                set default_display (math "$default_idx + 1")
            end

            set -l reply
            if test -n "$default_display"
                read -l -P "#? [$default_display] " reply
            else
                read -l -P "#? " reply
            end

            if test -z "$reply"; and test -n "$default_display"
                set reply $default_display
            end

            # Validate reply is a positive integer in range; cancel on invalid input
            if not string match -qr '^[0-9]+$' -- "$reply"; or test "$reply" -eq 0; or test "$reply" -gt (count $labels)
                commandline -r -- ""
                commandline -f execute
                return
            end

            set -l zero_based (math "$reply - 1")
            if test -n "$choice_indices"
                set choice_indices "$choice_indices,$zero_based"
            else
                set choice_indices "$zero_based"
            end
        end

        set -l transformed (swapx --dry-run --cmd "$cmd" --choice "$choice_indices")
        commandline -r -- $transformed
    else if test $swapx_status -eq 0; and test "$result" != "$cmd"
        if test "$SWAPX_AUTO_APPLY" = 1
            echo ""
            echo "swapx: $cmd"
            echo "    → $result"
            commandline -r -- $result
        else
            echo ""
            echo "swapx: $cmd"
            echo "    → $result"
            read -l -n 1 -P "Apply? [Y/n] " reply
            if test -z "$reply"; or test "$reply" = y; or test "$reply" = Y
                commandline -r -- $result
            end
        end
    end

    commandline -f execute
end

bind \r __swapx_enter
bind \n __swapx_enter
"##;

const POWERSHELL_HOOK: &str = r##"
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
            $result = & swapx --dry-run --cmd $line --list-choices 2>$null
            $swapxExit = $LASTEXITCODE
            if ($swapxExit -eq 20) {
                # Pending choices: show numbered menu for each
                Write-Host ""
                $lines = $result -split "`n"
                $choiceIndices = ""
                for ($i = 1; $i -lt $lines.Count; $i++) {
                    $fields = $lines[$i] -split "`t"
                    $matchPattern = $fields[0]
                    $defaultIdx = [int]$fields[1]
                    $labels = $fields[2..($fields.Count - 1)]

                    Write-Host "Choose replacement for '${matchPattern}':"
                    for ($j = 0; $j -lt $labels.Count; $j++) {
                        $suffix = ""
                        if ($j -eq $defaultIdx) { $suffix = " (default)" }
                        Write-Host ("  {0}) {1}{2}" -f ($j + 1), $labels[$j], $suffix)
                    }

                    $defaultDisplay = ""
                    if ($defaultIdx -ge 0) { $defaultDisplay = $defaultIdx + 1 }

                    if ($defaultDisplay -ne "") {
                        $reply = Read-Host "#? [$defaultDisplay]"
                    } else {
                        $reply = Read-Host "#?"
                    }

                    if ([string]::IsNullOrEmpty($reply) -and $defaultDisplay -ne "") {
                        $reply = $defaultDisplay
                    }

                    # Validate reply is a positive integer in range
                    $replyInt = 0
                    if (-not [int]::TryParse($reply, [ref]$replyInt) -or $replyInt -le 0 -or $replyInt -gt $labels.Count) {
                        [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
                        return
                    }

                    $zeroBased = [int]$reply - 1
                    if ($choiceIndices -ne "") {
                        $choiceIndices = "${choiceIndices},${zeroBased}"
                    } else {
                        $choiceIndices = "$zeroBased"
                    }
                }

                $transformed = & swapx --dry-run --cmd $line --choice $choiceIndices
                [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
                [Microsoft.PowerShell.PSConsoleReadLine]::Insert($transformed)
            } elseif ($swapxExit -eq 0 -and $result -ne $line) {
                if ($env:SWAPX_AUTO_APPLY -eq '1') {
                    Write-Host ""
                    Write-Host "swapx: $line"
                    Write-Host "    → $result"
                    [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
                    [Microsoft.PowerShell.PSConsoleReadLine]::Insert($result)
                } else {
                    Write-Host ""
                    Write-Host "swapx: $line"
                    Write-Host "    → $result"
                    $reply = Read-Host "Apply? [Y/n]"
                    if ($reply -eq '' -or $reply -eq 'y' -or $reply -eq 'Y') {
                        [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
                        [Microsoft.PowerShell.PSConsoleReadLine]::Insert($result)
                    }
                }
            }
        } catch {
            # swapx not found or errored — fall through to normal execution
        }

        [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
    }
}
"##;

const NUSHELL_HOOK: &str = r##"
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

    let result = try { ^swapx --dry-run --cmd $cmd --list-choices | complete } catch { { stdout: "", exit_code: 1 } }
    let output = ($result.stdout | str trim)
    let swapx_exit = $result.exit_code

    if $swapx_exit == 20 {
        # Pending choices: show numbered menu for each
        let all_lines = ($output | split row "\n")
        mut choice_indices = ""
        for i in 1..(($all_lines | length) - 1) {
            let line = ($all_lines | get ($i))
            let fields = ($line | split row "\t")
            let match_pattern = ($fields | get 0)
            let default_idx = ($fields | get 1 | into int)
            let labels = ($fields | skip 2)

            print $"Choose replacement for '($match_pattern)':"
            for j in 0..(($labels | length) - 1) {
                let suffix = if $j == $default_idx { " (default)" } else { "" }
                print $"  ($j + 1)\) ($labels | get $j)($suffix)"
            end

            let default_display = if $default_idx >= 0 { $default_idx + 1 } else { -1 }

            let reply = if $default_display >= 0 {
                input $"#? [($default_display)] "
            } else {
                input "#? "
            }

            let reply = if ($reply | is-empty) and $default_display >= 0 {
                ($default_display | into string)
            } else {
                $reply
            }

            # Validate reply is a positive integer in range
            let reply_int = try { $reply | into int } catch { 0 }
            if $reply_int <= 0 or $reply_int > ($labels | length) {
                commandline edit --replace $cmd
                commandline execute
                return
            }

            let zero_based = ($reply_int - 1)
            $choice_indices = if ($choice_indices | is-empty) {
                ($zero_based | into string)
            } else {
                $"($choice_indices),($zero_based)"
            }
        }

        let transformed = (^swapx --dry-run --cmd $cmd --choice $choice_indices)
        commandline edit --replace $transformed
    } else if $swapx_exit == 0 and (not ($output | is-empty)) and ($output != $cmd) {
        let auto_apply = ($env | get -i SWAPX_AUTO_APPLY | default "0")
        if $auto_apply == "1" {
            print $"\nswapx: ($cmd)"
            print $"    → ($output)"
            commandline edit --replace $output
        } else {
            print $"\nswapx: ($cmd)"
            print $"    → ($output)"
            let reply = (input "Apply? [Y/n] ")
            if ($reply | is-empty) or $reply == "y" or $reply == "Y" {
                commandline edit --replace $output
            } else {
                commandline edit --replace $cmd
            }
        }
    } else {
        commandline edit --replace $cmd
    }

    commandline execute
}
"##;

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
