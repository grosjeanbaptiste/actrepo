//! `actrepo pr view|list|retarget|close` end to end, against a scripted `gh`.
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

const GH: &str = r#"#!/bin/sh
echo "$*" >> "$GH_LOG"
case "$*" in
  "pr list"*) printf '3\037Add x\037feat\037main\037bob\037OPEN\037MERGEABLE\n' ;;
  "pr view"*"--json state"*) echo MERGED ;;
  "pr view"*) printf '8\037Add x\037feature\037main\037ana\037MERGEABLE\037APPROVED\037CLEAN\n' ;;
  "pr close 99"*) echo "no such pr" >&2; exit 1 ;;
esac
"#;

struct Fx {
    _d: tempfile::TempDir,
    bin: std::path::PathBuf,
    log: std::path::PathBuf,
}

fn fx() -> Fx {
    let d = ok(tempfile::tempdir());
    let base = ok(d.path().canonicalize());
    let bin = base.join("bin");
    ok(fs::create_dir_all(&bin));
    ok(fs::write(bin.join("gh"), GH));
    ok(fs::set_permissions(
        bin.join("gh"),
        fs::Permissions::from_mode(0o755),
    ));
    let log = base.join("gh.log");
    Fx { _d: d, bin, log }
}

fn run(f: &Fx, args: &[&str]) -> Output {
    let path = format!(
        "{}:{}",
        f.bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    ok(Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .args(["pr"])
        .args(args)
        .env("PATH", path)
        .env("GH_LOG", &f.log)
        .output())
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn view_shows_the_pr() {
    let f = fx();
    let o = run(&f, &["view", "8"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        out(&o),
        "#8 Add x\nfeature -> main by ana, MERGED, MERGEABLE\n"
    );
}

#[test]
fn list_defaults_to_open_and_takes_a_state() {
    let f = fx();
    let o = run(&f, &["list"]);
    assert_eq!(out(&o), "#3 OPEN feat -> main bob MERGEABLE Add x\n");
    assert!(run(&f, &["list", "--state", "merged"]).status.success());
    let log = ok(fs::read_to_string(&f.log));
    assert!(
        log.contains("pr list --state open ") && log.contains("pr list --state merged "),
        "{log}"
    );
}

#[test]
fn retarget_and_close_edit_the_pr() {
    let f = fx();
    assert_eq!(
        out(&run(&f, &["retarget", "8", "main"])),
        "#8 now targets main\n"
    );
    assert_eq!(
        out(&run(&f, &["close", "8", "--comment", "in #11"])),
        "#8 closed\n"
    );
    let log = ok(fs::read_to_string(&f.log));
    assert!(log.contains("pr edit 8 --base main"), "{log}");
    assert!(log.contains("pr close 8 --comment in #11"), "{log}");
}

#[test]
fn a_gh_failure_exits_non_zero_and_names_it() {
    let f = fx();
    let o = run(&f, &["close", "99"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("no such pr"));
    assert!(out(&o).is_empty());
}
