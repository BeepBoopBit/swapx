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
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Created"));

    let config_path = dir.path().join(".swapx.yaml");
    assert!(config_path.exists());

    let contents = fs::read_to_string(&config_path).unwrap();
    assert!(contents.contains("git checkout"));
    assert!(contents.contains("git switch"));
    assert!(contents.contains("python"));
}

#[test]
fn init_fails_if_config_exists() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".swapx.yaml"), "rules: []").unwrap();

    swapx()
        .arg("init")
        .current_dir(dir.path())
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
        .current_dir(dir.path())
        .assert()
        .success();

    // The generated config should be valid enough to use with list
    swapx()
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("git checkout").and(predicate::str::contains("python")));
}
