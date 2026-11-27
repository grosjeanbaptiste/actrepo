//! `actrepo move` and `actrepo rename` tell `actparc` where the folder went (fake `actparc`).
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn fake(dir: &Path, body: &str) -> PathBuf {
    let p = dir.join("fake-actparc");
    ok(fs::write(&p, body));
    ok(fs::set_permissions(&p, fs::Permissions::from_mode(0o755)));
    p
}

fn logging(dir: &Path) -> (PathBuf, PathBuf) {
    let log = dir.join("log");
    (
        fake(
            dir,
            &format!("#!/bin/sh\necho \"$@\" >> '{}'\n", log.display()),
        ),
        log,
    )
}

fn run(dir: &Path, actparc: &Path, args: &[&str]) -> Output {
    ok(Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .args(args)
        .current_dir(dir)
        .env("HOME", dir)
        .env("DOTFILES_STATE_ROOT", dir.join("state"))
        .env("ACTREPO_ACTPARC", actparc)
        .output())
}

fn tree(t: &Path) {
    ok(fs::create_dir_all(t.join("work/proj")));
    ok(fs::create_dir_all(t.join("work/dest")));
}

#[test]
fn move_tells_actparc_the_old_and_the_new_folder() {
    let t = ok(tempfile::tempdir());
    tree(t.path());
    let (p, log) = logging(t.path());
    let w = t.path().join("work");
    let o = run(&w, &p, &["move", "proj", "--into", "dest"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let real = ok(w.canonicalize());
    let want = format!("move {0}/proj {0}/dest/proj\n", real.display());
    assert_eq!(ok(fs::read_to_string(log)), want);
}

#[test]
fn a_dry_run_move_does_not_call_actparc() {
    let t = ok(tempfile::tempdir());
    tree(t.path());
    let (p, log) = logging(t.path());
    let o = run(
        &t.path().join("work"),
        &p,
        &["move", "proj", "--into", "dest", "--dry-run"],
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(!log.exists());
}

#[test]
fn a_failing_actparc_warns_and_the_move_still_succeeds() {
    let t = ok(tempfile::tempdir());
    tree(t.path());
    let p = fake(t.path(), "#!/bin/sh\necho 'disk full' >&2\nexit 1\n");
    let w = t.path().join("work");
    let o = run(&w, &p, &["move", "proj", "--into", "dest"]);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("disk full"));
    assert!(w.join("dest/proj").is_dir());
}

#[test]
fn rename_tells_actparc_the_old_and_the_new_folder() {
    let t = ok(tempfile::tempdir());
    let w = t.path().join("work");
    ok(fs::create_dir_all(w.join("old")));
    let git = |a: &[&str]| {
        ok(Command::new("git")
            .args(a)
            .current_dir(w.join("old"))
            .output())
    };
    git(&["init", "-q"]);
    git(&["remote", "add", "origin", "git@github.com:me/old.git"]);
    let (p, log) = logging(t.path());
    let o = run(
        &w,
        &p,
        &["rename", "me/old", "new", "--dir", "old", "--local-only"],
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let line = ok(fs::read_to_string(log));
    assert!(
        line.starts_with("move ") && line.trim_end().ends_with("/work/new"),
        "{line}"
    );
    assert!(line.contains("/work/old "), "{line}");
}
