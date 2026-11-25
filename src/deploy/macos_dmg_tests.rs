use super::*;
use crate::deploy::fake::{Fake, drive, message};
use crate::deploy::target::Env;
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
    fn out(&self) -> PathBuf {
        self.dir.path().join("out")
    }
    fn dmg(&self) -> PathBuf {
        self.out().join("Acteble-1.2.3-beta.7.dmg")
    }
}

fn case() -> Case {
    let dir = ok(tempfile::tempdir());
    ok(fs::create_dir_all(dir.path().join("Acteble.app")));
    Case { dir }
}

fn hashing() -> Vec<(String, String, Option<String>)> {
    vec![("shasum".into(), "-a".into(), Some("deadbeef  x.dmg".into()))]
}

fn creating(c: &Case) -> Fake {
    Fake { captures: hashing(), on_run: vec![("hdiutil".into(), "create".into(), c.dmg())], ..Fake::default() }
}

fn go(c: &Case, vars: &[(&str, &str)], tool: Fake) -> (Result<(PathBuf, String)>, Fake) {
    let (app, out) = (c.app(), c.out());
    let mut made = None;
    let (r, tool) = drive(Env::Test, c.dir.path(), vars, tool, |ctx| {
        made = Some(build(ctx, &app, "1.2.3-beta.7", &out)?);
        Ok(())
    });
    (r.map(|()| made.unwrap_or_default()), tool)
}

fn why<T>(r: &Result<T>) -> String {
    r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default()
}

#[test]
fn the_dmg_holds_the_app_and_its_hash_comes_back() {
    let c = case();
    let (r, t) = go(&c, &[], creating(&c));
    assert_eq!(ok(r), (c.dmg(), "deadbeef".to_string()));
    let lines = t.lines();
    let ditto: Vec<&str> = lines[0].split(' ').collect();
    assert_eq!((ditto[0], ditto[1]), ("ditto", c.app().to_str().unwrap_or_default()));
    let stage = ditto[2].trim_end_matches("/Acteble.app");
    assert_eq!(lines[1], format!("hdiutil create -volname Acteble -srcfolder {stage} -ov -format UDZO {}", c.dmg().display()));
    assert_eq!(lines.len(), 2);
    assert!(!Path::new(stage).exists(), "the stage folder goes");
    assert_eq!(t.captured.borrow()[0].args, ["-a", "256", &c.dmg().display().to_string()]);
}

#[test]
fn the_volume_and_file_take_the_app_name() {
    let c = case();
    let (_, t) = go(&c, &[("BREW_APP", "Other")], Fake::default());
    assert!(t.lines()[1].starts_with("hdiutil create -volname Other "), "{:?}", t.lines());
    assert!(t.lines()[1].ends_with("out/Other-1.2.3-beta.7.dmg"), "{:?}", t.lines());
}

#[test]
fn the_stage_has_the_app_copy_and_a_link_to_applications() {
    let c = case();
    let stage = c.dir.path().join("stage");
    ok(fs::create_dir_all(&stage));
    let (app, tool) = (c.app(), Fake::default());
    let (r, _) = drive(Env::Test, c.dir.path(), &[], tool, |ctx| stage_app(ctx, &app, &stage));
    assert!(r.is_ok(), "{}", message(&r));
    assert_eq!(ok(fs::read_link(stage.join("Applications"))), PathBuf::from("/Applications"));
}

#[test]
fn a_stale_dmg_goes_and_a_run_that_makes_none_fails() {
    let c = case();
    ok(fs::create_dir_all(c.out()));
    ok(fs::write(c.dmg(), "old"));
    let (r, _) = go(&c, &[], Fake { captures: hashing(), ..Fake::default() });
    assert!(why(&r).contains("produced no dmg"), "{}", why(&r));
    assert!(!c.dmg().exists());
}

#[test]
fn a_hash_that_cannot_be_read_fails() {
    let c = case();
    let tool = Fake { on_run: vec![("hdiutil".into(), "create".into(), c.dmg())], ..Fake::default() };
    let (r, _) = go(&c, &[], tool);
    assert!(why(&r).contains("could not hash"), "{}", why(&r));
}

#[test]
fn a_failing_step_stops_names_itself_and_leaves_no_stage() {
    for (prog, runs, step) in [("ditto", 1, "ditto"), ("hdiutil", 2, "hdiutil create")] {
        let c = case();
        let tool = Fake { failing: vec![prog.into()], ..creating(&c) };
        let (r, t) = go(&c, &[], tool);
        assert!(why(&r).contains(step), "{prog}: {}", why(&r));
        assert_eq!(t.runs().len(), runs, "{prog}");
        let stage = t.runs()[0].args.last().map(|a| a.trim_end_matches("/Acteble.app").to_string()).unwrap_or_default();
        assert!(!Path::new(&stage).exists(), "{prog}");
    }
}

#[test]
fn a_missing_app_is_named() {
    let c = case();
    ok(fs::remove_dir_all(c.app()));
    let (r, t) = go(&c, &[], Fake::default());
    assert!(why(&r).contains("no .app at"), "{}", why(&r));
    assert!(t.runs().is_empty());
}

#[test]
fn a_stale_dmg_that_cannot_be_removed_fails_loudly_before_anything_runs() {
    use std::os::unix::fs::PermissionsExt;
    let c = case();
    ok(fs::create_dir_all(c.out()));
    ok(fs::write(c.dmg(), "old"));
    ok(fs::set_permissions(c.out(), fs::Permissions::from_mode(0o555)));
    let (r, t) = go(&c, &[], creating(&c));
    ok(fs::set_permissions(c.out(), fs::Permissions::from_mode(0o755))); // lets the temp folder clean up
    assert!(why(&r).contains("removing"), "{}", why(&r));
    assert!(t.runs().is_empty());
}
