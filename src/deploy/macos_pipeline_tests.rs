use super::*;
use std::path::Path;
use crate::deploy::fake::{Fake, drive};
use crate::deploy::target::Env;
use std::fs;

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn project() -> tempfile::TempDir {
    let d = ok(tempfile::tempdir());
    ok(fs::create_dir_all(d.path().join("macos/Runner")));
    for f in ["macos/ExportOptionsDeveloperID.plist", "macos/Runner/DeveloperID.entitlements", "key.p8"] {
        ok(fs::write(d.path().join(f), "x"));
    }
    ok(fs::write(d.path().join("pubspec.yaml"), "name: a\nversion: 1.2.3+9\n"));
    d
}

fn vars(d: &Path) -> Vec<(&'static str, String)> {
    vec![
        ("APPLE_TEAM_ID", "T".into()),
        ("ASC_KEY_ID", "K".into()),
        ("ASC_ISSUER_ID", "I".into()),
        ("ASC_PRIVATE_KEY_PATH", d.join("key.p8").display().to_string()),
        ("BREW_BUILD", "42".into()),
    ]
}

fn building(d: &Path) -> Fake {
    let export = d.join("build/macos/homebrew/export");
    Fake {
        on_run: vec![
            ("xcodebuild".into(), "-exportArchive".into(), export.join("Acteble.app")),
            ("ditto".into(), "-c".into(), export.join("Acteble.zip")),
            ("hdiutil".into(), "create".into(), d.join("build/macos/homebrew/Acteble-1.2.3-beta.42.dmg")),
        ],
        captures: vec![("shasum".into(), "-a".into(), Some("cafe  x".into()))],
        ..Fake::default()
    }
}

fn go(d: &Path, tool: Fake) -> (Result<PathBuf>, Fake) {
    let owned = vars(d);
    let pairs: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let mut made = None;
    let (r, tool) = drive(Env::Test, d, &pairs, tool, |c| {
        made = Some(dmg(c)?);
        Ok(())
    });
    (r.map(|()| made.unwrap_or_default()), tool)
}

fn why(r: &Result<PathBuf>) -> String {
    r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default()
}

#[test]
fn the_three_slices_run_in_order_and_the_beta_named_dmg_comes_back() {
    let d = project();
    let (r, t) = go(d.path(), building(d.path()));
    assert_eq!(ok(r), d.path().join("build/macos/homebrew/Acteble-1.2.3-beta.42.dmg"));
    let progs: Vec<String> = t.runs().iter().map(|r| r.prog.clone()).collect();
    assert_eq!(progs, ["flutter", "xcodebuild", "xcodebuild", "ditto", "xcrun", "xcrun", "spctl", "xcrun", "ditto", "hdiutil"]);
    assert_eq!(t.lines()[0], "flutter build macos --release --build-name 1.2.3 --build-number 42");
}

#[test]
fn a_failing_export_stops_before_notarizing_and_a_failing_notarization_before_the_dmg() {
    for (prog, runs) in [("flutter", 1), ("spctl", 7)] {
        let d = project();
        let tool = Fake { failing: vec![prog.into()], ..building(d.path()) };
        let (r, t) = go(d.path(), tool);
        assert!(r.is_err(), "{prog}");
        assert_eq!(t.runs().len(), runs, "{prog}");
        assert!(!t.lines().iter().any(|l| l.starts_with("hdiutil")), "{prog}");
    }
}

#[test]
fn a_missing_tool_is_named_and_nothing_runs() {
    for tool in ["xcodebuild", "xcrun", "hdiutil", "ditto", "flutter"] {
        let d = project();
        let (r, t) = go(d.path(), Fake { missing: vec![tool.into()], ..building(d.path()) });
        assert!(why(&r).contains(tool), "{tool}: {}", why(&r));
        assert!(t.runs().is_empty(), "{tool}");
    }
}

#[test]
fn an_unreadable_version_is_named_before_anything_runs() {
    let d = project();
    ok(fs::remove_file(d.path().join("pubspec.yaml")));
    let (r, t) = go(d.path(), building(d.path()));
    assert!(why(&r).contains("BREW_VERSION"), "{}", why(&r));
    assert!(t.runs().is_empty());
}
