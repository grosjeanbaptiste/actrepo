use super::*;
use crate::scaffold::native::{Process, Quiet};
use std::fs;
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
    base: PathBuf,
}

/// `work` is on main; the remote is the origin.
fn world() -> World {
    let d = ok(tempfile::tempdir());
    let base = ok(d.path().canonicalize());
    let (work, remote) = (base.join("work"), base.join("remote.git"));
    for p in [&work, &remote] {
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
    World {
        _d: d,
        work,
        remote,
        base,
    }
}

/// A worktree on a new branch `name` with one commit, pushed (with upstream) unless `push` is false.
fn feature(w: &World, name: &str, push: bool) -> PathBuf {
    let wt = w.base.join(name);
    git(
        &w.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            name,
            &wt.display().to_string(),
            "main",
        ],
    );
    commit(&wt, name);
    if push {
        git(&wt, &["push", "-q", "-u", "origin", name]);
    }
    wt
}

/// The remote merges `name` into main, as a PR merge does.
fn merge_on_remote(w: &World, name: &str) {
    let helper = w.base.join(format!("helper-{name}"));
    ok(fs::create_dir_all(&helper));
    git(
        &helper,
        &["clone", "-q", &w.remote.display().to_string(), "."],
    );
    git(
        &helper,
        &[
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "merge",
            &format!("origin/{name}"),
        ],
    );
    git(&helper, &["push", "-q", "origin", "main"]);
}

fn prune_in(w: &World, dry_run: bool) -> (Result<usize>, String) {
    let mut out = Vec::new();
    let r = prune(&Quiet, &Process, &w.work, "main", dry_run, &mut out);
    (r, String::from_utf8_lossy(&out).into_owned())
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
fn a_clean_worktree_of_a_merged_branch_is_removed_with_its_branch() {
    let w = world();
    let wt = feature(&w, "feat", true);
    merge_on_remote(&w, "feat");
    let (r, out) = prune_in(&w, false);
    assert_eq!(ok(r), 1, "{out}");
    assert!(!wt.exists() && !branch_exists(&w.work, "feat"), "{out}");
}

#[test]
fn a_merged_worktree_with_local_changes_is_kept_and_named() {
    let w = world();
    let wt = feature(&w, "feat", true);
    merge_on_remote(&w, "feat");
    ok(fs::write(wt.join("scratch"), "wip"));
    let (r, out) = prune_in(&w, false);
    assert_eq!(ok(r), 0);
    assert!(
        out.contains("kept") && out.contains("local changes"),
        "{out}"
    );
    assert!(wt.join("scratch").exists() && branch_exists(&w.work, "feat"));
}

#[test]
fn an_unmerged_branch_is_left_alone() {
    let w = world();
    let wt = feature(&w, "feat", true);
    let (r, _) = prune_in(&w, false);
    assert_eq!(ok(r), 0);
    assert!(wt.exists() && branch_exists(&w.work, "feat"));
}

#[test]
fn a_fresh_branch_at_the_base_tip_is_not_taken_for_merged() {
    let w = world();
    let wt = w.base.join("fresh");
    git(
        &w.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "fresh",
            &wt.display().to_string(),
            "main",
        ],
    );
    git(&wt, &["push", "-q", "-u", "origin", "fresh"]);
    let (r, _) = prune_in(&w, false);
    assert_eq!(ok(r), 0);
    assert!(wt.exists());
}

#[test]
fn a_branch_never_pushed_is_not_removed_even_when_contained_in_the_base() {
    let w = world();
    let wt = feature(&w, "local", false);
    git(&w.work, &["merge", "-q", "local"]);
    git(&w.work, &["push", "-q", "origin", "main"]);
    let (r, _) = prune_in(&w, false);
    assert_eq!(ok(r), 0);
    assert!(wt.exists());
}

#[test]
fn a_dry_run_reports_and_touches_nothing() {
    let w = world();
    let wt = feature(&w, "feat", true);
    merge_on_remote(&w, "feat");
    let (r, out) = prune_in(&w, true);
    assert_eq!(ok(r), 1);
    assert!(out.contains("would remove"), "{out}");
    assert!(wt.exists() && branch_exists(&w.work, "feat"));
}

