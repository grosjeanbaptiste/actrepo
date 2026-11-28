use super::*;
use crate::hook::secrets::{PRE_COMMIT, PRE_PUSH};
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

fn read(d: &Path, name: &str) -> String {
    fs::read_to_string(d.join(".git/hooks").join(name)).unwrap_or_default()
}

#[test]
fn the_three_hooks_are_written_executable() {
    let d = repo();
    let out = install_all(&Process, d.path(), false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out.len(), 3, "{out:?}");
    assert_eq!(read(d.path(), "post-commit"), HOOK);
    assert_eq!(read(d.path(), "pre-commit"), PRE_COMMIT);
    assert_eq!(read(d.path(), "pre-push"), PRE_PUSH);
    for n in ["post-commit", "pre-commit", "pre-push"] {
        let m = fs::metadata(d.path().join(".git/hooks").join(n))
            .map(|m| m.permissions().mode() & 0o777)
            .unwrap_or(0);
        assert_eq!(m, 0o755, "{n}");
    }
}

#[test]
fn installing_all_twice_changes_nothing() {
    let d = repo();
    install_all(&Process, d.path(), false).unwrap_or_else(|e| panic!("{e}"));
    let again = install_all(&Process, d.path(), false).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        again.iter().all(|i| matches!(i, Installed::Unchanged(_))),
        "{again:?}"
    );
}

#[test]
fn another_pre_push_hook_is_kept_and_named_while_the_others_are_written() {
    let d = repo();
    let dir = d.path().join(".git/hooks");
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
    fs::write(dir.join("pre-push"), "#!/bin/sh\necho mine\n").unwrap_or_else(|e| panic!("{e}"));
    let err = install_all(&Process, d.path(), false)
        .err()
        .unwrap_or_else(|| panic!("a foreign hook must be reported"));
    assert!(err.to_string().contains("another pre-push hook"), "{err}");
    assert_eq!(read(d.path(), "pre-push"), "#!/bin/sh\necho mine\n");
    assert_eq!(read(d.path(), "pre-commit"), PRE_COMMIT);
    assert_eq!(read(d.path(), "post-commit"), HOOK);
}

#[test]
fn every_foreign_hook_is_named_in_one_error() {
    let d = repo();
    let dir = d.path().join(".git/hooks");
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
    for n in ["pre-commit", "pre-push"] {
        fs::write(dir.join(n), "#!/bin/sh\necho mine\n").unwrap_or_else(|e| panic!("{e}"));
    }
    let err = install_all(&Process, d.path(), false)
        .err()
        .unwrap_or_else(|| panic!("foreign hooks must be reported"))
        .to_string();
    assert!(err.contains("another pre-commit hook"), "{err}");
    assert!(err.contains("another pre-push hook"), "{err}");
}

#[test]
fn a_dry_run_writes_none_of_the_three() {
    let d = repo();
    let out = install_all(&Process, d.path(), true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out.len(), 3);
    for n in ["post-commit", "pre-commit", "pre-push"] {
        assert!(!d.path().join(".git/hooks").join(n).exists(), "{n}");
    }
}
