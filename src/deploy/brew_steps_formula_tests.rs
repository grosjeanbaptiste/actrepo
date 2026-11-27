use super::*;
use std::fs;

const STABLE: &str = "class ActebleLinux < Formula\n  desc \"Desktop client\"\n  url \"https://x/v1/a.tar.gz\"\n  version \"1.0.0\"\n  depends_on :linux\nend\n";

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn tap() -> tempfile::TempDir {
    let d = ok(tempfile::tempdir());
    ok(fs::create_dir_all(d.path().join("Formula")));
    d
}

#[test]
fn the_beta_formula_is_made_from_the_stable_one_when_it_does_not_exist() {
    let d = tap();
    ok(fs::write(d.path().join("Formula/acteble-linux.rb"), STABLE));
    ok(ensure_beta_formula(&d.path().join("Formula"), "acteble-linux"));
    let beta = ok(fs::read_to_string(d.path().join("Formula/acteble-linux@beta.rb")));
    assert!(beta.starts_with("class ActebleLinuxATBeta < Formula"), "{beta}");
}

#[test]
fn an_existing_beta_formula_is_left_alone() {
    let d = tap();
    ok(fs::write(d.path().join("Formula/acteble-linux.rb"), STABLE));
    ok(fs::write(d.path().join("Formula/acteble-linux@beta.rb"), "mine"));
    ok(ensure_beta_formula(&d.path().join("Formula"), "acteble-linux"));
    assert_eq!(ok(fs::read_to_string(d.path().join("Formula/acteble-linux@beta.rb"))), "mine");
}

#[test]
fn without_a_stable_formula_it_fails_and_names_it() {
    let d = tap();
    let e = match ensure_beta_formula(&d.path().join("Formula"), "acteble-linux") {
        Ok(_) => panic!("expected an error"),
        Err(e) => e,
    };
    assert!(e.to_string().contains("acteble-linux.rb"), "{e}");
}

#[test]
fn nothing_is_written_when_the_stable_formula_is_missing() {
    let d = tap();
    let _ = ensure_beta_formula(&d.path().join("Formula"), "acteble-linux");
    assert!(!d.path().join("Formula/acteble-linux@beta.rb").exists());
}
