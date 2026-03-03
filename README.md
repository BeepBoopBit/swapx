# swapx

A command rewriter for your terminal. Define rules that automatically transform commands before they execute — swap Git remotes, flip Docker ports, rewrite URLs, or anything else.

```
$ swapx git checkout main
swapx: → git switch main
```

## Why

You keep running the same commands with the wrong defaults. `git clone` grabs the wrong remote. `docker run` maps ports backwards. `ssh` connects to the wrong host. You catch it after the fact and re-run.

swapx sits in front of your shell and rewrites commands before they execute, based on rules you define in YAML.

## Features

- **Literal and regex matching** — simple string replacement or full regex with capture groups (`$1`, `$2`, `${name}`)
- **Multiple replacements per rule** — choose interactively or set a default
- **Directory-scoped auto-selection** — `when` conditions match on working directory or environment variables
- **Global + local config** — global rules in `~/.config/swapx/rules.yaml`, per-project overrides in `.swapx.yaml`
- **Shell integration** — transparent interception via zsh/bash/fish/PowerShell/nushell hooks
- **Explain mode** — debug which rules match and preview what they'd produce
- **Enable/disable rules** — toggle rules on and off without deleting them
- **Dry-run mode** — see the transformed command without executing
- **Two-phase selection** — `--list-choices` and `--choice` flags let shell hooks show native numbered menus for multi-option rules
- **Pipe mode** — use swapx as a filter in pipelines (`echo "cmd" | swapx`)

## Install

```sh
cargo install swapx
```

Or build from source:

```sh
git clone https://github.com/BeepBoopBit/swapx.git
cd swapx
cargo install --path .
```

## Quick start

```sh
# Create a .swapx.yaml in your project
swapx init

# See what's configured
swapx list

# Test it
swapx --dry-run git checkout main

# Run it for real
swapx git checkout main
```

See [QUICKSTART.md](QUICKSTART.md) for a full walkthrough.

## Documentation

- [QUICKSTART.md](QUICKSTART.md) — first-time setup and basic usage
- [COMMANDS.md](COMMANDS.md) — complete command reference
- [EXAMPLES.md](EXAMPLES.md) — real-world workflows and config examples
- [CHANGELOG.md](CHANGELOG.md) — release history
- [CONTRIBUTING.md](CONTRIBUTING.md) — how to contribute
- [SECURITY.md](SECURITY.md) — security policy and reporting

## Configuration

swapx loads rules from two locations (local overrides global):

| Location | Purpose |
|---|---|
| `~/.config/swapx/rules.yaml` | Global rules (all projects) |
| `.swapx.yaml` | Local rules (per-project, walks up to find it) |

### Rule format

```yaml
rules:
  # Literal match with multiple options
  - match: "git@github.com:"
    replace:
      - label: personal
        with: "git@github-personal:"
        default: true
      - label: work
        with: "git@github-work:"

  # Regex match with capture groups
  - match: "docker run -p (\\d+):(\\d+)"
    regex: true
    replace:
      - label: swap-ports
        with: "docker run -p $2:$1"

  # Directory-scoped auto-selection
  - match: "kubectl"
    replace:
      - label: staging
        with: "kubectl --context=staging"
        when:
          cwd: "~/work/staging/**"
      - label: production
        with: "kubectl --context=production"
        when:
          env: "KUBE_ENV=production"
```

### Rule fields

| Field | Type | Required | Description |
|---|---|---|---|
| `match` | string | yes | Pattern to match against the command |
| `regex` | bool | no | Treat `match` as a regex (default: `false`) |
| `enabled` | bool | no | Whether the rule is active (default: `true`) |
| `replace` | list | yes | One or more replacement options |

### Replacement fields

| Field | Type | Required | Description |
|---|---|---|---|
| `label` | string | yes | Name for this option (shown in interactive selection) |
| `with` | string | yes | The replacement string (supports `$1`, `$2`, `${name}` for regex) |
| `default` | bool | no | Auto-select this option in non-interactive mode (default: `false`) |
| `when` | object | no | Conditions for auto-selecting this option |

### When conditions

| Field | Type | Description |
|---|---|---|
| `cwd` | string | Glob pattern matched against the current working directory |
| `env` | string | `KEY=VALUE` (match value) or `KEY` (check existence) |

When multiple conditions are specified on a single replacement, all must match (AND logic). When exactly one replacement's `when` condition matches, it is auto-selected. When multiple match, you're prompted to choose.

## Shell integration

For transparent command interception (no `swapx` prefix needed):

```sh
# zsh — add to ~/.zshrc
eval "$(swapx shell-hook zsh)"

# bash — add to ~/.bashrc
eval "$(swapx shell-hook bash)"

# fish — add to ~/.config/fish/config.fish
swapx shell-hook fish | source

# PowerShell — add to $PROFILE
Invoke-Expression (swapx shell-hook powershell)

# nushell — follow printed instructions
swapx shell-hook nu
```

Shell hooks use a two-phase protocol internally. First, `swapx --list-choices` detects pending choices. If any exist (exit 20), the hook shows a shell-native numbered menu, reads your selection, then applies it via `swapx --choice`. If no choices are pending (exit 0), the hook shows the transformation and prompts "Apply? [Y/n]".

Set `SWAPX_AUTO_APPLY=1` to skip the confirmation prompt for non-interactive transformations.

## License

MIT
