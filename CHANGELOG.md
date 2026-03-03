# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `--cmd <COMMAND>` flag — pass a command string directly, keeping stdin as the terminal so interactive prompts (dialoguer selectors) work from shell hooks
- Exit code 10 semantics — signals the user already made an interactive selection, so shell hooks auto-apply without double-prompting

### Changed

- Shell hooks now use `swapx --dry-run --cmd "$BUFFER"` instead of piping through `echo "$BUFFER" | swapx 2>/dev/null`, enabling interactive replacement selection directly in the shell
- Shell hooks no longer suppress stderr with `2>/dev/null`, allowing dialoguer prompts to display

### Fixed

- Shell hooks could not show interactive options when a rule had multiple replacements with no matching `when` condition or default — the pipe-based invocation forced non-interactive mode

## [0.1.0] - 2025-02-27

### Added

- Literal and regex matching with capture groups (`$1`, `$2`, `${name}`)
- Multiple replacement options per rule with interactive selection
- Directory-scoped auto-selection via `when` conditions (`cwd`, `env`)
- Global config (`~/.config/swapx/rules.yaml`) and local config (`.swapx.yaml`)
- Shell integration for zsh, bash, fish, PowerShell, and nushell
- `swapx init` — generate a starter config
- `swapx list` — display all loaded rules
- `swapx add` — interactive rule creation wizard
- `swapx enable` / `swapx disable` — toggle rules on and off
- `swapx explain` — debug which rules match and preview results
- `--dry-run` flag — preview transformations without executing
- Pipe mode — use swapx as a filter in pipelines
- Interactive mode — REPL-style command entry

[Unreleased]: https://github.com/BeepBoopBit/swapx/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/BeepBoopBit/swapx/releases/tag/v0.1.0
