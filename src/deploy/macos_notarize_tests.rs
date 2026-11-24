use super::*;
use std::path::PathBuf;
use crate::deploy::target::Env;
use crate::deploy::fake::{Fake, drive, message};
use std::fs;

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

struct Case {
    dir: tempfile::TempDir,
}

impl Case {
    fn app(&self) -> PathBuf {
        self.dir.path().join("Acteble.app")
    }
    fn zip(&self) -> PathBuf {
        self.dir.path().join("Acteble.zip")
    }
    fn key(&self) -> String {
        self.dir.path().join("key.p8").display().to_string()
    }
}

fn case() -> Case {
    let dir = ok(tempfile::tempdir());
    ok(fs::create_dir_all(dir.path().join("Acteble.app")));
    ok(fs::write(dir.path().join("key.p8"), "k"));
    Case { dir }
}

fn zipping(c: &Case) -> Fake {
    Fake { on_run: vec![("ditto".into(), "-c".into(), c.zip())], ..Fake::default() }
}

fn go(c: &Case, extra: &[(&str, &str)], tool: Fake) -> (Result<()>, Fake) {
    let key = c.key();
    let base = [("ASC_KEY_ID", "KID"), ("ASC_ISSUER_ID", "ISS"), ("ASC_PRIVATE_KEY_PATH", key.as_str())];
    let vars = [&base[..], extra].concat();
    let app = c.app();
    drive(Env::Prod, c.dir.path(), &vars, tool, |ctx| notarize(ctx, &app))
}

#[test]
fn the_app_is_zipped_submitted_stapled_and_checked_then_the_zip_goes() {
    let c = case();
    let (r, t) = go(&c, &[], zipping(&c));
    assert!(r.is_ok(), "{}", message(&r));
    let (app, zip, key) = (c.app().display().to_string(), c.zip().display().to_string(), c.key());
    assert_eq!(
        t.lines(),
        [
            format!("ditto -c -k --keepParent {app} {zip}"),
            format!("xcrun notarytool submit {zip} --key {key} --key-id KID --issuer ISS --wait"),
            format!("xcrun stapler staple {app}"),
            format!("spctl --assess --type execute -vv {app}"),
            format!("xcrun stapler validate {app}"),
        ]
    );
    assert!(!c.zip().exists());
}

#[test]
fn credentials_are_required_and_named() {
    for (vars, name) in [(vec![("ASC_ISSUER_ID", "I")], "ASC_KEY_ID"), (vec![("ASC_KEY_ID", "K")], "ASC_ISSUER_ID")] {
        let c = case();
        let app = c.app();
        let (r, t) = drive(Env::Prod, c.dir.path(), &vars, Fake::default(), |ctx| notarize(ctx, &app));
        assert_eq!(message(&r), format!("set {name}."));
        assert!(t.runs().is_empty());
    }
}

#[test]
fn the_key_defaults_under_the_home_and_must_exist() {
    let c = case();
    let app = c.app();
    let vars = [("ASC_KEY_ID", "K"), ("ASC_ISSUER_ID", "I")];
    let (r, t) = drive(Env::Prod, c.dir.path(), &vars, Fake::default(), |ctx| notarize(ctx, &app));
    assert!(message(&r).contains("deploy-fake-home-unused/.config/asc/AuthKey.p8"), "{}", message(&r));
    assert!(t.runs().is_empty());
}

#[test]
fn a_missing_app_is_named_and_nothing_runs() {
    let c = case();
    ok(fs::remove_dir_all(c.app()));
    let (r, t) = go(&c, &[], Fake::default());
    assert!(message(&r).contains("no .app at") && message(&r).contains("Acteble.app"), "{}", message(&r));
    assert!(t.runs().is_empty());
}

#[test]
fn a_failing_step_stops_the_chain_names_itself_and_removes_the_zip() {
    for (prog, runs, step) in [("ditto", 1, "ditto"), ("xcrun", 2, "notarytool submit"), ("spctl", 4, "spctl")] {
        let c = case();
        let tool = Fake { failing: vec![prog.into()], ..zipping(&c) };
        let (r, t) = go(&c, &[], tool);
        let why = r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default();
        assert!(why.contains(step), "{prog}: {why}");
        assert_eq!(t.runs().len(), runs, "{prog}");
        assert!(!c.zip().exists(), "{prog}");
    }
}

/// Runs `notarize` with a hand-built context so the test can read stderr (`drive` drops it).
fn go_err(c: &Case, tool: Fake) -> (Result<()>, String) {
    let key = c.key();
    let pairs = [("ASC_KEY_ID", "KID"), ("ASC_ISSUER_ID", "ISS"), ("ASC_PRIVATE_KEY_PATH", key.as_str())];
    let home = std::env::temp_dir().join("deploy-fake-home-unused");
    let vars = crate::deploy::vars::Vars::of(&pairs, &home);
    let (mut out, mut err) = (vec![], vec![]);
    let app = c.app();
    let result = {
        let mut ctx = crate::deploy::ctx::Ctx {
            env: Env::Prod,
            root: c.dir.path(),
            vars: &vars,
            tool: &tool,
            forge: &crate::deploy::fake::forge(),
            out: &mut out,
            err: &mut err,
        };
        notarize(&mut ctx, &app)
    };
    (result, String::from_utf8_lossy(&err).into_owned())
}

#[test]
fn a_zip_that_cannot_be_removed_is_warned_about_by_name() {
    let c = case();
    // ditto makes a non-empty folder where the zip should be: remove_file fails with a non-NotFound error.
    let tool = Fake { on_run: vec![("ditto".into(), "-c".into(), c.zip().join("inner"))], ..Fake::default() };
    let (r, err) = go_err(&c, tool);
    assert!(r.is_ok(), "{}", message(&r));
    assert!(err.contains("warning: could not remove"), "{err}");
    assert!(err.contains(&c.zip().display().to_string()), "{err}");
}

#[test]
fn a_zip_that_is_already_gone_gives_no_warning() {
    let c = case();
    let (r, err) = go_err(&c, Fake::default()); // ditto makes no zip at all
    assert!(r.is_ok(), "{}", message(&r));
    assert!(err.is_empty(), "{err}");
}
