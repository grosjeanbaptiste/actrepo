//! `actrepo actit|actname|actautoma` end to end through the binary. No network.
use std::process::{Command, Output};

fn repo(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("{e}"))
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn repo_actname_prints_what_actname_prints() {
    let o = repo(&[
        "actname",
        "generate",
        "--prefix",
        "act",
        "--domain",
        "architecture plans",
        "--limit",
        "2",
        "--format",
        "json",
    ]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(
        stdout(&o).matches("\"name\":\"act").count(),
        2,
        "{}",
        stdout(&o)
    );
}

#[test]
fn repo_actname_with_a_bad_option_fails_and_says_why() {
    let o = repo(&["actname", "generate", "--zap", "1"]);
    assert!(!o.status.success());
    assert!(
        stderr(&o).contains("unknown option: --zap"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn repo_actit_with_a_bad_setting_fails_naming_the_flag() {
    let o = repo(&["actit", "--min-coverage", "3", "pdf"]);
    assert!(!o.status.success());
    assert!(stderr(&o).contains("--min-coverage"), "{}", stderr(&o));
}

#[test]
fn repo_actautoma_with_no_known_command_prints_the_usage() {
    let o = repo(&["actautoma"]);
    assert!(!o.status.success());
    assert!(
        stderr(&o).contains("usage: actautoma rename-repo"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn repo_move_finds_the_repositories_itself_and_a_dry_run_changes_nothing() {
    let base = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let group = base.path().join("group");
    for r in ["a", "b"] {
        std::fs::create_dir_all(group.join(r).join(".git")).unwrap_or_else(|e| panic!("{e}"));
    }
    std::fs::create_dir(base.path().join("into")).unwrap_or_else(|e| panic!("{e}"));
    let o = Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .arg("move")
        .arg(&group)
        .arg("--into")
        .arg(base.path().join("into"))
        .arg("--dry-run")
        .env("HOME", base.path())
        .output()
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(o.status.success(), "{}", stderr(&o));
    assert!(
        stdout(&o).contains("2 repositories found"),
        "{}",
        stdout(&o)
    );
    assert!(group.join("a/.git").is_dir() && !base.path().join("into/group").exists());
}

#[test]
fn repo_pin_propagate_without_a_dependency_prints_the_usage_of_actautoma() {
    let o = repo(&["pin-propagate"]);
    assert!(!o.status.success());
    assert!(
        stderr(&o).contains("usage: actautoma pin-propagate"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn repo_pin_propagate_dry_run_names_each_consumer_and_changes_nothing() {
    let base = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let dir = base.path().join("alpha");
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
    let old = "1".repeat(40);
    let manifest = format!(
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\n\n[dependencies]\nactit = {{ git = \"https://github.com/o/actit\", rev = \"{old}\" }}\n"
    );
    std::fs::write(dir.join("Cargo.toml"), &manifest).unwrap_or_else(|e| panic!("{e}"));
    let o = repo(&[
        "pin-propagate",
        "actit",
        "--rev",
        &"2".repeat(40),
        "--root",
        base.path().to_str().unwrap_or_default(),
        "--dry-run",
    ]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert!(stdout(&o).contains("alpha: planned"), "{}", stdout(&o));
    let after = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
    assert_eq!(after, manifest);
}
