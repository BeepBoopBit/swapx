use std::fs;

use assert_cmd::Command as AssertCommand;
use predicates::prelude::*;
use tempfile::TempDir;

fn swapx() -> AssertCommand {
    #[allow(deprecated)]
    AssertCommand::cargo_bin("swapx").unwrap()
}

fn create_config(dir: &TempDir, yaml: &str) -> std::path::PathBuf {
    let path = dir.path().join(".swapx.yaml");
    fs::write(&path, yaml).unwrap();
    path
}

// ─── init ───

#[test]
fn init_creates_config_file() {
    let dir = TempDir::new().unwrap();

    swapx()
        .arg("init")
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success()
        .stderr(predicate::str::contains("Created"));

    let config_dir = dir.path().join(".config").join("swapx");
    assert!(config_dir.is_dir());

    let rules_path = config_dir.join("rules.yaml");
    assert!(rules_path.exists());

    let suggestions_dir = config_dir.join("suggestions.d");
    assert!(suggestions_dir.is_dir());

    let builtin_path = suggestions_dir.join("builtin.yaml");
    assert!(builtin_path.exists());

    let builtin_contents = fs::read_to_string(&builtin_path).unwrap();
    assert!(builtin_contents.contains("suggestions:"));
    assert!(builtin_contents.contains("cat-to-bat"));
}

#[test]
fn init_force_replaces_existing() {
    let dir = TempDir::new().unwrap();
    let config_dir = dir.path().join(".config").join("swapx");
    let suggestions_dir = config_dir.join("suggestions.d");
    fs::create_dir_all(&suggestions_dir).unwrap();

    // Pre-create files with custom content
    fs::write(config_dir.join("rules.yaml"), "rules: [custom]\n").unwrap();
    fs::write(suggestions_dir.join("builtin.yaml"), "old content\n").unwrap();

    swapx()
        .args(["init", "--force"])
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success()
        .stderr(predicate::str::contains("Replaced"));

    // Verify files were overwritten with defaults
    let rules = fs::read_to_string(config_dir.join("rules.yaml")).unwrap();
    assert!(rules.contains("rules: []"));

    let builtin = fs::read_to_string(suggestions_dir.join("builtin.yaml")).unwrap();
    assert!(builtin.contains("suggestions:"));
}

#[test]
fn init_non_tty_errors_if_exists() {
    let dir = TempDir::new().unwrap();
    let config_dir = dir.path().join(".config").join("swapx");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("rules.yaml"), "rules: []\n").unwrap();

    // Without --force in a non-TTY context, should error
    swapx()
        .arg("init")
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

// ─── list ───

#[test]
fn list_shows_rules() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
        default: true
"#,
    );

    swapx()
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(
            predicate::str::contains("git checkout")
                .and(predicate::str::contains("use-switch"))
                .and(predicate::str::contains("git switch")),
        );
}

#[test]
fn list_shows_disabled_marker() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    enabled: false
    replace:
      - label: use-switch
        with: "git switch"
"#,
    );

    swapx()
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("DISABLED"));
}

#[test]
fn list_empty_config() {
    let dir = TempDir::new().unwrap();
    create_config(&dir, "rules: []\n");

    swapx()
        .arg("list")
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success()
        .stderr(predicate::str::contains("No rules configured"));
}

// ─── dry-run ───

#[test]
fn dry_run_transforms_command() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
        default: true
"#,
    );

    swapx()
        .args(["--dry-run", "git", "checkout", "main"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("git switch main"));
}

#[test]
fn dry_run_passthrough_when_no_match() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
        default: true
"#,
    );

    swapx()
        .args(["--dry-run", "echo", "hello"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("echo hello"));
}

#[test]
fn dry_run_regex_rule() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "docker run -p (\\d+):(\\d+)"
    regex: true
    replace:
      - label: swap-ports
        with: "docker run -p $2:$1"
"#,
    );

    swapx()
        .args(["--dry-run", "docker", "run", "-p", "8080:3000", "myapp"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("docker run -p 3000:8080 myapp"));
}

// ─── explain ───

#[test]
fn explain_shows_matching_rules() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
        default: true
"#,
    );

    swapx()
        .args(["explain", "git", "checkout", "main"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Command: git checkout main")
                .and(predicate::str::contains("literal"))
                .and(predicate::str::contains("enabled"))
                .and(predicate::str::contains("use-switch"))
                .and(predicate::str::contains("git switch main")),
        );
}

#[test]
fn explain_shows_no_match() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
"#,
    );

    swapx()
        .args(["explain", "echo", "hello"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("No rules match"));
}

