use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

fn real(p: &Path) -> PathBuf {
    ok(p.canonicalize())
}

#[test]
fn a_move_lands_in_the_parent_under_its_own_name() {
    let t = ok(tempfile::tempdir());
    let (dir, parent) = (t.path().join("proj"), t.path().join("dest"));
    ok(fs::create_dir_all(&dir));
    ok(fs::create_dir_all(&parent));
    let got = move_target(&args(&["proj", "--into", "dest"]), t.path());
    assert_eq!(got, Some((real(&dir), real(&parent).join("proj"))));
}

#[test]
fn an_absolute_dir_and_other_flags_after_into_are_read() {
    let t = ok(tempfile::tempdir());
    let (dir, parent) = (t.path().join("proj"), t.path().join("dest"));
    ok(fs::create_dir_all(&dir));
    ok(fs::create_dir_all(&parent));
    let a = args(&[dir.to_str().unwrap_or(""), "--root", "x", "--into", parent.to_str().unwrap_or("")]);
    assert_eq!(move_target(&a, Path::new("/")), Some((real(&dir), real(&parent).join("proj"))));
}

#[test]
fn a_dry_run_moves_nothing() {
    let t = ok(tempfile::tempdir());
    ok(fs::create_dir_all(t.path().join("proj")));
    ok(fs::create_dir_all(t.path().join("dest")));
    assert_eq!(move_target(&args(&["proj", "--into", "dest", "--dry-run"]), t.path()), None);
}

#[test]
fn arguments_actautoma_would_refuse_give_no_target() {
    let t = ok(tempfile::tempdir());
    ok(fs::create_dir_all(t.path().join("proj")));
    assert_eq!(move_target(&args(&["proj"]), t.path()), None, "no --into");
    assert_eq!(move_target(&args(&["--into", "dest"]), t.path()), None, "no DIR");
    assert_eq!(move_target(&args(&["proj", "--into", "gone"]), t.path()), None, "no parent");
    assert_eq!(move_target(&args(&["gone", "--into", "."]), t.path()), None, "no folder");
}

fn script(dir: &Path, body: &str) -> PathBuf {
    let p = dir.join("actparc");
    ok(fs::write(&p, body));
    ok(fs::set_permissions(&p, fs::Permissions::from_mode(0o755)));
    p
}

#[test]
fn follow_runs_actparc_move_with_both_paths() {
    let t = ok(tempfile::tempdir());
    let log = t.path().join("log");
    let p = script(t.path(), &format!("#!/bin/sh\necho \"$@\" > '{}'\n", log.display()));
    let got = follow(p.to_str().unwrap_or(""), Path::new("/a b/old"), Path::new("/a b/new"));
    assert_eq!(got, None);
    assert_eq!(ok(fs::read_to_string(&log)), "move /a b/old /a b/new\n");
}

#[test]
fn a_failing_actparc_is_a_warning_with_its_reason() {
    let t = ok(tempfile::tempdir());
    let p = script(t.path(), "#!/bin/sh\necho 'disk full' >&2\nexit 3\n");
    let w = follow(p.to_str().unwrap_or(""), Path::new("/o"), Path::new("/n"))
        .unwrap_or_else(|| panic!("expected a warning"));
    assert!(w.contains("disk full") && w.contains("/o"), "{w}");
}

#[test]
fn a_missing_actparc_is_silent() {
    assert_eq!(follow("/nonexistent/actparc", Path::new("/o"), Path::new("/n")), None);
}

#[test]
fn an_actparc_that_cannot_run_is_a_warning_not_silence() {
    let t = ok(tempfile::tempdir());
    let p = t.path().join("actparc");
    ok(fs::write(&p, "#!/bin/sh\n"));
    ok(fs::set_permissions(&p, fs::Permissions::from_mode(0o644)));
    let w = follow(p.to_str().unwrap_or(""), Path::new("/o"), Path::new("/n"))
        .unwrap_or_else(|| panic!("expected a warning"));
    assert!(w.contains("cannot run"), "{w}");
}
