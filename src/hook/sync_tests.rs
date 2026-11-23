use super::*;
use crate::scaffold::native::Process;
use std::cell::RefCell;
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
    base: PathBuf,
}

/// `work` is on main with one pushed commit; `remote` is its origin.
fn world() -> World {
    let d = ok(tempfile::tempdir());
    let base = ok(d.path().canonicalize());
    let (work, remote) = (base.join("work"), base.join("remote.git"));
    ok(fs::create_dir_all(&work));
    ok(fs::create_dir_all(&remote));
    git(&remote, &["init", "-q", "--bare", "-b", "main"]);
    git(&work, &["init", "-q", "-b", "main"]);
    commit(&work, "one");
    let url = remote.display().to_string();
    git(&work, &["remote", "add", "origin", &url]);
    git(&work, &["push", "-q", "-u", "origin", "main"]);
    World {
        _d: d,
        work,
        remote,
        base,
    }
}

fn remote_head(w: &World, branch: &str) -> String {
    git(&w.remote, &["rev-parse", branch])
}

/// Runs the real git and remembers every command line.
struct Recording(RefCell<Vec<String>>);

impl Exec for Recording {
    fn run(&self, prog: &str, args: &[&str], cwd: Option<&Path>) -> Result<String> {
        self.0
            .borrow_mut()
            .push(format!("{prog} {}", args.join(" ")));
        Process.run(prog, args, cwd)
    }
}

#[test]
fn a_new_branch_is_pushed_with_its_upstream() {
    let w = world();
    git(&w.work, &["switch", "-q", "-c", "feat/x"]);
    commit(&w.work, "two");
    let out = ok(sync(&Process, &w.work));
    assert_eq!(out, Outcome::Synced { pulled: false });
    assert_eq!(
        remote_head(&w, "feat/x"),
        git(&w.work, &["rev-parse", "HEAD"])
    );
    assert_eq!(
        git(&w.work, &["rev-parse", "--abbrev-ref", "@{u}"]),
        "origin/feat/x"
    );
}

#[test]
fn a_new_commit_on_a_pushed_branch_is_pushed() {
    let w = world();
    git(&w.work, &["switch", "-q", "-c", "feat/x"]);
    commit(&w.work, "two");
    ok(sync(&Process, &w.work));
    commit(&w.work, "three");
    ok(sync(&Process, &w.work));
    assert_eq!(
        remote_head(&w, "feat/x"),
        git(&w.work, &["rev-parse", "HEAD"])
    );
}

#[test]
fn the_default_branch_is_never_pushed() {
    let w = world();
    let before = remote_head(&w, "main");
    commit(&w.work, "two");
    let out = ok(sync(&Process, &w.work));
    assert_eq!(out, Outcome::Skipped(Skip::DefaultBranch("main".into())));
    assert_eq!(remote_head(&w, "main"), before);
}

#[test]
fn a_detached_head_is_skipped() {
    let w = world();
    git(&w.work, &["switch", "-q", "--detach"]);
    let out = ok(sync(&Process, &w.work));
    assert_eq!(out, Outcome::Skipped(Skip::Detached));
}

#[test]
fn a_repository_without_origin_is_skipped() {
    let w = world();
    git(&w.work, &["switch", "-q", "-c", "feat/x"]);
    git(&w.work, &["remote", "remove", "origin"]);
    let out = ok(sync(&Process, &w.work));
    assert_eq!(out, Outcome::Skipped(Skip::NoOrigin));
}

#[test]
fn the_emvi_repositories_are_never_touched() {
    let w = world();
    git(&w.work, &["switch", "-q", "-c", "feat/x"]);
    let url = "git@github.com:MAIA-company/emvi-app.git";
    git(&w.work, &["remote", "set-url", "origin", url]);
    let rec = Recording(RefCell::new(vec![]));
    let out = ok(sync(&rec, &w.work));
    assert_eq!(out, Outcome::Skipped(Skip::Forbidden(url.into())));
    assert!(
        rec.0
            .borrow()
            .iter()
            .all(|c| !c.contains("fetch") && !c.contains("push")),
        "{:?}",
        rec.0.borrow()
    );
}

#[test]
fn a_branch_behind_its_upstream_is_fast_forwarded_before_the_push() {
    let w = world();
    git(&w.work, &["switch", "-q", "-c", "feat/x"]);
    commit(&w.work, "two");
    ok(sync(&Process, &w.work));
    let other = w.base.join("other");
    git(
        &w.base,
        &["clone", "-q", &w.remote.display().to_string(), "other"],
    );
    git(&other, &["switch", "-q", "feat/x"]);
    commit(&other, "peer");
    git(&other, &["push", "-q", "origin", "feat/x"]);
    let out = ok(sync(&Process, &w.work));
    assert_eq!(out, Outcome::Synced { pulled: true });
    assert_eq!(
        git(&w.work, &["rev-parse", "HEAD"]),
        remote_head(&w, "feat/x")
    );
}

#[test]
fn a_diverged_branch_fails_loudly_and_pushes_nothing() {
    let w = world();
    git(&w.work, &["switch", "-q", "-c", "feat/x"]);
    commit(&w.work, "two");
    ok(sync(&Process, &w.work));
    let other = w.base.join("other");
    git(
        &w.base,
        &["clone", "-q", &w.remote.display().to_string(), "other"],
    );
    git(&other, &["switch", "-q", "feat/x"]);
    commit(&other, "peer");
    git(&other, &["push", "-q", "origin", "feat/x"]);
    let peer = remote_head(&w, "feat/x");
    commit(&w.work, "mine");
    let err = sync(&Process, &w.work)
        .err()
        .unwrap_or_else(|| panic!("expected an error"));
    assert!(err.to_string().contains("merge"), "{err}");
    assert_eq!(remote_head(&w, "feat/x"), peer);
}

#[test]
fn an_unreachable_origin_is_an_error_naming_the_fetch() {
    let w = world();
    git(&w.work, &["switch", "-q", "-c", "feat/x"]);
    ok(fs::remove_dir_all(&w.remote));
    let err = sync(&Process, &w.work)
        .err()
        .unwrap_or_else(|| panic!("expected an error"));
    assert!(err.to_string().contains("fetch"), "{err}");
}

#[test]
fn git_is_never_forced_nor_made_to_skip_its_gate() {
    let w = world();
    git(&w.work, &["switch", "-q", "-c", "feat/x"]);
    commit(&w.work, "two");
    let rec = Recording(RefCell::new(vec![]));
    ok(sync(&rec, &w.work));
    let calls = rec.0.borrow();
    assert!(calls.iter().any(|c| c.contains("push")), "{calls:?}");
    for c in calls.iter() {
        for bad in ["--force", "-f ", "--no-verify", "--force-with-lease"] {
            assert!(!c.contains(bad), "{c}");
        }
    }
}
