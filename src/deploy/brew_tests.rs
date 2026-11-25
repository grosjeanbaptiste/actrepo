use super::*;
use crate::deploy::fake::{Fake, drive, message};
use crate::deploy::target::Env;
use std::fs;

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn project(casks: bool, version: Option<&str>) -> tempfile::TempDir {
    let d = ok(tempfile::tempdir());
    if casks {
        ok(fs::create_dir_all(d.path().join("stores/brew/Casks")));
    }
    if let Some(v) = version {
        ok(fs::write(d.path().join("pubspec.yaml"), format!("name: a\nversion: {v}\n")));
    }
    d
}

fn go(root: &std::path::Path, env: Env, extra: &[(&str, &str)], tool: Fake) -> (Result<()>, Fake) {
    let pairs = [&[("BREW_APP_URL", "https://h/app.zip"), ("STORES_PYTHON", "/py"), ("STORES_SCRIPTS_DIR", "/s")][..], extra].concat();
    drive(env, root, &pairs, tool, update)
}

#[test]
fn prod_bumps_the_stable_cask_with_the_pubspec_version() {
    let d = project(true, Some("1.2.3+9"));
    let (r, t) = go(d.path(), Env::Prod, &[], Fake::default());
    assert!(r.is_ok(), "{}", message(&r));
    let brew = d.path().join("stores/brew");
    assert_eq!(
        t.lines(),
        vec![format!("/py /s/update_brew.py --version 1.2.3 --app-url https://h/app.zip --brew-root {} --channel stable", brew.display())]
    );
}

#[test]
fn test_bumps_the_beta_cask() {
    let d = project(true, Some("1.0.0"));
    let (_, t) = go(d.path(), Env::Test, &[], Fake::default());
    assert!(t.lines()[0].ends_with("--channel beta"));
}

#[test]
fn the_version_and_brew_root_can_be_overridden() {
    let d = project(false, None);
    ok(fs::create_dir_all(d.path().join("elsewhere/Formula")));
    let root = d.path().join("elsewhere");
    let (r, t) = go(d.path(), Env::Prod, &[("BREW_VERSION", "7.0.0"), ("BREW_ROOT_DIR", root.to_str().unwrap_or_default())], Fake::default());
    assert!(r.is_ok(), "{}", message(&r));
    assert!(t.lines()[0].contains(&format!("--version 7.0.0 --app-url https://h/app.zip --brew-root {}", root.display())));
}

#[test]
fn a_missing_hosted_url_is_named_before_anything() {
    let d = project(true, Some("1.0.0"));
    let (r, t) = go(d.path(), Env::Prod, &[("BREW_APP_URL", "")], Fake::default());
    assert_eq!(message(&r), "set BREW_APP_URL to the hosted macOS artifact (Homebrew references a URL, not a local file).");
    assert!(t.runs().is_empty());
}

#[test]
fn a_brew_root_without_casks_or_formulae_is_refused() {
    let d = project(false, Some("1.0.0"));
    let (r, _) = go(d.path(), Env::Prod, &[], Fake::default());
    assert_eq!(message(&r), format!("no cask/formula under {}/stores/brew (scaffold stores/brew).", d.path().display()));
}

#[test]
fn a_formula_folder_alone_is_enough() {
    let d = project(false, Some("1.0.0"));
    ok(fs::create_dir_all(d.path().join("stores/brew/Formula")));
    assert!(go(d.path(), Env::Prod, &[], Fake::default()).0.is_ok());
}

#[test]
fn an_unresolvable_version_is_named() {
    let d = project(true, None);
    let (r, _) = go(d.path(), Env::Prod, &[], Fake::default());
    assert_eq!(message(&r), "cannot resolve version (set BREW_VERSION).");
}

#[test]
fn a_failing_updater_is_an_error() {
    let d = project(true, Some("1.0.0"));
    assert!(go(d.path(), Env::Prod, &[], Fake { failing: vec!["/py".into()], ..Fake::default() }).0.is_err());
}

#[test]
fn test_with_a_dmg_publishes_a_beta_release_instead_of_bumping_a_hosted_url() {
    let d = project(true, Some("1.0.0"));
    let (r, t) = go(d.path(), Env::Test, &[("BREW_DMG", "/nope.dmg")], Fake::default());
    assert!(message(&r).starts_with("set BREW_TAP_DIR") || message(&r).starts_with("BREW_DMG not found"), "{}", message(&r));
    assert!(t.runs().is_empty());
}

