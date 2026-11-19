//! `repo new --need` through the real binary. No network: clap refuses, or actit's indexer
//! is a closed local port, so nothing is created and no name is printed.
use std::process::{Command, Output};

fn repo(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_repo"))
        .args(args)
        .env("ACTIT_INDEXER", "http://127.0.0.1:1")
        .output()
        .unwrap_or_else(|e| panic!("cannot run repo: {e}"))
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

#[test]
fn force_without_need_is_refused_by_clap() {
    let out = repo(&["new", "package/rust", "--name", "zzqx", "--force"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains("--need"),
        "{}",
        text(&out.stderr)
    );
}

#[test]
fn a_need_whose_lookup_fails_stops_before_naming_and_names_the_indexer() {
    let out = repo(&[
        "new",
        "package/rust",
        "--name",
        "zzqx",
        "--dry-run",
        "--need",
        "pdf text extraction",
    ]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    assert!(
        text(&out.stderr).contains("127.0.0.1:1"),
        "{}",
        text(&out.stderr)
    );
    assert!(
        text(&out.stdout).is_empty(),
        "no name must be printed: {}",
        text(&out.stdout)
    );
}

#[test]
fn force_does_not_hide_a_failing_lookup() {
    let out = repo(&[
        "new",
        "package/rust",
        "--name",
        "zzqx",
        "--dry-run",
        "--need",
        "pdf text",
        "--force",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("127.0.0.1:1"));
    assert!(text(&out.stdout).is_empty());
}

#[test]
fn a_need_with_no_searchable_term_is_an_error_not_a_pass() {
    let out = repo(&[
        "new",
        "package/rust",
        "--name",
        "zzqx",
        "--dry-run",
        "--need",
        "   ",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stdout).is_empty());
}
