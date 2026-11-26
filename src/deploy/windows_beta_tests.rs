use super::*;
use crate::deploy::ctx::Ctx;
use crate::deploy::fake::{Fake, forge, message};
use crate::deploy::target::Env;
use crate::deploy::vars::Vars;
use crate::fakes::FakeForge;
use std::fs;

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn project() -> tempfile::TempDir {
    let d = ok(tempfile::tempdir());
    ok(fs::write(d.path().join("pubspec.yaml"), "name: a\nversion: 1.2.3+9\n"));
    ok(fs::write(d.path().join("a.msix"), "msix"));
    d
}

/// `release_exists`: what `gh release view` answers (a missing release is a non-zero exit).
fn fake(release_exists: bool) -> Fake {
    Fake {
        captures: vec![
            ("git".into(), "rev-list".into(), Some("42".into())),
            ("gh".into(), "release".into(), release_exists.then(|| "v1.2.3-beta.42".into())),
        ],
        ..Fake::default()
    }
}

fn go_env(env: Env, d: &tempfile::TempDir, tool: Fake) -> (Result<()>, Fake, FakeForge) {
    let msix = d.path().join("a.msix");
    let vars = Vars::of(&[("BREW_WINDOWS_MSIX", msix.to_str().unwrap_or_default())], &d.path().join("home"));
    let (mut out, mut err) = (vec![], vec![]);
    let forge = forge();
    let result = {
        let mut ctx = Ctx { env, root: d.path(), vars: &vars, tool: &tool, forge: &forge, out: &mut out, err: &mut err };
        publish(&mut ctx)
    };
    (result, tool, forge)
}

#[test]
fn a_new_beta_is_a_prerelease_carrying_the_msix_named_after_its_version() {
    let d = project();
    let (r, _, f) = go_env(Env::Test, &d, fake(false));
    assert!(r.is_ok(), "{}", message(&r));
    let rel = f.calls.borrow().clone();
    assert_eq!(rel.len(), 1, "{rel:?}");
    assert!(rel[0].starts_with("create_release acteble/homebrew-acteble v1.2.3-beta.42 prerelease "), "{rel:?}");
    assert!(rel[0].ends_with("Acteble-1.2.3-beta.42.msix"), "{rel:?}");
}

#[test]
fn a_release_already_made_for_the_tag_gets_the_msix_uploaded() {
    let d = project();
    let (r, t, f) = go_env(Env::Test, &d, fake(true));
    assert!(r.is_ok(), "{}", message(&r));
    assert!(f.calls.borrow().is_empty(), "no second release for the same tag");
    let up = t.lines().into_iter().find(|l| l.starts_with("gh release upload")).unwrap_or_default();
    assert!(up.contains(" v1.2.3-beta.42 "), "{up}");
    assert!(up.contains("Acteble-1.2.3-beta.42.msix"), "{up}");
    assert!(up.ends_with("--repo acteble/homebrew-acteble --clobber"), "{up}");
}

#[test]
fn a_missing_msix_is_named_before_anything() {
    let d = project();
    ok(fs::remove_file(d.path().join("a.msix")));
    let (r, t, f) = go_env(Env::Test, &d, fake(false));
    assert_eq!(message(&r), format!("BREW_WINDOWS_MSIX not found: {}.", d.path().join("a.msix").display()));
    assert!(t.runs().is_empty() && f.calls.borrow().is_empty());
}

#[test]
fn prod_is_refused_because_this_publishes_the_beta_only() {
    let d = project();
    let (r, t, f) = go_env(Env::Prod, &d, fake(false));
    assert_eq!(message(&r), "the Windows GitHub release is a beta: deploy it to test.");
    assert!(t.runs().is_empty() && f.calls.borrow().is_empty());
}

#[test]
fn an_unreadable_commit_count_is_named_and_nothing_is_released() {
    let d = project();
    let mut t = fake(false);
    t.captures.retain(|(p, _, _)| p != "git");
    let (r, _, f) = go_env(Env::Test, &d, t);
    assert!(message(&r).contains("BREW_BUILD"), "{}", message(&r));
    assert!(f.calls.borrow().is_empty());
}
