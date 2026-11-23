use super::*;
use crate::deploy::ctx::Ctx;
use crate::deploy::fake::{Fake, forge, message};
use crate::deploy::vars::Vars;
use crate::fakes::FakeForge;
use crate::deploy::target::Env;
use std::fs;

const STABLE: &str = "cask \"acteble\" do\n  version \"0.0.0\"\n  desc \"D\"\nend\n";

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

struct Case {
    proj: tempfile::TempDir,
    tap: tempfile::TempDir,
}

fn case(stable: bool) -> Case {
    let (proj, tap) = (ok(tempfile::tempdir()), ok(tempfile::tempdir()));
    ok(fs::write(proj.path().join("pubspec.yaml"), "name: a\nversion: 1.2.3+9\n"));
    ok(fs::write(proj.path().join("a.dmg"), "dmg"));
    ok(fs::create_dir_all(tap.path().join("Casks")));
    if stable {
        ok(fs::write(tap.path().join("Casks/acteble.rb"), STABLE));
    }
    Case { proj, tap }
}

fn fake() -> Fake {
    Fake { captures: vec![("git".into(), "rev-list".into(), Some("42".into()))], ..Fake::default() }
}

fn go(c: &Case, extra: &[(&str, &str)], tool: Fake) -> (Result<()>, Fake) {
    let (r, t, _) = go_forge(c, extra, tool, forge());
    (r, t)
}

fn go_forge(c: &Case, extra: &[(&str, &str)], tool: Fake, forge: FakeForge) -> (Result<()>, Fake, FakeForge) {
    let (dmg, tap) = (c.proj.path().join("a.dmg"), c.tap.path().to_path_buf());
    let base = [
        ("BREW_DMG", dmg.to_str().unwrap_or_default()),
        ("BREW_TAP_DIR", tap.to_str().unwrap_or_default()),
        ("STORES_PYTHON", "/py"),
        ("STORES_SCRIPTS_DIR", "/s"),
    ];
    let home = c.proj.path().join("home");
    let vars = Vars::of(&[&base[..], extra].concat(), &home);
    let (mut out, mut err) = (vec![], vec![]);
    let result = {
        let mut ctx = Ctx { env: Env::Test, root: c.proj.path(), vars: &vars, tool: &tool, forge: &forge, out: &mut out, err: &mut err };
        publish(&mut ctx)
    };
    (result, tool, forge)
}

#[test]
fn a_beta_is_a_prerelease_then_a_bump_then_a_commit() {
    let c = case(true);
    let (r, t, f) = go_forge(&c, &[], fake(), forge());
    assert!(r.is_ok(), "{}", message(&r));
    let rel = f.calls.borrow().clone();
    assert_eq!(rel.len(), 1, "{rel:?}");
    assert!(rel[0].starts_with("create_release acteble/homebrew-acteble v1.2.3-beta.42 prerelease "));
    assert!(rel[0].ends_with("Acteble-1.2.3-beta.42.dmg"));
    let l = t.lines();
    assert_eq!(l.len(), 3, "{l:?}");
    assert!(l[0].starts_with("/py /s/update_brew.py --version 1.2.3-beta.42 --app-url https://github.com/acteble/homebrew-acteble/releases/download/v1.2.3-beta.42/Acteble-1.2.3-beta.42.dmg"));
    assert!(l[0].ends_with("--channel beta"));
    assert_eq!(l[1], "git add Casks");
    assert_eq!(l[2], "git commit -m chore: Acteble beta 1.2.3-beta.42");
}

#[test]
fn the_git_calls_run_in_the_tap() {
    let c = case(true);
    let (_, t) = go(&c, &[], fake());
    let runs = t.runs();
    assert_eq!(runs[1].cwd.as_deref(), Some(c.tap.path()));
    assert_eq!(runs[2].cwd.as_deref(), Some(c.tap.path()));
}

#[test]
fn the_build_number_counts_the_commits_of_the_project() {
    let c = case(true);
    let (_, t) = go(&c, &[], fake());
    assert_eq!(t.captured.borrow()[0].cwd.as_deref(), Some(c.proj.path()));
}

