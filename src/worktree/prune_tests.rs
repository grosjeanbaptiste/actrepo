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
