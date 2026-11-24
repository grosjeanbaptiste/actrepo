use super::*;
use crate::scaffold::native::Process;
use std::fs;
use std::process::Command;

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn bad<T, E>(r: std::result::Result<T, E>) -> E {
    match r {
        Ok(_) => panic!("expected an error"),
        Err(e) => e,
    }
}

fn git_out(dir: &Path, args: &[&str]) -> std::process::Output {
    ok(Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .current_dir(dir)
        .output())
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = git_out(dir, args);
    assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

/// `work` is on main with one commit pushed to the bare `remote`.
fn world() -> (tempfile::TempDir, PathBuf) {
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
    git(&work, &["remote", "add", "origin", &remote.display().to_string()]);
    git(&work, &["push", "-q", "-u", "origin", "main"]);
    (d, work)
}

#[test]
fn the_worktree_goes_under_dot_worktrees_named_after_the_branch() {
    let (_d, work) = world();
    let path = ok(create(&Process, &work, "feat/x", "main"));
    assert_eq!(path, work.join(".worktrees/feat-x"));
    assert_eq!(git(&path, &["branch", "--show-current"]), "feat/x");
    assert_eq!(git(&path, &["rev-parse", "HEAD"]), git(&work, &["rev-parse", "origin/main"]));
}

#[test]
fn the_new_branch_tracks_nothing_so_pull_never_merges_main_into_it() {
    let (_d, work) = world();
    let path = ok(create(&Process, &work, "feat/x", "main"));
    let o = git_out(&path, &["config", "--get", "branch.feat/x.merge"]);
    assert!(!o.status.success(), "upstream set: {}", String::from_utf8_lossy(&o.stdout));
}

#[test]
fn it_starts_from_the_remote_base_not_the_local_one() {
    let (_d, work) = world();
    ok(fs::write(work.join("local"), "x"));
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "local only"]);
    let path = ok(create(&Process, &work, "feat/y", "main"));
    assert!(!path.join("local").exists());
}

#[test]
fn called_from_a_linked_worktree_it_still_lands_in_the_main_one() {
    let (_d, work) = world();
    let first = ok(create(&Process, &work, "feat/a", "main"));
    let second = ok(create(&Process, &first, "feat/b", "main"));
    assert_eq!(second, work.join(".worktrees/feat-b"));
}

#[test]
fn an_existing_branch_is_an_error_naming_it_and_changes_nothing() {
    let (_d, work) = world();
    git(&work, &["branch", "taken"]);
    let e = bad(create(&Process, &work, "taken", "main")).to_string();
    assert!(e.contains("taken"), "{e}");
    assert!(!work.join(".worktrees/taken").exists());
}

#[test]
fn an_unknown_base_is_an_error_naming_it() {
    let (_d, work) = world();
    let e = bad(create(&Process, &work, "feat/z", "nope")).to_string();
    assert!(e.contains("origin/nope"), "{e}");
}

#[test]
fn a_folder_that_is_not_a_repository_is_an_error() {
    let d = ok(tempfile::tempdir());
    let e = bad(create(&Process, d.path(), "feat/z", "main")).to_string();
    assert!(e.contains("git"), "{e}");
}
