use super::*;
use crate::scaffold::native::Process;
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

fn mode(p: &Path) -> u32 {
    fs::metadata(p)
        .map(|m| m.permissions().mode() & 0o777)
        .unwrap_or(0)
}

#[test]
fn the_hook_is_written_executable_and_calls_the_rust_binary() {
    let d = repo();
    let Installed::Written(file) =
        install(&Process, d.path(), false).unwrap_or_else(|e| panic!("{e}"))
    else {
        panic!("expected a write");
    };
    assert!(file.ends_with("hooks/post-commit"), "{}", file.display());
    assert_eq!(fs::read_to_string(&file).unwrap_or_default(), HOOK);
    assert_eq!(mode(&file), 0o755);
    assert!(HOOK.contains("exec actrepo hook run"));
}

#[test]
fn installing_twice_changes_nothing() {
    let d = repo();
    install(&Process, d.path(), false).unwrap_or_else(|e| panic!("{e}"));
    let again = install(&Process, d.path(), false).unwrap_or_else(|e| panic!("{e}"));
    assert!(matches!(again, Installed::Unchanged(_)), "{again:?}");
}

#[test]
fn another_post_commit_hook_is_never_overwritten() {
    let d = repo();
    let file = d.path().join(".git/hooks/post-commit");
    fs::create_dir_all(file.parent().unwrap_or(d.path())).unwrap_or_else(|e| panic!("{e}"));
    fs::write(&file, "#!/bin/sh\necho mine\n").unwrap_or_else(|e| panic!("{e}"));
    let err = install(&Process, d.path(), false)
        .err()
        .unwrap_or_else(|| panic!("expected an error"));
    assert!(
        err.to_string().contains("another post-commit hook"),
        "{err}"
    );
    assert_eq!(
        fs::read_to_string(&file).unwrap_or_default(),
        "#!/bin/sh\necho mine\n"
    );
}

#[test]
fn a_dry_run_writes_nothing() {
    let d = repo();
    let out = install(&Process, d.path(), true).unwrap_or_else(|e| panic!("{e}"));
    let Installed::Written(file) = out else {
        panic!("expected a plan")
    };
    assert!(!file.exists());
}

#[test]
fn a_folder_that_is_not_a_repository_is_an_error() {
    let d = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let err = install(&Process, d.path(), false)
        .err()
        .unwrap_or_else(|| panic!("expected an error"));
    assert!(err.to_string().contains("not a git repository"), "{err}");
}

#[test]
fn core_hooks_path_is_honoured() {
    let d = repo();
    let o = Command::new("git")
        .args(["config", "core.hooksPath", "githooks"])
        .current_dir(d.path())
        .output()
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(o.status.success());
    let Installed::Written(file) =
        install(&Process, d.path(), false).unwrap_or_else(|e| panic!("{e}"))
    else {
        panic!("expected a write");
    };
    assert!(file.ends_with("githooks/post-commit"), "{}", file.display());
}

#[test]
fn an_unreadable_hook_path_is_an_error_not_a_missing_hook() {
    let d = repo();
    let dir = d.path().join(".git/hooks/post-commit");
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
    let err = install(&Process, d.path(), false)
        .err()
        .unwrap_or_else(|| panic!("expected an error"));
    assert!(err.to_string().contains("reading"), "{err}");
}