#[test]
fn prod_with_a_dmg_publishes_the_stable_release_instead_of_bumping_a_hosted_url() {
    let d = project(true, Some("1.0.0"));
    let (r, t) = go(d.path(), Env::Prod, &[("BREW_DMG", "/nope.dmg")], Fake::default());
    assert!(message(&r).starts_with("set BREW_TAP_DIR") || message(&r).starts_with("BREW_DMG not found"), "{}", message(&r));
    assert!(t.runs().is_empty());
}

#[test]
fn prod_without_a_dmg_still_bumps_the_hosted_url_and_never_builds() {
    let d = project(true, Some("1.0.0"));
    let (_, t) = go(d.path(), Env::Prod, &[], Fake::default());
    assert!(t.lines()[0].ends_with("--channel stable"));
    assert_eq!(t.runs().len(), 1);
}

fn bare(root: &std::path::Path, env: Env, extra: &[(&str, &str)], tool: Fake) -> (Result<()>, Fake) {
    drive(env, root, extra, tool, update)
}

#[test]
fn a_dmg_is_built_only_for_a_beta_with_neither_a_dmg_nor_a_hosted_url() {
    let home = std::path::Path::new("/h");
    let on = |env, pairs: &[(&str, &str)]| {
        let vars = crate::deploy::vars::Vars::of(pairs, home);
        let (mut o, mut e) = (vec![], vec![]);
        let c = Ctx { env, root: std::path::Path::new("."), vars: &vars, tool: &Fake::default(), forge: &crate::deploy::fake::forge(), out: &mut o, err: &mut e };
        builds_dmg(&c)
    };
    assert!(on(Env::Test, &[]));
    assert!(!on(Env::Prod, &[]));
    assert!(!on(Env::Test, &[("BREW_DMG", "/a.dmg")]));
    assert!(!on(Env::Test, &[("BREW_APP_URL", "https://h/a.zip")]));
}

#[test]
fn the_tap_is_checked_before_any_build() {
    let d = project(true, Some("1.0.0"));
    let (r, t) = bare(d.path(), Env::Test, &[], Fake::default());
    assert_eq!(message(&r), "set BREW_TAP_DIR.");
    assert!(t.runs().is_empty());
}

#[test]
fn a_beta_without_a_dmg_builds_it_then_publishes_it() {
    let d = project(true, Some("1.2.3+9"));
    let root = d.path();
    ok(fs::create_dir_all(root.join("macos/Runner")));
    for f in ["macos/ExportOptionsDeveloperID.plist", "macos/Runner/DeveloperID.entitlements", "key.p8"] {
        ok(fs::write(root.join(f), "x"));
    }
    let tap = ok(tempfile::tempdir());
    ok(fs::create_dir_all(tap.path().join("Casks")));
    ok(fs::write(tap.path().join("Casks/acteble.rb"), "cask \"acteble\" do\n  version \"0.0.0\"\n  desc \"D\"\nend\n"));
    let (export, key, tap_s) = (root.join("build/macos/homebrew"), root.join("key.p8"), tap.path().display().to_string());
    let tool = Fake {
        on_run: vec![
            ("xcodebuild".into(), "-exportArchive".into(), export.join("export/Acteble.app")),
            ("ditto".into(), "-c".into(), export.join("export/Acteble.zip")),
            ("hdiutil".into(), "create".into(), export.join("Acteble-1.2.3-beta.42.dmg")),
        ],
        captures: vec![("shasum".into(), "-a".into(), Some("cafe  x".into()))],
        ..Fake::default()
    };
    let key_s = key.display().to_string();
    let vars = [("APPLE_TEAM_ID", "T"), ("ASC_KEY_ID", "K"), ("ASC_ISSUER_ID", "I"), ("ASC_PRIVATE_KEY_PATH", key_s.as_str()), ("BREW_BUILD", "42"), ("BREW_TAP_DIR", tap_s.as_str()), ("STORES_PYTHON", "/py"), ("STORES_SCRIPTS_DIR", "/s")];
    let (r, t) = bare(root, Env::Test, &vars, tool);
    assert!(r.is_ok(), "{}", message(&r));
    let progs: Vec<String> = t.runs().iter().map(|r| r.prog.clone()).collect();
    assert_eq!(progs, ["flutter", "xcodebuild", "xcodebuild", "ditto", "xcrun", "xcrun", "spctl", "xcrun", "ditto", "hdiutil", "/py", "git", "git"]);
    assert!(t.lines()[10].contains("--version 1.2.3-beta.42 --app-url https://github.com/acteble/homebrew-acteble/releases/download/v1.2.3-beta.42/Acteble-1.2.3-beta.42.dmg"), "{}", t.lines()[10]);
}
