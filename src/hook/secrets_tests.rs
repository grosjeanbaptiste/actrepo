use super::*;
use crate::scaffold::native::Process;
use std::fs;
use std::path::Path;
use std::process::Command;

// Shaped like a GitHub token, made up for this test: not a credential.
const TOKEN: &str = "ghp_aB3dE6gH9jK2mN5pQ8sT1vW4yZ7bC0eF3hJ6";

fn git(dir: &Path, args: &[&str]) {
    let o = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(o.status.success(), "{args:?}: {o:?}");
}

fn repo() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    git(d.path(), &["init", "-q", "-b", "main"]);
    git(d.path(), &["config", "user.email", "t@example.org"]);
    git(d.path(), &["config", "user.name", "t"]);
    d
}

fn write(dir: &Path, name: &str, text: &str) {
    fs::write(dir.join(name), text).unwrap_or_else(|e| panic!("{e}"));
    git(dir, &["add", name]);
}

struct NoGitleaks;
impl Exec for NoGitleaks {
    fn run(&self, prog: &str, _: &[&str], _: Option<&Path>) -> Result<String> {
        anyhow::bail!("{prog}: No such file or directory")
    }
}

#[test]
fn a_clean_staged_file_passes() {
    let d = repo();
    write(d.path(), "a.txt", "nothing to hide\n");
    scan(&Process, d.path(), Mode::Staged).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn a_staged_secret_is_refused_and_the_message_names_the_scan() {
    let d = repo();
    write(d.path(), "a.env", &format!("GITHUB_TOKEN={TOKEN}\n"));
    let err = scan(&Process, d.path(), Mode::Staged)
        .err()
        .unwrap_or_else(|| panic!("a secret must fail the scan"));
    assert!(err.to_string().contains("secret scan"), "{err}");
}

#[test]
fn an_unpushed_commit_with_a_secret_is_refused() {
    let d = repo();
    write(d.path(), "a.env", &format!("GITHUB_TOKEN={TOKEN}\n"));
    git(d.path(), &["commit", "-q", "-m", "oops"]);
    assert!(scan(&Process, d.path(), Mode::Unpushed).is_err());
}

#[test]
fn a_commit_already_on_a_remote_is_not_scanned_again() {
    let d = repo();
    let remote = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    git(remote.path(), &["init", "-q", "--bare", "-b", "main"]);
    let url = remote.path().to_string_lossy().into_owned();
    write(d.path(), "a.env", &format!("GITHUB_TOKEN={TOKEN}\n"));
    git(d.path(), &["commit", "-q", "-m", "old"]);
    git(d.path(), &["remote", "add", "origin", &url]);
    git(d.path(), &["push", "-q", "origin", "main"]);
    scan(&Process, d.path(), Mode::Unpushed).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn a_missing_gitleaks_is_an_error_naming_it_never_a_pass() {
    let d = repo();
    let err = scan(&NoGitleaks, d.path(), Mode::Staged)
        .err()
        .unwrap_or_else(|| panic!("no gitleaks must not pass"));
    assert!(err.to_string().contains("gitleaks"), "{err}");
}

#[test]
fn the_stubs_only_call_the_rust_binary() {
    assert!(PRE_COMMIT.contains("exec actrepo hook secrets --staged"));
    assert!(PRE_PUSH.contains("exec actrepo hook secrets\n"));
    assert!(PRE_COMMIT.starts_with("#!/bin/sh\n"));
    assert!(PRE_PUSH.starts_with("#!/bin/sh\n"));
}
