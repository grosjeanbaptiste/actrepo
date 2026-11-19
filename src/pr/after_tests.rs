use super::*;
use crate::scaffold::native::{Process, Quiet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
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

fn commit(dir: &Path, file: &str) {
    ok(fs::write(dir.join(file), file));
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", file]);
}

struct World {
    _d: tempfile::TempDir,
    work: PathBuf,
    remote: PathBuf,
    base: PathBuf, // the directory holding work/, remote.git, other/
}

/// `work` is on `feature`, whose PR was merged on the remote (main has one more commit).
fn world() -> World {
    let d = ok(tempfile::tempdir());
    let base = ok(d.path().canonicalize());
    let (work, remote, helper) = (
        base.join("work"),
        base.join("remote.git"),
        base.join("helper"),
    );
    for p in [&work, &remote, &helper] {
        ok(fs::create_dir_all(p));
    }
    git(&remote, &["init", "-q", "--bare", "-b", "main"]);
    git(&work, &["init", "-q", "-b", "main"]);
    commit(&work, "one");
    git(
        &work,
        &["remote", "add", "origin", &remote.display().to_string()],
    );
    git(&work, &["push", "-q", "-u", "origin", "main"]);
    git(&work, &["checkout", "-q", "-b", "feature"]);
    commit(&work, "feat");
    git(&work, &["push", "-q", "-u", "origin", "feature"]);
    // the "merge" happens on the remote, from another clone
    git(
        &helper,
        &["clone", "-q", &remote.display().to_string(), "."],
    );
    git(
        &helper,
        &[
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "merge feature",
            "origin/feature",
        ],
    );
    git(&helper, &["push", "-q", "origin", "main"]);
    World {
        _d: d,
        work,
        remote,
        base,
    }
}

/// Real git; any other program (here `actinstall`) is recorded, never run.
struct Spy {
    calls: std::cell::RefCell<Vec<String>>,
    fail: bool,
}

impl Spy {
    fn new(fail: bool) -> Self {
        Spy { calls: Default::default(), fail }
    }
}

impl Exec for Spy {
    fn run(&self, prog: &str, args: &[&str], cwd: Option<&Path>) -> Result<String> {
        if prog == "git" {
            return Process.run(prog, args, cwd);
        }
        self.calls.borrow_mut().push(format!("{prog} {}", args.join(" ")));
        if self.fail {
            anyhow::bail!("{prog} exploded");
        }
        Ok(String::new())
    }
}

fn update_in(_w: &World, dir: &Path) -> (Result<()>, String, String) {
    update_with(&Spy::new(false), dir)
}

fn update_with(spy: &Spy, dir: &Path) -> (Result<()>, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let r = update(&Quiet, spy, dir, "main", "feature", &mut out, &mut err);
    (
        r,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

fn branch_exists(dir: &Path, b: &str) -> bool {
    Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{b}"),
        ])
        .current_dir(dir)
        .status()
        .is_ok_and(|s| s.success())
}

#[test]
fn base_not_held_elsewhere_is_checked_out_pulled_and_the_head_branch_deleted() {
    let w = world();
    let (r, out, err) = update_in(&w, &w.work);
    assert!(r.is_ok(), "{err}");
    assert!(out.contains("Updating local branches..."), "{out}");
    assert_eq!(git(&w.work, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert_eq!(git(&w.work, &["log", "-1", "--format=%s"]), "merge feature");
    assert!(!branch_exists(&w.work, "feature"));
}

#[test]
fn base_held_by_another_worktree_is_fast_forwarded_there_and_here_is_detached() {
    let w = world();
    let main_wt = w.base.join("main-wt");
    // work moves off main so another worktree can hold it
    git(
        &w.work,
        &[
            "worktree",
            "add",
            "-q",
            &main_wt.display().to_string(),
            "main",
        ],
    );
    let (r, _, err) = update_in(&w, &w.work);
    assert!(r.is_ok(), "{err}");
    assert_eq!(
        git(&main_wt, &["log", "-1", "--format=%s"]),
        "merge feature"
    );
    assert_eq!(
        git(&w.work, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "HEAD",
        "detached"
    );
    assert!(!branch_exists(&w.work, "feature"));
}

#[test]
fn a_dirty_worktree_holding_the_base_is_left_alone_with_a_warning() {
    let w = world();
    let main_wt = w.base.join("main-wt");
    git(
        &w.work,
        &[
            "worktree",
            "add",
            "-q",
            &main_wt.display().to_string(),
            "main",
        ],
    );
    ok(fs::write(main_wt.join("scratch"), "wip"));
    let (r, _, err) = update_in(&w, &w.work);
    assert!(
        err.contains("warning: PR merged; main is not updated: its worktree")
            && err.contains("has local changes"),
        "{err}"
    );
    assert!(r.is_err());
    assert_eq!(git(&main_wt, &["log", "-1", "--format=%s"]), "one");
    assert!(main_wt.join("scratch").exists());
}

#[test]
fn a_head_branch_that_cannot_be_deleted_is_reported_not_swallowed() {
    let w = world();
    git(&w.work, &["checkout", "-q", "feature"]);
    commit(&w.work, "unmerged-local-work"); // not on main: the merge check refuses before `branch -d`
    let (r, _, err) = update_in(&w, &w.work);
    assert!(
        err.contains("warning: PR merged; feature is not contained in main"),
        "{err}"
    );
    assert!(r.is_err());
    assert!(branch_exists(&w.work, "feature"));
}

#[test]
fn a_head_branch_already_gone_is_not_a_failure() {
    let w = world();
    git(&w.work, &["checkout", "-q", "main"]);
    git(&w.work, &["branch", "-D", "feature"]);
    let (r, _, err) = update_in(&w, &w.work);
    assert!(r.is_ok(), "{err}");
    assert!(!err.contains("not deleted"), "{err}");
}

#[test]
fn a_failing_worktree_lookup_still_says_the_pr_was_merged() {
    struct NoWorktrees;
    impl Exec for NoWorktrees {
        fn run(&self, p: &str, a: &[&str], c: Option<&Path>) -> Result<String> {
            if a.contains(&"worktree") {
                anyhow::bail!("worktree list failed");
            }
            Quiet.run(p, a, c)
        }
    }
    let w = world();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let r = update(
        &NoWorktrees,
        &Process,
        &w.work,
        "main",
        "feature",
        &mut out,
        &mut err,
    );
    assert!(r.is_err());
    let err = String::from_utf8_lossy(&err);
    assert!(
        err.contains(
            "warning: PR merged; the worktrees could not be listed, main was not updated."
        ),
        "{err}"
    );
    assert_eq!(
        git(&w.work, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "feature"
    );
}

#[test]
fn a_failing_fetch_says_the_pr_was_merged_and_stops() {
    let w = world();
    git(
        &w.work,
        &["remote", "set-url", "origin", "/nonexistent/remote.git"],
    );
    let (r, _, err) = update_in(&w, &w.work);
    assert!(
        err.contains("warning: PR merged; git fetch failed."),
        "{err}"
    );
    assert!(r.is_err());
    assert_eq!(
        git(&w.work, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "feature"
    );
    let _ = &w.remote;
}

#[test]
fn a_failing_base_checkout_is_reported() {
    let w = world();
    ok(fs::write(
        w.work.join("one"),
        "local edit that blocks the checkout",
    ));
    git(&w.work, &["checkout", "-q", "-b", "other"]); // keeps the edit
    ok(fs::write(w.work.join("one"), "conflicting"));
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let r = update(
        &Quiet,
        &Process,
        &w.work,
        "nonexistent-base",
        "feature",
        &mut out,
        &mut err,
    );
    assert!(r.is_err());
    assert!(
        String::from_utf8_lossy(&err)
            .contains("warning: PR merged; nonexistent-base was not updated."),
        "{}",
        String::from_utf8_lossy(&err)
    );
}

#[test]
fn a_worktree_on_another_branch_does_not_hold_the_base() {
    let w = world();
    let other = w.base.join("other-wt");
    git(
        &w.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "other",
            &other.display().to_string(),
            "main",
        ],
    );
    let (r, _, err) = update_in(&w, &w.work);
    assert!(r.is_ok(), "{err}");
    assert_eq!(git(&w.work, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert_eq!(git(&w.work, &["log", "-1", "--format=%s"]), "merge feature");
}

#[test]
fn the_checkout_running_the_update_is_not_its_own_holder() {
    let w = world();
    git(&w.work, &["checkout", "-q", "main"]); // base is checked out right here
    let (r, _, err) = update_in(&w, &w.work);
    assert!(r.is_ok(), "{err}");
    assert_eq!(
        git(&w.work, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "main",
        "not detached"
    );
    assert_eq!(git(&w.work, &["log", "-1", "--format=%s"]), "merge feature");
}

/// `work` leaves `feature`, which another worktree then holds.
fn head_held_elsewhere(w: &World) -> PathBuf {
    let feature_wt = w.base.join("feature-wt");
    git(&w.work, &["checkout", "-q", "main"]);
    git(
        &w.work,
        &[
            "worktree",
            "add",
            "-q",
            &feature_wt.display().to_string(),
            "feature",
        ],
    );
    feature_wt
}

#[test]
fn a_clean_worktree_holding_the_merged_head_is_removed_with_its_branch() {
    let w = world();
    let feature_wt = head_held_elsewhere(&w);
    let (r, _, err) = update_in(&w, &w.work);
    assert!(r.is_ok(), "{err}");
    assert!(!feature_wt.exists(), "worktree still there");
    assert!(!branch_exists(&w.work, "feature"));
}

#[test]
fn a_dirty_worktree_holding_the_merged_head_is_kept_with_a_warning() {
    let w = world();
    let feature_wt = head_held_elsewhere(&w);
    ok(fs::write(feature_wt.join("scratch"), "wip"));
    let (r, _, err) = update_in(&w, &w.work);
    assert!(r.is_err());
    assert!(
        err.contains("warning: PR merged; worktree") && err.contains("has local changes"),
        "{err}"
    );
    assert!(feature_wt.join("scratch").exists());
    assert!(branch_exists(&w.work, "feature"));
}

#[test]
fn another_merged_worktree_is_pruned_by_the_same_call() {
    let w = world();
    git(&w.work, &["checkout", "-q", "main"]);
    let other = w.base.join("older-wt");
    git(
        &w.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "older",
            &other.display().to_string(),
            "main",
        ],
    );
    commit(&other, "older");
    git(&other, &["push", "-q", "-u", "origin", "older"]);
    let helper = w.base.join("helper");
    git(&helper, &["pull", "-q"]);
    git(
        &helper,
        &[
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "merge older",
            "origin/older",
        ],
    );
    git(&helper, &["push", "-q", "origin", "main"]);
    let (r, out, err) = update_in(&w, &w.work);
    assert!(r.is_ok(), "{err}");
    assert!(
        out.contains("removed") && !other.exists() && !branch_exists(&w.work, "older"),
        "{out}"
    );
}

fn remote_has(w: &World, branch: &str) -> bool {
    !git(&w.work, &["ls-remote", "--heads", "origin", branch]).is_empty()
}

#[test]
fn the_remote_head_branch_still_there_after_the_merge_is_deleted() {
    let w = world();
    assert!(remote_has(&w, "feature"));
    let (r, _, err) = update_in(&w, &w.work);
    assert!(r.is_ok(), "{err}");
    assert!(!remote_has(&w, "feature"));
}

#[test]
fn a_remote_head_that_refuses_deletion_is_reported() {
    let w = world();
    git(
        &w.work,
        &[
            "remote",
            "set-url",
            "--push",
            "origin",
            "/nonexistent/remote.git",
        ],
    );
    let (r, _, err) = update_in(&w, &w.work);
    assert!(r.is_err());
    assert!(
        err.contains("warning: PR merged; remote branch feature was not deleted"),
        "{err}"
    );
}

#[test]
fn a_worktree_whose_folder_is_gone_does_not_hold_the_base() {
    let w = world();
    let gone = w.base.join("gone-wt");
    // work leaves main so a second worktree can hold it, then that folder disappears
    git(&w.work, &["worktree", "add", "-q", &gone.display().to_string(), "main"]);
    ok(fs::remove_dir_all(&gone));
    let (r, _, err) = update_in(&w, &w.work);
    assert!(r.is_ok(), "{err}");
    assert_eq!(git(&w.work, &["log", "-1", "--format=%s"]), "merge feature");
    assert!(!branch_exists(&w.work, "feature"));
}

#[test]
fn a_verified_merge_asks_actinstall_to_update_the_main_worktree() {
    let w = world();
    let spy = Spy::new(false);
    let (r, _, err) = update_with(&spy, &w.work);
    assert!(r.is_ok(), "{err}");
    let main = ok(w.work.canonicalize());
    assert_eq!(*spy.calls.borrow(), [format!("actinstall update --repo {}", main.display())]);
}

#[test]
fn a_failing_actinstall_is_a_warning_and_a_failed_update() {
    let w = world();
    let (r, _, err) = update_with(&Spy::new(true), &w.work);
    assert!(r.is_err());
    assert!(err.contains("warning: actinstall update failed for") && err.contains("actinstall exploded"), "{err}");
}

#[test]
fn an_unverified_merge_does_not_install_anything() {
    let w = world();
    git(&w.work, &["checkout", "-q", "feature"]);
    commit(&w.work, "unmerged");
    let spy = Spy::new(false);
    let (r, _, err) = update_with(&spy, &w.base.join("work"));
    assert!(r.is_err(), "{err}");
    assert!(spy.calls.borrow().is_empty(), "{:?}", spy.calls);
}

#[test]
fn merging_from_the_copy_removes_the_copy_with_its_branch() {
    let w = world();
    let copy = head_held_elsewhere(&w);
    let (r, out, err) = update_in(&w, &copy);
    assert!(r.is_ok(), "{err}");
    assert!(!copy.exists(), "the copy the merge ran from is still there");
    assert!(!branch_exists(&w.work, "feature"));
    assert!(out.contains("Removed this worktree"), "{out}");
}

#[test]
fn the_copy_is_removed_after_actinstall_ran_in_it() {
    let w = world();
    let copy = head_held_elsewhere(&w);
    let spy = Spy::new(false);
    let (r, _, err) = update_with(&spy, &copy);
    assert!(r.is_ok(), "{err}");
    let main = ok(w.work.canonicalize());
    assert_eq!(*spy.calls.borrow(), [format!("actinstall update --repo {}", main.display())]);
}

#[test]
fn a_copy_with_local_changes_is_kept_with_a_warning() {
    let w = world();
    let copy = head_held_elsewhere(&w);
    ok(fs::write(copy.join("scratch"), "wip"));
    let (r, _, err) = update_in(&w, &copy);
    assert!(r.is_err());
    assert!(err.contains("has local changes, so it is kept"), "{err}");
    assert!(copy.join("scratch").exists());
}

#[test]
fn a_linked_worktree_that_was_not_on_the_head_is_not_removed() {
    let w = world();
    git(&w.work, &["checkout", "-q", "main"]);
    let other = w.base.join("other-copy");
    git(&w.work, &["worktree", "add", "-q", "-b", "other", &other.display().to_string(), "main"]);
    let (r, _, err) = update_in(&w, &other);
    assert!(r.is_ok(), "{err}");
    assert!(other.exists(), "a worktree the PR did not come from was removed");
}

#[test]
fn the_main_worktree_is_never_removed() {
    let w = world();
    let (r, _, err) = update_in(&w, &w.work); // main worktree, on the head
    assert!(r.is_ok(), "{err}");
    assert!(w.work.exists());
}
