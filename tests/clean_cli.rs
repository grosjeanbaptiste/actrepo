//! `actrepo clean` end to end on scratch repositories (a bare origin in the temp folder). No network.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn some<T>(o: Option<T>, what: &str) -> T {
    o.unwrap_or_else(|| panic!("missing {what}"))
}

fn git(dir: &Path, args: &[&str]) {
    let o = ok(Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output());
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

fn branches(dir: &Path) -> String {
    let o = ok(Command::new("git")
        .args(["branch", "--list"])
        .current_dir(dir)
        .output());
    String::from_utf8_lossy(&o.stdout).to_string()
}

/// A repository on `main` with a pushed branch `feat/done`, merged, and its worktree.
fn merged_branch() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = ok(tempfile::tempdir());
    let work = root.path().join("work");
    ok(std::fs::create_dir_all(&work));
    git(
        root.path(),
        &["init", "-q", "--bare", "-b", "main", "origin.git"],
    );
    git(&work, &["init", "-q", "-b", "main"]);
    git(
        &work,
        &[
            "remote",
            "add",
            "origin",
            some(root.path().join("origin.git").to_str(), "utf-8 origin path"),
        ],
    );
    ok(std::fs::write(work.join("a.txt"), "base\n"));
    git(&work, &["add", "a.txt"]);
    git(&work, &["commit", "-q", "-m", "base"]);
    git(&work, &["push", "-q", "origin", "main"]);
    let wt = work.join(".worktrees/feat-done");
    git(
        &work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "feat/done",
            some(wt.to_str(), "utf-8 worktree path"),
            "main",
        ],
    );
    ok(std::fs::write(wt.join("d.txt"), "d\n"));
    git(&wt, &["add", "d.txt"]);
    git(&wt, &["commit", "-q", "-m", "work"]);
    git(&wt, &["push", "-q", "-u", "origin", "feat/done"]);
    git(
        &work,
        &["merge", "-q", "--no-ff", "-m", "merge", "feat/done"],
    );
    git(&work, &["push", "-q", "origin", "main"]);
    (root, work, wt)
}

fn clean(dir: &Path, extra: &[&str]) -> Output {
    ok(Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .arg("clean")
        .arg(dir)
        .args(extra)
        .output())
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).to_string()
}

#[test]
fn clean_removes_a_merged_branch_and_its_worktree() {
    let (_root, work, wt) = merged_branch();

    let out = clean(&work, &[]);

    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(
        text(&out.stdout).contains("removed feat/done"),
        "{}",
        text(&out.stdout)
    );
    assert!(!wt.exists());
    assert!(!branches(&work).contains("feat/done"));
}

#[test]
fn clean_dry_run_changes_nothing() {
    let (_root, work, wt) = merged_branch();

    let out = clean(&work, &["--dry-run"]);

    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(
        text(&out.stdout).contains("would remove feat/done"),
        "{}",
        text(&out.stdout)
    );
    assert!(wt.exists());
    assert!(branches(&work).contains("feat/done"));
}
