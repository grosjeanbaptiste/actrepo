//! `repo actit|actname|actautoma` end to end through the binary. No network.
use std::process::{Command, Output};

fn repo(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_repo"))
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
