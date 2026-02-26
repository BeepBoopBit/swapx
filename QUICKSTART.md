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

## 2. Initialize a config

Navigate to a project directory and create a local config:

```sh
cd ~/my-project
swapx init
```

This creates `.swapx.yaml` with two example rules:

- `git checkout` → `git switch` — nudge toward modern Git commands
- `python` → `python3` — common fix for macOS/Linux where `python` is missing or points to 2.x

## 3. See your rules

```sh
swapx list
```

Output:

```
1. [literal] match: "git checkout"
     → use-switch: "git switch" (default)
2. [literal] match: "python "
     → use-python3: "python3 " (default)
```

## 4. Test with dry-run

Preview a transformation without executing anything:

```sh
swapx --dry-run git checkout main
```

Output:

```
git switch main
```

The default replacement was applied automatically.

## 5. Run for real

```sh
swapx git checkout main
```

swapx shows the rewritten command and executes it:

```
swapx: → git switch main
```

## 6. Debug with explain

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

## 7. Edit rules

Open `.swapx.yaml` in your editor and modify rules directly. The format is straightforward YAML. Here's a minimal rule:

```yaml
rules:
  - match: "npm test"
    replace:
      - label: use-vitest
        with: "npx vitest"
```

## 8. Add shell integration (optional)

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

Now just type commands normally. When a rule matches, swapx shows the transformation and asks to confirm before applying.

Set `SWAPX_AUTO_APPLY=1` in your shell config to skip the confirmation.

## Next steps

- [COMMANDS.md](COMMANDS.md) — full command reference
- [EXAMPLES.md](EXAMPLES.md) — real-world workflow examples
- [README.md](README.md) — configuration format and all features