#[test]
fn explain_shows_disabled_rules() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    enabled: false
    replace:
      - label: use-switch
        with: "git switch"
"#,
    );

    swapx()
        .args(["explain", "git", "checkout", "main"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("DISABLED"));
}

// ─── shell-hook ───

#[test]
fn shell_hook_zsh() {
    swapx()
        .args(["shell-hook", "zsh"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("__swapx_accept_line")
                .and(predicate::str::contains("zle -N accept-line")),
        );
}

#[test]
fn shell_hook_bash() {
    swapx()
        .args(["shell-hook", "bash"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("__swapx_debug_trap")
                .and(predicate::str::contains("shopt -s extdebug")),
        );
}

#[test]
fn shell_hook_fish() {
    swapx()
        .args(["shell-hook", "fish"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("__swapx_enter")
                .and(predicate::str::contains("bind \\r __swapx_enter")),
        );
}

#[test]
fn shell_hook_powershell() {
    swapx()
        .args(["shell-hook", "powershell"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set-PSReadLineKeyHandler"));
}

#[test]
fn shell_hook_nushell() {
    swapx()
        .args(["shell-hook", "nu"])
        .assert()
        .success()
        .stdout(predicate::str::contains("__swapx_handler"));
}

#[test]
fn shell_hook_unsupported_shell() {
    swapx()
        .args(["shell-hook", "tcsh"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported shell"));
}

// ─── enable / disable ───

#[test]
fn disable_then_enable_rule() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
        default: true
"#,
    );

    // Disable
    swapx()
        .args(["disable", "git checkout"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Disabled"));

    // Verify disabled in config
    let contents = fs::read_to_string(dir.path().join(".swapx.yaml")).unwrap();
    assert!(contents.contains("enabled: false"));

    // Dry-run should pass through unchanged
    swapx()
        .args(["--dry-run", "git", "checkout", "main"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("git checkout main"));

    // Re-enable
    swapx()
        .args(["enable", "git checkout"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Enabled"));

    // Dry-run should transform again
    swapx()
        .args(["--dry-run", "git", "checkout", "main"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("git switch main"));
}

#[test]
fn disable_nonexistent_rule() {
    let dir = TempDir::new().unwrap();
    create_config(&dir, "rules: []\n");

    swapx()
        .args(["disable", "nonexistent"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("No rule found"));
}

// ─── pipe mode ───

#[test]
fn pipe_mode_transforms_stdin() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
        default: true
"#,
    );

    swapx()
        .current_dir(dir.path())
        .write_stdin("git checkout main\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("git switch main"));
}

#[test]
fn pipe_mode_multiple_lines() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
        default: true
"#,
    );

    swapx()
        .current_dir(dir.path())
        .write_stdin("git checkout main\ngit checkout dev\necho hello\n")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("git switch main")
                .and(predicate::str::contains("git switch dev"))
                .and(predicate::str::contains("echo hello")),
        );
}

#[test]
fn pipe_mode_no_match_passthrough() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
        default: true
"#,
    );

    swapx()
        .current_dir(dir.path())
        .write_stdin("echo hello world\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("echo hello world"));
}

// ─── multi-replacement with default in pipe mode ───

#[test]
fn pipe_mode_uses_default_replacement() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git@github.com:"
    replace:
      - label: personal
        with: "git@github-personal:"
        default: true
      - label: work
        with: "git@github-work:"
"#,
    );

    swapx()
        .current_dir(dir.path())
        .write_stdin("git clone git@github.com:user/repo.git\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "git clone git@github-personal:user/repo.git",
        ));
}

// ─── --help ───

#[test]
fn help_flag() {
    swapx()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("swapx").and(predicate::str::contains("--dry-run")));
}

// ─── no config ───

#[test]
fn dry_run_works_without_config() {
    let dir = TempDir::new().unwrap();
    // No .swapx.yaml, no global config

    swapx()
        .args(["--dry-run", "echo", "hello"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success()
        .stdout(predicate::str::contains("echo hello"));
}

// ─── regex with named capture groups ───

#[test]
fn dry_run_named_capture_groups() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "deploy (?P<service>\\w+) to (?P<env>\\w+)"
    regex: true
    replace:
      - label: k8s-deploy
        with: "kubectl -n ${env} rollout restart deployment/${service}"
"#,
    );

    swapx()
        .args(["--dry-run", "deploy", "api", "to", "production"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "kubectl -n production rollout restart deployment/api",
        ));
}

// ─── when condition with env ───

#[test]
fn dry_run_when_env_condition() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
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
"#,
    );

    // With KUBE_ENV=staging, should auto-select staging
    swapx()
        .args(["--dry-run", "kubectl", "get", "pods"])
        .current_dir(dir.path())
        .env("KUBE_ENV", "staging")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "kubectl --context=staging get pods",
        ));
}

