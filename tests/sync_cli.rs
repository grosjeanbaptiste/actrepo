//! `actrepo sync` end to end: real repositories, a bare remote, 3 branches to push and 3 to pull.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

fn commit(dir: &Path, file: &str) {
    ok(fs::write(dir.join(file), file));
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", file]);
}

struct Fx {
    _d: tempfile::TempDir,
    work: PathBuf,
    remote: PathBuf,
}

/// `work` holds p1..p3 ahead of origin and l1..l3 behind it (new commits pushed from `other`).
fn fx() -> Fx {
    let d = ok(tempfile::tempdir());
    let base = ok(d.path().canonicalize());
    let (work, remote, other) = (
        base.join("work"),
        base.join("remote.git"),
        base.join("other"),
    );
    for p in [&work, &remote, &other] {
        ok(fs::create_dir_all(p));
    }
    git(&remote, &["init", "-q", "--bare", "-b", "main"]);
    git(&work, &["init", "-q", "-b", "main"]);
    commit(&work, "base");
    git(
        &work,
        &["remote", "add", "origin", &remote.display().to_string()],
    );
    git(&work, &["push", "-q", "-u", "origin", "main"]);
    for n in ["p1", "p2", "p3", "l1", "l2", "l3"] {
        git(&work, &["push", "-q", "origin", &format!("main:{n}")]);
        git(
            &work,
            &["branch", "-q", "--track", n, &format!("origin/{n}")],
        );
    }
    git(&other, &["clone", "-q", &remote.display().to_string(), "."]);
    for n in ["l1", "l2", "l3"] {
        git(&other, &["checkout", "-q", n]);
        commit(&other, &format!("new-{n}"));
        git(&other, &["push", "-q", "origin", n]);
    }
    for n in ["p1", "p2", "p3"] {
        git(&work, &["checkout", "-q", n]);
        commit(&work, &format!("mine-{n}"));
    }
    git(&work, &["checkout", "-q", "main"]);
    Fx {
        _d: d,
        work,
        remote,
    }
}

