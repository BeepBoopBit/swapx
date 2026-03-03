# Examples

Real-world workflows and configuration patterns for swapx.

## Git remote switching

**Problem:** You have personal and work GitHub accounts with different SSH keys configured as separate hosts in `~/.ssh/config`. Every `git clone` from GitHub defaults to the wrong one.

**Config (`.swapx.yaml`):**

```yaml
rules:
  - match: "git@github.com:"
    replace:
      - label: personal
        with: "git@github-personal:"
        default: true
      - label: work
        with: "git@github-work:"
```

**Usage:**

```sh
# Default applies automatically in scripts/pipes
$ swapx --dry-run git clone git@github.com:torvalds/linux.git
git clone git@github-personal:torvalds/linux.git

# Interactive prompt when multiple options exist and no default
$ swapx git clone git@github.com:company/internal.git
Choose replacement for 'git@github.com:':
> personal
  work
```

---

## Git remote switching with directory-scoped auto-selection

**Problem:** Same as above, but you want it to auto-select based on which directory you're in — personal projects in `~/personal`, work projects in `~/work`.

**Config:**

```yaml
rules:
  - match: "git@github.com:"
    replace:
      - label: personal
        with: "git@github-personal:"
        when:
          cwd: "~/personal/**"
      - label: work
        with: "git@github-work:"
        when:
          cwd: "~/work/**"
```

**Usage:**

```sh
$ cd ~/work/projects
$ swapx git clone git@github.com:company/api.git
swapx: → git clone git@github-work:company/api.git

$ cd ~/personal
$ swapx git clone git@github.com:me/dotfiles.git
swapx: → git clone git@github-personal:me/dotfiles.git
```

No prompts needed — the `when` conditions auto-select the right option.

---

## Docker port swap

**Problem:** You keep mixing up the host:container port order in `docker run -p`.

**Config:**

```yaml
rules:
  - match: "docker run -p (\\d+):(\\d+)"
    regex: true
    replace:
      - label: swap-ports
        with: "docker run -p $2:$1"
```

**Usage:**

```sh
$ swapx docker run -p 3000:8080 myapp
swapx: → docker run -p 8080:3000 myapp
```

The regex captures both port numbers and swaps them using `$1` and `$2`.

---

## Environment-based kubectl context

**Problem:** You want `kubectl` commands to automatically target the right cluster based on an environment variable.

**Config:**

```yaml
rules:
  - match: "kubectl"
    replace:
      - label: staging
        with: "kubectl --context=staging"
        when:
          env: "KUBE_ENV=staging"
      - label: production
        with: "kubectl --context=production"
        when:
          env: "KUBE_ENV=production"
      - label: local
        with: "kubectl --context=minikube"
        default: true
```

**Usage:**

```sh
$ export KUBE_ENV=staging
$ swapx kubectl get pods
swapx: → kubectl --context=staging get pods

$ unset KUBE_ENV
$ swapx kubectl get pods
swapx: → kubectl --context=minikube get pods
```

When `KUBE_ENV` matches a `when` condition, that option is auto-selected. Otherwise, the default (`minikube`) is used.

---

## npm to pnpm migration

**Problem:** You're migrating a project from npm to pnpm but keep typing `npm` out of habit.

**Config:**

```yaml
rules:
  - match: "npm install"
    replace:
      - label: pnpm
        with: "pnpm install"

  - match: "npm run"
    replace:
      - label: pnpm
        with: "pnpm run"

  - match: "npm test"
    replace:
      - label: pnpm
        with: "pnpm test"
```

**Usage:**

```sh
$ swapx npm install express
swapx: → pnpm install express

$ swapx npm test
swapx: → pnpm test
```

---

## SSH host alias

**Problem:** You connect to servers with long hostnames and want shorthand.

**Config:**

```yaml
rules:
  - match: "ssh prod"
    replace:
      - label: production
        with: "ssh admin@prod-server.us-east-1.company.internal"

  - match: "ssh staging"
    replace:
      - label: staging
        with: "ssh deploy@staging.us-east-1.company.internal"
```

**Usage:**

```sh
$ swapx ssh prod
swapx: → ssh admin@prod-server.us-east-1.company.internal
```

---

## Regex named capture groups

**Problem:** You want to rewrite a command that has a structured pattern and refer to parts by name.

**Config:**

```yaml
rules:
  - match: "deploy (?P<service>\\w+) to (?P<env>\\w+)"
    regex: true
    replace:
      - label: k8s-deploy
        with: "kubectl -n ${env} rollout restart deployment/${service}"
```

**Usage:**

```sh
$ swapx deploy api to production
swapx: → kubectl -n production rollout restart deployment/api
```

---

## Temporarily disabling a rule

You don't need to delete rules to stop them from matching. Use `disable` and `enable`:

```sh
# Turn off the git remote rule
$ swapx disable "git@github.com:"
Disabled rule "git@github.com:" in .swapx.yaml

# Verify it's disabled
$ swapx list
1. [literal] match: "git@github.com:" [DISABLED]
     → personal: "git@github-personal:" (default)
     → work: "git@github-work:"

# Commands pass through unchanged
$ swapx --dry-run git clone git@github.com:user/repo.git
git clone git@github.com:user/repo.git

# Re-enable it
$ swapx enable "git@github.com:"
Enabled rule "git@github.com:" in .swapx.yaml
```

---

## Using explain to debug rules

When you're not sure which rules will fire or what they'll produce:

```sh
$ swapx explain docker run -p 8080:3000 myapp
Command: docker run -p 8080:3000 myapp

Rule 1: [regex] [enabled] match: "docker run -p (\d+):(\d+)"
  → swap-ports: "docker run -p $2:$1" → "docker run -p 3000:8080 myapp"
```

This shows the full result of every replacement option, including whether `when` conditions match.

---

## Pipe mode for scripting

Use swapx as a filter in shell pipelines:

```sh
# Transform a list of commands from a file
cat commands.txt | swapx

# Chain with other tools
history | grep "git clone" | awk '{$1=""; print $0}' | swapx
```

In pipe mode, defaults are applied automatically and there are no interactive prompts.

---

## Shell integration workflow

Instead of prefixing every command with `swapx`, install the shell hook:

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

Now type commands normally. When a rule has a single replacement or a `when` condition matches, the hook shows the transformation and asks to confirm:

```
$ git checkout main

swapx: git checkout main
    → git switch main
Apply? [Y/n]
```

When a rule has multiple replacement options and no default or `when` condition matches, you'll see a numbered menu directly in your shell:

```
$ git clone git@github.com:user/repo.git
Choose replacement for 'git@github.com:':
  1) personal
  2) work
#? 1
```

After you select an option, the transformation is auto-applied (no extra "Apply?" prompt since you already chose).

To skip the confirmation prompt for non-interactive transformations and always auto-apply:

```sh
export SWAPX_AUTO_APPLY=1
```

The hook automatically skips commands that start with `swapx` to prevent infinite loops.

---

## Global vs local config

**Global config** (`~/.config/swapx/rules.yaml`) — rules that apply everywhere:

```yaml
rules:
  - match: "ssh prod"
    replace:
      - label: production
        with: "ssh admin@prod.company.internal"
```

**Local config** (`.swapx.yaml` in project root) — project-specific rules that override global:

```yaml
rules:
  - match: "npm test"
    replace:
      - label: vitest
        with: "npx vitest"
```

swapx walks up the directory tree from your cwd to find `.swapx.yaml`, so it works in subdirectories too. When both configs have a rule with the same `match` pattern, the local one wins.