// ─── init content validation ───

#[test]
fn init_creates_valid_yaml() {
    let dir = TempDir::new().unwrap();

    swapx()
        .arg("init")
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success();

    // The installed suggestions should be loadable by suggest --check
    let result = swapx()
        .args(["suggest", "--check"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success();

    // Should show either suggestions or "No suggestions found" (depending on installed tools)
    let output = result.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("suggestion") || stderr.contains("No suggestions found"),
        "expected suggestion output, got: {}",
        stderr
    );
}

// ─── --cmd flag ───

#[test]
fn cmd_flag_single_replacement() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
"#,
    );

    swapx()
        .args(["--dry-run", "--cmd", "git checkout main"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("git switch main"));
}

#[test]
fn cmd_flag_no_match_passthrough() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
"#,
    );

    swapx()
        .args(["--dry-run", "--cmd", "echo hello world"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("echo hello world"));
}

#[test]
fn cmd_flag_when_condition_auto_select() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
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
"#,
    );

    swapx()
        .args(["--dry-run", "--cmd", "kubectl get pods"])
        .current_dir(dir.path())
        .env("KUBE_ENV", "staging")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "kubectl --context=staging get pods",
        ));
}

#[test]
fn cmd_flag_multi_replacement_no_default_non_tty() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git@github.com:"
    replace:
      - label: personal
        with: "git@github-personal:"
      - label: work
        with: "git@github-work:"
"#,
    );

    // In test, stdin is not a TTY, so pending choices should pass through as-is
    swapx()
        .args([
            "--dry-run",
            "--cmd",
            "git clone git@github.com:user/repo.git",
        ])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "git clone git@github.com:user/repo.git",
        ));
}

#[test]
fn cmd_flag_multi_replacement_has_default_non_tty() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git@github.com:"
    replace:
      - label: personal
        with: "git@github-personal:"
        default: true
      - label: work
        with: "git@github-work:"
"#,
    );

    // Non-tty with a default should apply the default
    swapx()
        .args([
            "--dry-run",
            "--cmd",
            "git clone git@github.com:user/repo.git",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "git clone git@github-personal:user/repo.git",
        ));
}

#[test]
fn cmd_flag_preserves_special_characters() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "echo"
    replace:
      - label: printf
        with: "printf"
"#,
    );

    swapx()
        .args(["--dry-run", "--cmd", "echo 'hello world' | grep foo && bar"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "printf 'hello world' | grep foo && bar",
        ));
}

#[test]
fn cmd_flag_combined_with_subcommand_errors() {
    swapx()
        .args(["--cmd", "git checkout main", "list"])
        .assert()
        .failure();
}

#[test]
fn shell_hook_zsh_contains_cmd_flag() {
    swapx()
        .args(["shell-hook", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("swapx --dry-run --cmd"));
}

#[test]
fn shell_hook_bash_contains_cmd_flag() {
    swapx()
        .args(["shell-hook", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("swapx --dry-run --cmd"));
}

#[test]
fn shell_hook_fish_contains_cmd_flag() {
    swapx()
        .args(["shell-hook", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("swapx --dry-run --cmd"));
}

#[test]
fn shell_hook_powershell_contains_cmd_flag() {
    swapx()
        .args(["shell-hook", "powershell"])
        .assert()
        .success()
        .stdout(predicate::str::contains("swapx --dry-run --cmd"));
}

#[test]
fn shell_hook_nushell_contains_cmd_flag() {
    swapx()
        .args(["shell-hook", "nu"])
        .assert()
        .success()
        .stdout(predicate::str::contains("swapx --dry-run --cmd"));
}

// ─── --list-choices ───

#[test]
fn list_choices_no_pending_outputs_transformed() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
        default: true
"#,
    );

    swapx()
        .args(["--cmd", "git checkout main", "--list-choices"])
        .current_dir(dir.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("git switch main"));
}

#[test]
fn list_choices_with_pending_exits_20() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "melon"
    replace:
      - label: water
        with: "watermelon"
      - label: papaya
        with: "papaya"
"#,
    );

    swapx()
        .args(["--cmd", "echo melon", "--list-choices"])
        .current_dir(dir.path())
        .assert()
        .code(20)
        .stdout(
            predicate::str::contains("echo melon\n")
                .and(predicate::str::contains("melon\t-1\twater\tpapaya")),
        );
}

