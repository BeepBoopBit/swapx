# Command Reference

## Global flags

### `--dry-run`

Show the transformed command without executing it. Works with any external command.

```sh
swapx --dry-run git clone git@github.com:user/repo.git
# prints: git clone git@github-personal:user/repo.git
```

## Subcommands

### `swapx init`

Create a `.swapx.yaml` in the current directory with example rules.

```sh
swapx init
# Created /home/user/project/.swapx.yaml
```

Fails if `.swapx.yaml` already exists in the current directory.

The generated config includes:

- `git checkout` → `git switch` — nudge toward modern Git commands
- `python` → `python3` — fix for systems where `python` is missing or points to 2.x

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
