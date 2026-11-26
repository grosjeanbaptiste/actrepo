use super::*;
use crate::deploy::ctx::Ctx;
use crate::deploy::fake::{Fake, forge, message};
use crate::deploy::target::Env;
use crate::deploy::vars::Vars;
use crate::fakes::FakeForge;
use std::fs;

const STABLE: &str = "class ActebleLinux < Formula\n  desc \"D\"\n  url \"https://h/download/v0.0.0/acteble-linux-x86_64.tar.gz\"\n  version \"0.0.0\"\n  sha256 \"old\" # release-bumped\nend\n";

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
    ok(fs::write(proj.path().join("a.tar.gz"), "tar"));
    ok(fs::create_dir_all(tap.path().join("Formula")));
    if stable {
        ok(fs::write(tap.path().join("Formula/acteble-linux.rb"), STABLE));
    }
    Case { proj, tap }
}

fn fake() -> Fake {
    Fake {
        captures: vec![
            ("git".into(), "rev-list".into(), Some("42".into())),
            ("shasum".into(), "-a".into(), Some("ab12  a.tar.gz".into())),
        ],
        ..Fake::default()
    }
}

fn go_env(env: Env, c: &Case, extra: &[(&str, &str)], tool: Fake) -> (Result<()>, Fake, FakeForge) {
    run(env, c, extra, tool, forge())
}

fn run(env: Env, c: &Case, extra: &[(&str, &str)], tool: Fake, forge: FakeForge) -> (Result<()>, Fake, FakeForge) {
    let (tar, tap) = (c.proj.path().join("a.tar.gz"), c.tap.path().to_path_buf());
    let base = [
        ("BREW_LINUX_TARBALL", tar.to_str().unwrap_or_default()),
        ("BREW_TAP_DIR", tap.to_str().unwrap_or_default()),
    ];
    let vars = Vars::of(&[&base[..], extra].concat(), &c.proj.path().join("home"));
    let (mut out, mut err) = (vec![], vec![]);
    let result = {
        let mut ctx = Ctx { env, root: c.proj.path(), vars: &vars, tool: &tool, forge: &forge, out: &mut out, err: &mut err };
        publish(&mut ctx)
    };
    (result, tool, forge)
}

fn go(c: &Case, tool: Fake) -> (Result<()>, Fake, FakeForge) {
    go_env(Env::Test, c, &[], tool)
}

#[test]
fn a_new_beta_is_a_prerelease_carrying_the_tarball() {
    let c = case(true);
    let (r, _, f) = go(&c, fake());
    assert!(r.is_ok(), "{}", message(&r));
    let rel = f.calls.borrow().clone();
    assert_eq!(rel.len(), 2, "{rel:?}");
    assert_eq!(rel[0], "release_exists acteble/homebrew-acteble v1.2.3-beta.42");
    assert!(rel[1].starts_with("create_release acteble/homebrew-acteble v1.2.3-beta.42 prerelease "), "{rel:?}");
    assert!(rel[1].ends_with("acteble-linux-x86_64.tar.gz"), "{rel:?}");
}

#[test]
fn a_release_already_made_for_the_tag_gets_the_tarball_uploaded() {
    let c = case(true);
    let (r, _, f) = run(Env::Test, &c, &[], fake(), forge().with_release());
    assert!(r.is_ok(), "{}", message(&r));
    let calls = f.calls.borrow().clone();
    assert!(calls.iter().all(|l| !l.starts_with("create_release")), "no second release for the same tag: {calls:?}");
    let up = calls.iter().find(|l| l.starts_with("upload_release_asset")).cloned().unwrap_or_default();
    assert!(up.starts_with("upload_release_asset acteble/homebrew-acteble v1.2.3-beta.42 "), "{calls:?}");
    assert!(up.ends_with("acteble-linux-x86_64.tar.gz"), "{calls:?}");
}

#[test]
fn the_beta_formula_is_bumped_to_the_beta_tag_with_the_checksum() {
    let c = case(true);
    assert!(go(&c, fake()).0.is_ok());
    let f = ok(fs::read_to_string(c.tap.path().join("Formula/acteble-linux@beta.rb")));
    assert!(f.contains("url \"https://github.com/acteble/homebrew-acteble/releases/download/v1.2.3-beta.42/acteble-linux-x86_64.tar.gz\""), "{f}");
    assert!(f.contains("version \"1.2.3-beta.42\""), "{f}");
    assert!(f.contains("sha256 \"ab12\" # release-bumped"), "{f}");
}

#[test]
fn the_stable_formula_is_left_as_it_was() {
    let c = case(true);
    assert!(go(&c, fake()).0.is_ok());
    assert_eq!(ok(fs::read_to_string(c.tap.path().join("Formula/acteble-linux.rb"))), STABLE);
}

#[test]
fn the_tap_gets_one_commit_of_the_beta_formula_run_in_the_tap() {
    let c = case(true);
    let (_, t, _) = go(&c, fake());
    let l: Vec<String> = t.lines().into_iter().filter(|l| l.starts_with("git ")).collect();
    assert_eq!(l, ["git add Formula", "git commit -m chore: Acteble linux beta 1.2.3-beta.42"]);
    assert!(t.runs().iter().filter(|r| r.prog == "git").all(|r| r.cwd.as_deref() == Some(c.tap.path())));
}

#[test]
fn nothing_is_pushed_unless_asked() {
    let c = case(true);
    assert!(!go(&c, fake()).1.lines().iter().any(|l| l == "git push"));
    let (_, t, _) = go_env(Env::Test, &c, &[("BREW_PUSH", "1")], fake());
    assert!(t.lines().iter().any(|l| l == "git push"));
}

#[test]
fn a_missing_tarball_is_named_before_anything() {
    let c = case(true);
    ok(fs::remove_file(c.proj.path().join("a.tar.gz")));
    let (r, t, f) = go(&c, fake());
    assert_eq!(message(&r), format!("BREW_LINUX_TARBALL not found: {}.", c.proj.path().join("a.tar.gz").display()));
    assert!(t.runs().is_empty() && f.calls.borrow().is_empty());
}

#[test]
fn without_a_stable_formula_nothing_is_released() {
    let c = case(false);
    let (r, t, f) = go(&c, fake());
    assert_eq!(message(&r), format!("no stable formula {} to make the beta formula from.", c.tap.path().join("Formula/acteble-linux.rb").display()));
    assert!(t.runs().is_empty() && f.calls.borrow().is_empty());
}

#[test]
fn a_tarball_that_cannot_be_hashed_is_not_released() {
    let c = case(true);
    let mut t = fake();
    t.captures.retain(|(p, _, _)| p != "shasum");
    let (r, _, f) = go(&c, t);
    assert!(message(&r).contains("could not hash"), "{}", message(&r));
    assert!(f.calls.borrow().is_empty());
}

#[test]
fn prod_is_refused_because_this_publishes_the_beta_only() {
    let c = case(true);
    let (r, t, f) = go_env(Env::Prod, &c, &[], fake());
    assert_eq!(message(&r), "the Linux Homebrew release is a beta: deploy it to test (stable: scripts/release_homebrew_linux.sh).");
    assert!(t.runs().is_empty() && f.calls.borrow().is_empty());
}