#[test]
fn the_beta_cask_is_made_from_the_stable_one_when_absent() {
    let c = case(true);
    assert!(go(&c, &[], fake()).0.is_ok());
    let beta = ok(fs::read_to_string(c.tap.path().join("Casks/acteble@beta.rb")));
    assert!(beta.starts_with("cask \"acteble@beta\" do\n  conflicts_with cask: \"acteble\"\n"));
}

#[test]
fn an_existing_beta_cask_is_left_as_it_is() {
    let c = case(true);
    ok(fs::write(c.tap.path().join("Casks/acteble@beta.rb"), "mine"));
    assert!(go(&c, &[], fake()).0.is_ok());
    assert_eq!(ok(fs::read_to_string(c.tap.path().join("Casks/acteble@beta.rb"))), "mine");
}

#[test]
fn without_a_stable_cask_the_beta_cask_cannot_be_made() {
    let c = case(false);
    let (r, t) = go(&c, &[], fake());
    assert_eq!(message(&r), format!("no stable cask {} to make the beta cask from.", c.tap.path().join("Casks/acteble.rb").display()));
    assert!(t.runs().is_empty());
}

#[test]
fn a_missing_dmg_is_named_before_anything() {
    let c = case(true);
    ok(fs::remove_file(c.proj.path().join("a.dmg")));
    let (r, t) = go(&c, &[], fake());
    assert_eq!(message(&r), format!("BREW_DMG not found: {} (build it with build_dmg.sh).", c.proj.path().join("a.dmg").display()));
    assert!(t.runs().is_empty());
}

#[test]
fn an_unreadable_commit_count_is_named() {
    let c = case(true);
    let (r, t) = go(&c, &[], Fake::default());
    assert_eq!(message(&r), "cannot count the commits (set BREW_BUILD).");
    assert!(t.runs().is_empty());
}

#[test]
fn the_build_number_can_be_given() {
    let c = case(true);
    let (_, _, f) = go_forge(&c, &[("BREW_BUILD", "7")], Fake::default(), forge());
    assert!(f.calls.borrow()[0].starts_with("create_release acteble/homebrew-acteble v1.2.3-beta.7 "));
}

#[test]
fn a_build_number_that_is_not_a_number_is_refused() {
    let c = case(true);
    let (r, _) = go(&c, &[("BREW_BUILD", "x")], Fake::default());
    assert_eq!(message(&r), "BREW_BUILD is not a number: x.");
}

#[test]
fn a_failing_release_stops_before_the_bump_and_the_commit() {
    let c = case(true);
    let (r, t, _) = go_forge(&c, &[], fake(), forge().failing("create_release"));
    assert_eq!(message(&r), "creating the GitHub prerelease");
    assert!(t.runs().is_empty());
}

#[test]
fn a_failing_bump_stops_before_the_commit() {
    let c = case(true);
    let (r, t) = go(&c, &[], Fake { failing: vec!["/py".into()], ..fake() });
    assert!(r.is_err());
    assert!(t.lines().iter().all(|l| !l.starts_with("git")));
}

#[test]
fn nothing_is_pushed_unless_asked() {
    let c = case(true);
    let (_, t) = go(&c, &[], fake());
    assert!(t.lines().iter().all(|l| !l.starts_with("git push")));
    let (_, t) = go(&c, &[("BREW_PUSH", "1")], fake());
    assert_eq!(t.lines().last().map(String::as_str), Some("git push"));
}

#[test]
fn the_repository_and_the_app_can_be_overridden() {
    let c = case(true);
    ok(fs::write(c.tap.path().join("Casks/x.rb"), STABLE.replace("acteble", "x")));
    let (_, _, f) = go_forge(&c, &[("BREW_REPO", "o/r"), ("BREW_APP", "X")], fake(), forge());
    let rel = f.calls.borrow()[0].clone();
    assert!(rel.starts_with("create_release o/r v1.2.3-beta.42 "));
    assert!(rel.ends_with("X-1.2.3-beta.42.dmg"));
}
