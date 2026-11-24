use super::*;
use crate::deploy::fake::{Fake, drive};
use crate::deploy::target::Env;
use std::fs;
use std::path::Path;

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

const VARS: [(&str, &str); 1] = [("APPLE_TEAM_ID", "TEAM123")];

fn project(plist: bool, entitlements: bool) -> tempfile::TempDir {
    let d = ok(tempfile::tempdir());
    ok(fs::create_dir_all(d.path().join("macos/Runner")));
    if plist {
        ok(fs::write(d.path().join("macos/ExportOptionsDeveloperID.plist"), "p"));
    }
    if entitlements {
        ok(fs::write(d.path().join("macos/Runner/DeveloperID.entitlements"), "e"));
    }
    d
}

fn exporting(root: &Path) -> Fake {
    let app = root.join("build/macos/homebrew/export/Acteble.app");
    Fake { on_run: vec![("xcodebuild".into(), "-exportArchive".into(), app)], ..Fake::default() }
}

fn go(root: &Path, vars: &[(&str, &str)], tool: Fake) -> (Result<PathBuf>, Fake) {
    let mut found = None;
    let (r, tool) = drive(Env::Prod, root, vars, tool, |c| {
        found = Some(export_app(c, "1.2.3", 42)?);
        Ok(())
    });
    (r.map(|()| found.unwrap_or_default()), tool)
}

fn why(r: &Result<PathBuf>) -> String {
    r.as_ref().err().map(|e| format!("{e:#}")).unwrap_or_default()
}

#[test]
fn the_app_is_built_archived_with_developer_id_then_exported() {
    let d = project(true, true);
    let (r, t) = go(d.path(), &VARS, exporting(d.path()));
    assert_eq!(ok(r), d.path().join("build/macos/homebrew/export/Acteble.app"));
    let root = d.path().display();
    let lines = t.lines();
    assert_eq!(lines[0], "flutter build macos --release --build-name 1.2.3 --build-number 42");
    assert_eq!(
        lines[1],
        format!(
            "xcodebuild -workspace macos/Runner.xcworkspace -scheme Runner -configuration Release -allowProvisioningUpdates archive -archivePath build/macos/archive-homebrew/Runner.xcarchive CODE_SIGN_STYLE=Manual CODE_SIGN_IDENTITY=Developer ID Application DEVELOPMENT_TEAM=TEAM123 CODE_SIGN_ENTITLEMENTS={root}/macos/Runner/DeveloperID.entitlements ENABLE_HARDENED_RUNTIME=YES OTHER_CODE_SIGN_FLAGS=--options=runtime"
        )
    );
    assert_eq!(
        lines[2],
        format!("xcodebuild -exportArchive -archivePath build/macos/archive-homebrew/Runner.xcarchive -exportPath build/macos/homebrew/export -exportOptionsPlist {root}/macos/ExportOptionsDeveloperID.plist -allowProvisioningUpdates")
    );
    assert_eq!(lines.len(), 3);
    assert!(t.runs().iter().all(|r| r.cwd.as_deref() == Some(d.path())));
}

#[test]
fn the_signing_identity_can_be_chosen() {
    let d = project(true, true);
    let vars = [("APPLE_TEAM_ID", "T"), ("MACOS_SIGN_IDENTITY", "Developer ID Application: X (T)")];
    let (_, t) = go(d.path(), &vars, exporting(d.path()));
    assert!(t.lines()[1].contains("CODE_SIGN_IDENTITY=Developer ID Application: X (T) "), "{:?}", t.lines());
}

#[test]
fn without_a_team_id_nothing_runs() {
    let d = project(true, true);
    let (r, t) = go(d.path(), &[], Fake::default());
    assert_eq!(why(&r), "set APPLE_TEAM_ID.");
    assert!(t.runs().is_empty());
}

#[test]
fn a_missing_plist_or_entitlements_file_is_named_and_nothing_runs() {
    for (plist, ent, name) in [(false, true, "macos/ExportOptionsDeveloperID.plist"), (true, false, "macos/Runner/DeveloperID.entitlements")] {
        let d = project(plist, ent);
        let (r, t) = go(d.path(), &VARS, Fake::default());
        assert!(why(&r).contains(name), "{}", why(&r));
        assert!(t.runs().is_empty());
    }
}

#[test]
fn a_failing_step_stops_the_chain_and_names_itself() {
    for (prog, runs, step) in [("flutter", 1, "flutter build macos"), ("xcodebuild", 2, "xcodebuild archive")] {
        let d = project(true, true);
        let tool = Fake { failing: vec![prog.into()], ..Fake::default() };
        let (r, t) = go(d.path(), &VARS, tool);
        assert!(why(&r).contains(step), "{}", why(&r));
        assert_eq!(t.runs().len(), runs, "{prog}");
    }
}

#[test]
fn a_leftover_app_from_an_earlier_export_is_never_taken() {
    let d = project(true, true);
    let old = d.path().join("build/macos/homebrew/export/Old.app");
    ok(fs::create_dir_all(&old));
    let (r, _) = go(d.path(), &VARS, Fake::default());
    assert!(why(&r).contains("produced no .app"), "{}", why(&r));
    assert!(!old.exists());
}

#[test]
fn an_export_with_two_apps_is_refused() {
    let d = project(true, true);
    let extra = d.path().join("build/macos/homebrew/export/Two.app");
    let mut tool = exporting(d.path());
    tool.on_run.push(("xcodebuild".into(), "-exportArchive".into(), extra));
    let (r, _) = go(d.path(), &VARS, tool);
    assert!(why(&r).contains("2 .app"), "{}", why(&r));
}

#[test]
fn an_export_folder_that_cannot_be_cleared_stops_before_any_build() {
    use std::os::unix::fs::PermissionsExt;
    let d = project(true, true);
    let export = d.path().join("build/macos/homebrew/export");
    ok(fs::create_dir_all(&export));
    ok(fs::write(export.join("Old.app"), "old"));
    ok(fs::set_permissions(&export, fs::Permissions::from_mode(0o555)));
    let (r, t) = go(d.path(), &VARS, Fake::default());
    ok(fs::set_permissions(&export, fs::Permissions::from_mode(0o755))); // lets the temp folder clean up
    assert!(why(&r).contains("clearing"), "{}", why(&r));
    assert!(t.runs().is_empty());
}
