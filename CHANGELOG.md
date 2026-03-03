# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `--cmd <COMMAND>` flag — pass a command string directly, preserving stdin as TTY for interactive prompts
- `--list-choices` flag — output pending choices as tab-separated lines (exit 20) for shell hooks to parse
- `--choice <INDICES>` flag — apply comma-separated 0-based choice indices selected by the user
- Two-phase interactive selection protocol — shell hooks show native numbered menus when a rule has multiple replacements with no default or matching `when` condition
- Exit code 10 — signals the user made an interactive selection via dialoguer (direct TTY usage)
- Exit code 20 — signals pending choices that the caller must resolve (used by `--list-choices`)

### Changed

- Shell hooks now use a two-phase protocol: `--list-choices` to detect pending choices, then `--choice` to apply the user's selection via shell-native numbered menus
- Shell hooks read from `/dev/tty` in zsh/bash to avoid buffered stdin issues inside widget/trap contexts

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