#[test]
fn list_choices_with_default_index() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "melon"
    replace:
      - label: water
        with: "watermelon"
      - label: papaya
        with: "papaya"
        default: true
"#,
    );

    swapx()
        .args(["--cmd", "echo melon", "--list-choices"])
        .current_dir(dir.path())
        .assert()
        .code(20)
        .stdout(predicate::str::contains("melon\t1\twater\tpapaya"));
}

#[test]
fn list_choices_no_match_exits_0() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "git checkout"
    replace:
      - label: use-switch
        with: "git switch"
"#,
    );

    swapx()
        .args(["--cmd", "echo hello", "--list-choices"])
        .current_dir(dir.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("echo hello"));
}

#[test]
fn list_choices_requires_cmd() {
    swapx()
        .args(["--list-choices"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--list-choices requires --cmd"));
}

// ─── --choice ───

#[test]
fn choice_applies_selection() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "melon"
    replace:
      - label: water
        with: "watermelon"
      - label: papaya
        with: "papaya"
"#,
    );

    swapx()
        .args(["--cmd", "echo melon", "--choice", "1"])
        .current_dir(dir.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("echo papaya"));
}

#[test]
fn choice_index_0() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "melon"
    replace:
      - label: water
        with: "watermelon"
      - label: papaya
        with: "papaya"
"#,
    );

    swapx()
        .args(["--cmd", "echo melon", "--choice", "0"])
        .current_dir(dir.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("echo watermelon"));
}

#[test]
fn choice_out_of_range() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "melon"
    replace:
      - label: water
        with: "watermelon"
      - label: papaya
        with: "papaya"
"#,
    );

    swapx()
        .args(["--cmd", "echo melon", "--choice", "5"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("out of range"));
}

#[test]
fn choice_invalid_index() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "melon"
    replace:
      - label: water
        with: "watermelon"
      - label: papaya
        with: "papaya"
