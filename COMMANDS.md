# Command Reference

## Global flags

### `--dry-run`

Show the transformed command without executing it. Works with any external command or `--cmd`.

```sh
swapx --dry-run git clone git@github.com:user/repo.git
# prints: git clone git@github-personal:user/repo.git
```

### `--cmd <COMMAND>`

Pass a command string directly instead of using positional arguments or stdin. This keeps stdin connected to the terminal, enabling interactive prompts when a rule has multiple replacement options.

```sh
swapx --cmd "git clone git@github.com:user/repo.git"
# If multiple options exist and no default/when matches: shows interactive selector
# Then executes the selected transformation

swapx --dry-run --cmd "git clone git@github.com:user/repo.git"
# Same, but prints the result instead of executing
```

Cannot be combined with a subcommand.

### `--list-choices`

Requires `--cmd`. Detects pending choices without resolving them.

- **No pending choices** → stdout = transformed command, exit 0
- **Pending choices** → stdout = tab-separated choice data, exit 20

Output format when exit 20:

```
partially_transformed_command
MATCH_PATTERN\tDEFAULT_INDEX\tLABEL1\tLABEL2\t...
```

Line 1 is the command with auto-resolved rules applied (pending ones untouched). Lines 2+ describe each pending choice. Default index is `-1` if no default exists.

### `--choice <INDICES>`

Requires `--cmd`. Applies the user's selection from a previous `--list-choices` call. Takes comma-separated 0-based indices (e.g. `"1"` or `"1,0"` for multiple pending rules). Implies dry-run behavior.

```sh
# List available choices
swapx --cmd "echo melon" --list-choices
# exit 20, outputs: echo melon\nmelon\t-1\twater\tpapaya

# Apply choice index 1 (papaya)
swapx --cmd "echo melon" --choice 1
# outputs: echo papaya
```

`--list-choices` and `--choice` are mutually exclusive. Neither works with subcommands.

**Exit codes:**

| Code | Meaning |
|------|---------|
| 0 | No change, or transformation was applied |
| 10 | User made an interactive selection via dialoguer (direct TTY usage) |
| 20 | Pending choices exist (returned by `--list-choices`) |
| 1 | Error |

## Subcommands

### `swapx init`

Initialize global swapx config and install builtin suggestion packs.

```sh
swapx init
# Created /home/user/.config/swapx/
# Created /home/user/.config/swapx/rules.yaml
# Created /home/user/.config/swapx/suggestions.d/
# Created /home/user/.config/swapx/suggestions.d/builtin.yaml
```

Fails if `~/.config/swapx/` already exists (already initialized).

This creates:

- `~/.config/swapx/rules.yaml` — empty rules file with commented-out examples
- `~/.config/swapx/suggestions.d/builtin.yaml` — builtin suggestion pack (modern CLI tool replacements like `cat` → `bat`, `ls` → `eza`, `grep` → `rg`, etc.)

After init, run `swapx suggest` to auto-detect installed tools and generate rules from the suggestion packs.

---

### `swapx list`

Display all loaded rules from both global and local configs.

```sh
swapx list
```

Output includes the match type (`literal`/`regex`), match pattern, enabled status, replacement options, default markers, and `when` conditions.

```
1. [literal] match: "git@github.com:"
     → personal: "git@github-personal:" (default)
     → work: "git@github-work:" [when: cwd=~/work/**]
2. [regex] match: "docker run -p (\d+):(\d+)" [DISABLED]
     → swap-ports: "docker run -p $2:$1"
```

---

### `swapx add`

Interactive wizard to create a new rule.

```sh
swapx add
```

Prompts for:

1. Match pattern (literal string or regex)
2. Whether the pattern is a regex
3. One or more replacement options (label, replacement string, default flag)
4. Where to save (local `.swapx.yaml` or global config)

---

### `swapx enable <pattern>`

Re-enable a disabled rule. The `<pattern>` must exactly match the rule's `match` field.

```sh
swapx enable "git@github.com:"
# Enabled rule "git@github.com:" in /home/user/project/.swapx.yaml
```

Searches local config first, then global.

---

### `swapx disable <pattern>`

Disable a rule without deleting it. The rule remains in the config file with `enabled: false`.