#[test]
fn the_worktree_it_runs_in_is_never_removed() {
    let w = world();
    let wt = feature(&w, "feat", true);
    merge_on_remote(&w, "feat");
    let mut out = Vec::new();
    let r = prune(&Quiet, &Process, &wt, "main", false, &mut out);
    assert_eq!(ok(r), 0);
    assert!(wt.exists());
}

#[test]
fn a_failing_fetch_is_an_error_naming_it() {
    let w = world();
    git(
        &w.work,
        &["remote", "set-url", "origin", "/nonexistent/remote.git"],
    );
    let (r, _) = prune_in(&w, false);
    assert!(
        format!("{:#}", r.err().unwrap_or_else(|| panic!("prune succeeded")))
            .contains("git fetch -p")
    );
}

#[test]
fn a_worktree_whose_folder_is_gone_is_skipped_not_an_error() {
    let w = world();
    let wt = feature(&w, "feat", true);
    merge_on_remote(&w, "feat");
    ok(fs::remove_dir_all(&wt));
    let (r, out) = prune_in(&w, false);
    assert_eq!(ok(r), 0, "{out}");
}

#[test]
fn a_merged_branch_is_deleted_even_when_the_local_head_does_not_contain_it() {
    let w = world();
    feature(&w, "feat", true);
    merge_on_remote(&w, "feat");
    git(&w.work, &["push", "-q", "origin", "--delete", "feat"]); // upstream gone, local main still behind
    let (r, out) = prune_in(&w, false);
    assert_eq!(ok(r), 1, "{out}");
    assert!(!branch_exists(&w.work, "feat"), "{out}");
}

/// Real git; any other program (here `actinstall`) is recorded, never run.
struct Spy(std::cell::RefCell<Vec<String>>);

impl Exec for Spy {
    fn run(&self, prog: &str, args: &[&str], cwd: Option<&Path>) -> Result<String> {
        if prog == "git" {
            return Process.run(prog, args, cwd);
        }
        self.0.borrow_mut().push(format!("{prog} {}", args.join(" ")));
        Ok(String::new())
    }
}

fn prune_command(w: &World, dry_run: bool) -> (Result<()>, Vec<String>) {
    let spy = Spy(Default::default());
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let r = crate::worktree::command::prune_and_refresh(&Quiet, &spy, &w.work, "main", dry_run, &mut out, &mut err);
    (r, spy.0.into_inner())
}

#[test]
fn a_pruned_worktree_asks_actinstall_to_update_the_repository() {
    let w = world();
    feature(&w, "feat", true);
    merge_on_remote(&w, "feat");
    let (r, calls) = prune_command(&w, false);
    ok(r);
    let main = ok(w.work.canonicalize());
    assert_eq!(calls, [format!("actinstall update --repo {}", main.display())]);
}

#[test]
fn nothing_pruned_or_a_dry_run_installs_nothing() {
    let w = world();
    let (r, calls) = prune_command(&w, false);
    ok(r);
    assert!(calls.is_empty());
    feature(&w, "feat", true);
    merge_on_remote(&w, "feat");
    let (r, calls) = prune_command(&w, true);
    ok(r);
    assert!(calls.is_empty());
}

fn detached_at(w: &World, name: &str, rev: &str) -> PathBuf {
    let wt = w.base.join(name);
    git(&w.work, &["worktree", "add", "-q", "--detach", &wt.display().to_string(), rev]);
    wt
}

#[test]
fn a_clean_worktree_detached_in_origin_base_holds_nothing_and_goes() {
    let w = world();
    let wt = detached_at(&w, "left-over", "origin/main");
    let (r, out) = prune_in(&w, false);
    assert_eq!(ok(r), 1, "{out}");
    assert!(!wt.exists(), "{out}");
}

#[test]
fn a_detached_worktree_with_its_own_commit_or_changes_is_kept() {
    let w = world();
    let behind = detached_at(&w, "behind", "origin/main");
    commit(&behind, "mine"); // detached, but no longer at origin/main
    let dirty = detached_at(&w, "dirty", "origin/main");
    ok(fs::write(dirty.join("scratch"), "wip"));
    let (r, out) = prune_in(&w, false);
    assert_eq!(ok(r), 0, "{out}");
    assert!(behind.exists() && dirty.exists());
    assert!(out.contains("kept") && out.contains("dirty"), "{out}");
}