"#,
    );

    swapx()
        .args(["--cmd", "echo melon", "--choice", "abc"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid choice index"));
}

#[test]
fn choice_requires_cmd() {
    swapx()
        .args(["--choice", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--choice requires --cmd"));
}

#[test]
fn list_choices_and_choice_mutual_exclusion() {
    swapx()
        .args(["--cmd", "echo hello", "--list-choices", "--choice", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("mutually exclusive"));
}

// ─── shell hooks contain --list-choices and --choice ───

#[test]
fn shell_hook_zsh_contains_list_choices() {
    swapx()
        .args(["shell-hook", "zsh"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--list-choices").and(predicate::str::contains("--choice")),
        );
}

#[test]
fn shell_hook_bash_contains_list_choices() {
    swapx()
        .args(["shell-hook", "bash"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--list-choices").and(predicate::str::contains("--choice")),
        );
}

#[test]
fn shell_hook_fish_contains_list_choices() {
    swapx()
        .args(["shell-hook", "fish"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--list-choices").and(predicate::str::contains("--choice")),
        );
}

#[test]
fn shell_hook_powershell_contains_list_choices() {
    swapx()
        .args(["shell-hook", "powershell"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--list-choices").and(predicate::str::contains("--choice")),
        );
}

#[test]
fn shell_hook_nushell_contains_list_choices() {
    swapx()
        .args(["shell-hook", "nu"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--list-choices").and(predicate::str::contains("--choice")),
        );
}

// ─── rules.plk.yaml loading ───

#[test]
fn plk_config_is_loaded() {
    let dir = TempDir::new().unwrap();
    let config_dir = dir.path().join(".config").join("swapx");
    fs::create_dir_all(&config_dir).unwrap();

    // Write a rules.plk.yaml with a rule
    fs::write(
        config_dir.join("rules.plk.yaml"),
        r#"rules:
  - match: "cd swapx"
    dir: /some/path
    replace:
      - label: "plk: swapx-edit"
        with: "plk run swapx-edit"
      - label: "just cd swapx"
        with: "cd swapx"
        default: true
"#,
    )
    .unwrap();

    // list should show the plk rule
    swapx()
        .arg("list")
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success()
        .stderr(
            predicate::str::contains("cd swapx")
                .and(predicate::str::contains("plk: swapx-edit"))
                .and(predicate::str::contains("dir: \"/some/path\"")),
        );
}

#[test]
fn plk_config_rule_with_dir_matching_cwd_applies() {
    let dir = TempDir::new().unwrap();
    let config_dir = dir.path().join(".config").join("swapx");
    fs::create_dir_all(&config_dir).unwrap();

    // Rule with dir matching the temp directory
    let dir_path = dir.path().to_string_lossy().to_string();
    fs::write(
        config_dir.join("rules.plk.yaml"),
        format!(
            r#"rules:
  - match: "git checkout"
    dir: "{}"
    replace:
      - label: use-switch
        with: "git switch"
"#,
            dir_path
        ),
    )
    .unwrap();

    swapx()
        .args(["--dry-run", "--cmd", "git checkout main"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success()
        .stdout(predicate::str::contains("git switch main"));
}

#[test]
fn plk_config_rule_with_dir_not_matching_cwd_skips() {
    let dir = TempDir::new().unwrap();
    let config_dir = dir.path().join(".config").join("swapx");
    fs::create_dir_all(&config_dir).unwrap();

    // Rule with dir NOT matching the temp directory
    fs::write(
        config_dir.join("rules.plk.yaml"),
        r#"rules:
  - match: "git checkout"
    dir: /nonexistent/path
    replace:
      - label: use-switch
        with: "git switch"
"#,
    )
    .unwrap();

    swapx()
        .args(["--dry-run", "--cmd", "git checkout main"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success()
        .stdout(predicate::str::contains("git checkout main"));
}

// ─── suggest ───

#[test]
fn suggest_check_shows_suggestions_after_init() {
    // After init, suggestion packs are on disk. --check should at least run without error.
    let dir = TempDir::new().unwrap();

    // First run init to install builtin suggestions
    swapx()
        .arg("init")
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success();

    let result = swapx()
        .args(["suggest", "--check"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success();

    // Should show either "suggestion(s)" or "No suggestions found"
    let output = result.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("suggestion") || stderr.contains("No suggestions found"),
        "expected suggestion output, got: {}",
        stderr
    );
}

#[test]
fn suggest_auto_creates_rules() {
    let dir = TempDir::new().unwrap();
    let config_dir = dir.path().join(".config").join("swapx");
    fs::create_dir_all(&config_dir).unwrap();

    // Create a suggestion pack that detects "sh" (always available)
    let suggestions_dir = config_dir.join("suggestions.d");
    fs::create_dir_all(&suggestions_dir).unwrap();
    fs::write(
        suggestions_dir.join("test.yaml"),
        r#"suggestions:
  - name: test-suggestion
    description: "test suggestion for sh"
    detect:
      bin: sh
    rules:
      - match: "test-match-pattern"
        replace:
          - label: "use alt"
            with: "alt-pattern"
            default: true
          - label: "keep original"
            with: "test-match-pattern"
"#,
    )
    .unwrap();

    swapx()
        .args(["suggest", "--auto"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .assert()
        .success()
        .stderr(predicate::str::contains("Saved"));

    // Verify rules were saved to global config
    let rules_path = config_dir.join("rules.yaml");
    assert!(rules_path.exists(), "rules.yaml should have been created");
    let contents = fs::read_to_string(&rules_path).unwrap();
    assert!(
        contents.contains("test-match-pattern"),
        "saved rules should contain the match pattern"
    );
    assert!(
        contents.contains("alt-pattern"),
        "saved rules should contain the replacement"
    );
}

#[test]
fn suggest_no_suggestions() {
    let dir = TempDir::new().unwrap();

    // Use an empty PATH so no binaries are detected
    swapx()
        .args(["suggest", "--check"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join(".config"))
        .env("PATH", dir.path().join("empty-bin"))
        .assert()
        .success()
        .stderr(predicate::str::contains("No suggestions found"));
}

// ─── reset ───

#[test]
fn reset_requires_interactive_confirmation() {
    // In test, stdin is not a TTY, so reset should refuse
    swapx()
        .arg("reset")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "reset requires interactive confirmation",
        ));
}

#[test]
fn reset_shows_in_help() {
    swapx()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("reset").or(predicate::str::contains("Reset")));
}

// ─── dir field ───

#[test]
fn list_shows_dir_field() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "cd project"
    dir: /home/user/projects
    replace:
      - label: goto
        with: "cd /home/user/projects"
"#,
    );

    swapx()
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(
            predicate::str::contains("cd project")
                .and(predicate::str::contains("dir: \"/home/user/projects\"")),
        );
}

#[test]
fn explain_shows_dir_field() {
    let dir = TempDir::new().unwrap();
    create_config(
        &dir,
        r#"rules:
  - match: "cd project"
    dir: /nonexistent/dir
    replace:
      - label: goto
        with: "cd /home/user/projects"
"#,
    );

    swapx()
        .args(["explain", "cd", "project"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(
            predicate::str::contains("dir: \"/nonexistent/dir\"")
                .and(predicate::str::contains("no match")),
        );
}