```sh
swapx disable "git@github.com:"
# Disabled rule "git@github.com:" in /home/user/project/.swapx.yaml
```

Disabled rules are skipped during command transformation but still appear in `swapx list` (marked `[DISABLED]`) and `swapx explain`.

---

### `swapx explain <command...>`

Show all rules that match a command, what each replacement option would produce, and whether `when` conditions currently match.

```sh
swapx explain git clone git@github.com:user/repo.git
```

Output:

```
Command: git clone git@github.com:user/repo.git

Rule 1: [literal] [enabled] match: "git@github.com:"
  → personal: "git@github-personal:" → "git clone git@github-personal:user/repo.git" (default)
  → work: "git@github-work:" → "git clone git@github-work:user/repo.git"
```

Disabled rules are included in the output (marked `[DISABLED]`) so you can see what would match if re-enabled.

---

### `swapx shell-hook [shell]`

Output a shell hook script for transparent command interception. The shell argument is optional — if omitted, swapx auto-detects from `$NU_VERSION`, `$SHELL`, or `$PSModulePath`.

```sh
# Generate and inspect the hook
swapx shell-hook zsh

# Install per shell:
eval "$(swapx shell-hook zsh)"                        # zsh — add to ~/.zshrc
eval "$(swapx shell-hook bash)"                       # bash — add to ~/.bashrc
swapx shell-hook fish | source                        # fish — add to ~/.config/fish/config.fish
Invoke-Expression (swapx shell-hook powershell)       # PowerShell — add to $PROFILE
swapx shell-hook nu                                   # nushell — follow printed instructions
```

Supported shells: `zsh`, `bash`, `fish`, `powershell` (alias: `pwsh`), `nu` (alias: `nushell`).

The generated hooks use a two-phase protocol to handle interactive selection:

1. **Phase 1:** `swapx --dry-run --cmd "$BUFFER" --list-choices` — detects pending choices
2. **Exit 20** — pending choices exist; the hook parses the tab-separated output, shows a shell-native numbered menu, reads the user's selection, then calls phase 2
3. **Phase 2:** `swapx --dry-run --cmd "$BUFFER" --choice "$idx"` — applies the selected indices and auto-applies the result
4. **Exit 0** + command changed — no pending choices; the hook shows the transformation and prompts "Apply? [Y/n]" (or auto-applies if `SWAPX_AUTO_APPLY=1`)
5. **Exit 0** + command unchanged — no transformation; command runs as-is

**Environment variables:**

| Variable | Effect |
|---|---|
| `SWAPX_AUTO_APPLY=1` | Skip the confirmation prompt and auto-apply transformations |

---

### External commands (pass-through)

Any arguments that don't match a built-in subcommand are treated as a command to transform and execute.

```sh
swapx git clone git@github.com:user/repo.git
swapx docker run -p 8080:3000 myimage
swapx ssh admin@prod-server
```

**Behavior:**

- If a rule matches with a single replacement: auto-applied
- If a rule matches with multiple replacements and a `when` condition matches exactly one: auto-selected
- If a rule matches with multiple replacements and a default is set: default applied (non-interactive/pipe mode)
- If a rule matches with multiple replacements and no default/when: interactive prompt to choose
- If no rules match: command executes unchanged

---

## Modes

### Interactive mode

Run `swapx` with no arguments in a terminal to enter interactive mode. Type commands at the `swapx>` prompt. Each command is transformed, shown, and you're asked to confirm before execution.

```sh
swapx
swapx> git clone git@github.com:user/repo.git
  → git clone git@github-personal:user/repo.git
Execute? [Y/n]
```

Type `exit` or `quit` to leave.

### Pipe mode

When stdin is not a terminal, swapx reads commands from stdin, transforms each line, and writes to stdout. Defaults are applied automatically.

```sh
echo "git clone git@github.com:user/repo.git" | swapx
# git clone git@github-personal:user/repo.git
```

## Config resolution

swapx merges rules from two locations:

1. **Global**: `~/.config/swapx/rules.yaml` (or `$XDG_CONFIG_HOME/swapx/rules.yaml`)
2. **Local**: `.swapx.yaml` (walks up the directory tree from cwd)

Local rules override global rules when they share the same `match` pattern.
