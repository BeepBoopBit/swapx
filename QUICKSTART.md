# Quick Start

Get up and running with swapx in under 5 minutes.

## 1. Install

```sh
cargo install --path .
```

Verify it works:

```sh
swapx --help
```

## 2. Initialize config

Set up the global config directory and install builtin suggestion packs:

```sh
swapx init
```

This creates:

- `~/.config/swapx/rules.yaml` — empty rules file with commented-out examples
- `~/.config/swapx/suggestions.d/builtin.yaml` — builtin suggestions for modern CLI tool replacements

## 3. Generate rules from suggestions

Auto-detect installed tools and generate rules:

```sh
swapx suggest --auto
```

Or interactively pick which suggestions to accept:

```sh
swapx suggest
```

## 4. See your rules

```sh
swapx list
```

## 5. Test with dry-run

Preview a transformation without executing anything:

```sh
swapx --dry-run git checkout main
```

Output:

```
git switch main
```

The default replacement was applied automatically.

## 6. Run for real

```sh
swapx git checkout main
```

swapx shows the rewritten command and executes it:

```
swapx: → git switch main
```

## 7. Debug with explain

Not sure which rules will match? Use `explain`:

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

## 8. Edit rules

Open `~/.config/swapx/rules.yaml` (global) or `.swapx.yaml` (per-project) in your editor and modify rules directly. The format is straightforward YAML. Here's a minimal rule:

```yaml
rules:
  - match: "npm test"
    replace:
      - label: use-vitest
        with: "npx vitest"
```

## 9. Add shell integration (optional)

To intercept commands transparently without the `swapx` prefix:

**zsh** — add to `~/.zshrc`:

```sh
eval "$(swapx shell-hook zsh)"
```

**bash** — add to `~/.bashrc`:

```sh
eval "$(swapx shell-hook bash)"
```

**fish** — add to `~/.config/fish/config.fish`:

```fish
swapx shell-hook fish | source
```

**PowerShell** — add to `$PROFILE`:

```powershell
Invoke-Expression (swapx shell-hook powershell)
```

**nushell** — run and follow the printed instructions:

```sh
swapx shell-hook nu
```

This outputs a function to save to `~/.config/nushell/swapx.nu` and a keybinding to add to your `config.nu`.

Now just type commands normally. When a rule matches, swapx shows the transformation and asks to confirm before applying. If a rule has multiple options with no matching `when` condition or default, a numbered menu appears directly in your shell so you can choose.

Set `SWAPX_AUTO_APPLY=1` in your shell config to skip the confirmation for non-interactive transformations.

## Next steps

- [COMMANDS.md](COMMANDS.md) — full command reference
- [EXAMPLES.md](EXAMPLES.md) — real-world workflow examples
- [README.md](README.md) — configuration format and all features