fn sync(f: &Fx, args: &[&str]) -> Output {
    ok(Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .arg("sync")
        .args(args)
        .arg(&f.work)
        .output())
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn remote_sha(f: &Fx, branch: &str) -> String {
    git(&f.remote, &["rev-parse", branch])
}

#[test]
fn dry_run_prints_the_six_steps_and_changes_nothing() {
    let f = fx();
    let before: Vec<String> = ["p1", "l1"].iter().map(|b| remote_sha(&f, b)).collect();
    let o = sync(&f, &["--dry-run"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let t = out(&o);
    for n in ["p1", "p2", "p3"] {
        assert!(t.contains(&format!(" {n}: push 1")), "{t}");
    }
    for n in ["l1", "l2", "l3"] {
        assert!(t.contains(&format!(" {n}: pull 1")), "{t}");
    }
    let after: Vec<String> = ["p1", "l1"].iter().map(|b| remote_sha(&f, b)).collect();
    assert_eq!(before, after);
}

#[test]
fn three_pushes_and_three_pulls_are_settled_in_one_run() {
    let f = fx();
    let o = sync(&f, &[]);
    assert!(
        o.status.success(),
        "{}\n{}",
        out(&o),
        String::from_utf8_lossy(&o.stderr)
    );
    for n in ["p1", "p2", "p3"] {
        assert_eq!(
            remote_sha(&f, n),
            git(&f.work, &["rev-parse", n]),
            "{n} pushed"
        );
    }
    for n in ["l1", "l2", "l3"] {
        assert_eq!(
            git(&f.work, &["rev-parse", n]),
            remote_sha(&f, n),
            "{n} pulled"
        );
    }
    assert!(sync(&f, &["--dry-run"]).status.success());
    assert!(out(&sync(&f, &["--dry-run"])).contains("nothing to sync"));
}

#[test]
fn a_diverged_branch_is_reported_and_fails_the_run_but_the_rest_is_settled() {
    let f = fx();
    git(&f.work, &["checkout", "-q", "l1"]);
    commit(&f.work, "local-on-l1");
    git(&f.work, &["checkout", "-q", "main"]);
    let o = sync(&f, &[]);
    assert!(!o.status.success());
    assert!(
        out(&o).contains("l1: diverged 1 ahead, 1 behind"),
        "{}",
        out(&o)
    );
    assert_eq!(remote_sha(&f, "p1"), git(&f.work, &["rev-parse", "p1"]));
}

/// New commit on `main` pushed from the other clone, so `work`'s checked-out `main` is behind.
fn main_behind(f: &Fx) {
    let other = ok(f.work.parent().ok_or("no parent")).join("other");
    git(&other, &["checkout", "-q", "main"]);
    commit(&other, "new-main");
    git(&other, &["push", "-q", "origin", "main"]);
}

#[test]
fn a_dirty_checked_out_branch_that_is_behind_is_reported_not_pulled() {
    let f = fx();
    main_behind(&f);
    ok(fs::write(f.work.join("scratch"), "wip"));
    let before = git(&f.work, &["rev-parse", "main"]);
    let o = sync(&f, &[]);
    assert!(!o.status.success());
    assert!(out(&o).contains("main: skip, dirty tree"), "{}", out(&o));
    assert_eq!(git(&f.work, &["rev-parse", "main"]), before);
}

#[test]
fn a_clean_checked_out_branch_that_is_behind_is_pulled() {
    let f = fx();
    main_behind(&f);
    let o = sync(&f, &[]);
    assert!(
        o.status.success(),
        "{}\n{}",
        out(&o),
        String::from_utf8_lossy(&o.stderr)
    );
    assert_eq!(git(&f.work, &["rev-parse", "main"]), remote_sha(&f, "main"));
}

#[test]
fn pull_only_pulls_the_behind_branches_and_pushes_nothing() {
    let f = fx();
    let pushed_before: Vec<String> = ["p1", "p2", "p3"]
        .iter()
        .map(|b| remote_sha(&f, b))
        .collect();
    let o = sync(&f, &["--pull-only"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    for n in ["l1", "l2", "l3"] {
        assert_eq!(
            git(&f.work, &["rev-parse", n]),
            remote_sha(&f, n),
            "{n} pulled"
        );
    }
    let pushed_after: Vec<String> = ["p1", "p2", "p3"]
        .iter()
        .map(|b| remote_sha(&f, b))
        .collect();
    assert_eq!(pushed_before, pushed_after, "nothing may be pushed");
    let t = out(&sync(&f, &["--pull-only", "--dry-run"]));
    assert!(!t.contains(": push "), "{t}");
}

fn sync_in(dir: &Path, args: &[&str]) -> Output {
    ok(Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .arg("sync")
        .args(args)
        .arg(dir)
        .output())
}

#[test]
fn here_works_from_a_subfolder_and_pulls_only_the_enclosing_repository() {
    let f = fx();
    let sub = f.work.join("sub");
    ok(fs::create_dir_all(&sub));
    let nested = f.work.join("nested");
    ok(fs::create_dir_all(&nested));
    git(&nested, &["init", "-q", "-b", "main"]);
    commit(&nested, "n");
    let o = sync_in(&sub, &["--pull-only", "--here"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    for n in ["l1", "l2", "l3"] {
        assert_eq!(
            git(&f.work, &["rev-parse", n]),
            remote_sha(&f, n),
            "{n} pulled"
        );
    }
    let plan = out(&sync_in(&sub, &["--here", "--dry-run"]));
    assert!(
        !plan.contains("nested"),
        "nothing below the repository is scanned: {plan}"
    );
    let scanned = out(&sync_in(&f.work, &["--dry-run"]));
    assert!(
        scanned.contains("nested"),
        "without --here the nested repository is planned: {scanned}"
    );
}

#[test]
fn here_outside_a_repository_succeeds_and_says_so() {
    let d = ok(tempfile::tempdir());
    let o = sync_in(d.path(), &["--pull-only", "--here"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(out(&o).contains("not in a git repository"), "{}", out(&o));
}

#[test]
fn pull_only_notes_a_diverged_branch_and_still_succeeds_while_plain_sync_fails() {
    let f = fx();
    git(&f.work, &["checkout", "-q", "-b", "dv", "main"]);
    commit(&f.work, "local-dv");
    git(&f.work, &["push", "-q", "-u", "origin", "dv"]);
    let other = f.work.parent().map(|p| p.join("other")).unwrap_or_default();
    git(&other, &["fetch", "-q", "origin"]);
    git(&other, &["checkout", "-q", "-B", "dv", "origin/dv"]);
    commit(&other, "remote-dv");
    git(&other, &["push", "-q", "origin", "dv"]);
    git(&f.work, &["reset", "-q", "--soft", "HEAD~1"]);
    git(&f.work, &["commit", "-q", "-m", "rewritten-dv"]);
    git(&f.work, &["fetch", "-q", "origin"]);
    git(&f.work, &["checkout", "-q", "main"]);
    let o = sync(&f, &["--pull-only"]);
    assert!(
        o.status.success(),
        "{}\n{}",
        out(&o),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(out(&o).contains("note"), "{}", out(&o));
    assert!(out(&o).contains("dv: diverged"), "{}", out(&o));
    assert!(
        !sync(&f, &[]).status.success(),
        "plain sync still needs a human"
    );
}