#[test]
fn a_worktree_detached_at_an_older_commit_of_origin_base_goes_too() {
    let w = world();
    feature(&w, "feat", true);
    merge_on_remote(&w, "feat");
    git(&w.work, &["fetch", "-q"]);
    let old = detached_at(&w, "old", "main"); // the commit before the merge, contained in origin/main
    let (r, out) = prune_in(&w, false);
    assert!(ok(r) >= 1, "{out}");
    assert!(!old.exists(), "{out}");
}

/// A merged, pushed branch whose worktree is already gone: `pr merge` of a peer left it behind.
fn orphan_branch(w: &World, name: &str, push: bool) {
    let wt = feature(w, name, push);
    git(
        &w.work,
        &["worktree", "remove", "--force", &wt.display().to_string()],
    );
}

#[test]
fn a_merged_branch_with_no_worktree_is_deleted() {
    let w = world();
    orphan_branch(&w, "gone", true);
    merge_on_remote(&w, "gone");
    let (r, out) = prune_in(&w, false);
    assert_eq!(ok(r), 1, "{out}");
    assert!(!branch_exists(&w.work, "gone"), "{out}");
    assert!(out.contains("deleted branch gone"), "{out}");
}

#[test]
fn dry_run_names_the_merged_branch_and_keeps_it() {
    let w = world();
    orphan_branch(&w, "gone", true);
    merge_on_remote(&w, "gone");
    let (r, out) = prune_in(&w, true);
    assert_eq!(ok(r), 1, "{out}");
    assert!(out.contains("would delete branch gone"), "{out}");
    assert!(branch_exists(&w.work, "gone"));
}

#[test]
fn an_unmerged_branch_with_no_worktree_is_kept() {
    let w = world();
    orphan_branch(&w, "other", true);
    let (r, _) = prune_in(&w, false);
    assert_eq!(ok(r), 0);
    assert!(branch_exists(&w.work, "other"));
}

#[test]
fn a_merged_branch_never_pushed_is_kept() {
    let w = world();
    orphan_branch(&w, "local", false);
    git(&w.work, &["merge", "-q", "--no-ff", "-m", "m", "local"]);
    git(&w.work, &["push", "-q", "origin", "main"]);
    let (r, _) = prune_in(&w, false);
    assert_eq!(ok(r), 0);
    assert!(branch_exists(&w.work, "local"));
}

#[test]
fn the_base_branch_and_the_checked_out_one_are_never_deleted() {
    let w = world();
    git(&w.work, &["push", "-q", "-u", "origin", "main"]);
    let (r, _) = prune_in(&w, false);
    assert_eq!(ok(r), 0);
    assert!(branch_exists(&w.work, "main"));
}

#[test]
fn dry_run_counts_and_names_a_detached_worktree_and_keeps_it() {
    let w = world();
    let wt = detached_at(&w, "left-over", "origin/main");
    let (r, out) = prune_in(&w, true);
    assert_eq!(ok(r), 1, "{out}");
    assert!(out.contains("would remove"), "{out}");
    assert!(wt.exists(), "{out}");
}

#[test]
fn a_never_pushed_branch_that_adds_nothing_to_the_base_is_removed() {
    let w = world();
    let wt = feature(&w, "local", false);
    // a net-empty branch: its own commit changes nothing against the base
    git(&wt, &["reset", "-q", "--hard", "main"]);
    git(&wt, &["commit", "-q", "--allow-empty", "-m", "nothing"]);
    let (r, out) = prune_in(&w, false);
    assert_eq!(ok(r), 1, "{out}");
    assert!(!wt.exists() && !branch_exists(&w.work, "local"));
}

#[test]
fn a_never_pushed_branch_with_work_missing_from_the_base_is_kept() {
    let w = world();
    let wt = feature(&w, "local", false);
    let (r, _) = prune_in(&w, false);
    assert_eq!(ok(r), 0);
    assert!(wt.exists() && branch_exists(&w.work, "local"));
}
