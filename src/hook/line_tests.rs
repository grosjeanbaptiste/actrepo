use super::*;
use crate::hook::install::HOOK;
use crate::scaffold::native::Process;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let o = Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(d.path())
        .output()
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(o.status.success());
    d
}

fn hook_file(d: &Path) -> PathBuf {
    d.join(".git/hooks/post-commit")
}

#[test]
fn apply_writes_the_hook_and_says_so() {
    let d = repo();
    let line = line(&Process, d.path(), Mode::Apply);
    assert_eq!(line, "  + wrote      post-commit hook");
    assert_eq!(
        fs::read_to_string(hook_file(d.path())).unwrap_or_default(),
        HOOK
    );
}

#[test]
fn force_writes_the_hook_too() {
    let d = repo();
    assert_eq!(
        line(&Process, d.path(), Mode::Force),
        "  + wrote      post-commit hook"
    );
    assert!(hook_file(d.path()).exists());
}

#[test]
fn a_second_apply_reports_ok_and_changes_nothing() {
    let d = repo();
    line(&Process, d.path(), Mode::Apply);
    assert_eq!(
        line(&Process, d.path(), Mode::Apply),
        "  = ok       post-commit hook"
    );
}

#[test]
fn check_reports_missing_and_writes_nothing() {
    let d = repo();
    assert_eq!(
        line(&Process, d.path(), Mode::Check),
        "  - missing  post-commit hook"
    );
    assert!(!hook_file(d.path()).exists());
}

#[test]
fn diff_says_it_would_write_and_writes_nothing() {
    let d = repo();
    assert_eq!(
        line(&Process, d.path(), Mode::Diff),
        "  - would write  post-commit hook"
    );
    assert!(!hook_file(d.path()).exists());
}

#[test]
fn another_hook_is_kept_and_reported_loudly() {
    let d = repo();
    fs::create_dir_all(hook_file(d.path()).parent().unwrap_or(d.path()))
        .unwrap_or_else(|e| panic!("{e}"));
    fs::write(hook_file(d.path()), "#!/bin/sh\necho mine\n").unwrap_or_else(|e| panic!("{e}"));
    let out = line(&Process, d.path(), Mode::Apply);
    assert!(out.starts_with("  ! kept     post-commit hook"), "{out}");
    assert_eq!(
        fs::read_to_string(hook_file(d.path())).unwrap_or_default(),
        "#!/bin/sh\necho mine\n"
    );
}

#[test]
fn a_folder_that_is_not_a_repository_is_skipped_with_a_reason() {
    let d = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let out = line(&Process, d.path(), Mode::Apply);
    assert!(out.starts_with("  - skipped  post-commit hook: "), "{out}");
}
