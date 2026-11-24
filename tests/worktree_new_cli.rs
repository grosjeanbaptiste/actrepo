//! `actrepo worktree new` end to end: the folder is printed, exists, and the branch tracks nothing.
use std::fs;
use std::path::Path;
use std::process::Command;

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = ok(Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .current_dir(dir)
        .output());
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn repo() -> (tempfile::TempDir, std::path::PathBuf) {
    let d = ok(tempfile::tempdir());
    let base = ok(d.path().canonicalize());
    let (work, remote) = (base.join("work"), base.join("remote.git"));
    for p in [&work, &remote] {
        ok(fs::create_dir_all(p));
    }
    git(&remote, &["init", "-q", "--bare", "-b", "main"]);
    git(&work, &["init", "-q", "-b", "main"]);
    ok(fs::write(work.join("one"), "one"));
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "one"]);
    git(
        &work,
        &["remote", "add", "origin", &remote.display().to_string()],
    );
    git(&work, &["push", "-q", "-u", "origin", "main"]);
    (d, work)
}

#[test]
fn it_prints_the_new_worktree_and_creates_it() {
    let (_d, work) = repo();
    let o = ok(Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .args(["worktree", "new", "feat/x"])
        .current_dir(&work)
        .output());
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let printed = String::from_utf8_lossy(&o.stdout).trim().to_string();
    assert_eq!(Path::new(&printed), work.join(".worktrees/feat-x"));
    assert_eq!(
        git(Path::new(&printed), &["branch", "--show-current"]),
        "feat/x"
    );
}

#[test]
fn a_base_that_does_not_exist_fails_and_names_it() {
    let (_d, work) = repo();
    let o = ok(Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .args(["worktree", "new", "feat/x", "--base", "nope"])
        .current_dir(&work)
        .output());
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("origin/nope"));
}
