use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn folder(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("actrepo-kind-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d); // a leftover of a crashed run
    fs::create_dir_all(d.join("scripts")).unwrap_or_else(|e| panic!("{e}"));
    for (file, body) in files {
        fs::write(d.join(file), body).unwrap_or_else(|e| panic!("{e}"));
    }
    d
}

fn kind(name: &str, files: &[(&str, &str)]) -> Result<Kind, KindError> {
    detect(&folder(name, files))
}

#[test]
fn a_pubspec_with_flutter_is_a_flutter_app() {
    let k = kind(
        "flutter",
        &[(
            "pubspec.yaml",
            "dependencies:\n  flutter:\n    sdk: flutter\n",
        )],
    );
    assert_eq!(k, Ok(Kind::Flutter));
    assert_eq!(Kind::Flutter.engine(), Some(actmake::Engine::Flutter));
}

#[test]
fn a_dart_package_is_a_package() {
    assert_eq!(
        kind("dart", &[("pubspec.yaml", "name: x\n")]),
        Ok(Kind::Package)
    );
    assert_eq!(
        Kind::Package.engine(),
        None,
        "dart and npm have no engine yet"
    );
}

#[test]
fn dioxus_toml_is_a_dioxus_app_even_with_a_cargo_toml() {
    let k = kind(
        "dioxus",
        &[("Dioxus.toml", ""), ("Cargo.toml", "[package]\n")],
    );
    assert_eq!(k, Ok(Kind::Dioxus));
    assert_eq!(Kind::Dioxus.engine(), Some(actmake::Engine::Dioxus));
}

#[test]
fn an_executable_device_hook_makes_an_app_and_wins_over_dioxus() {
    let d = folder(
        "hook",
        &[
            ("Dioxus.toml", ""),
            ("scripts/device-install.sh", "#!/bin/sh\n"),
        ],
    );
    let hook = d.join("scripts/device-install.sh");
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(detect(&d), Ok(Kind::Hook));
    assert_eq!(Kind::Hook.engine(), None);
}

#[test]
fn a_hook_that_is_not_executable_does_not_count() {
    let k = kind("noexec", &[("scripts/device-install.sh", "")]);
    assert!(matches!(k, Err(KindError::Unknown(_))));
}

#[test]
fn a_cargo_crate_with_axum_is_an_api_otherwise_a_crate() {
    let api = kind("api", &[("Cargo.toml", "[dependencies]\naxum = \"0.8\"\n")]);
    assert_eq!(
        (api, Kind::Api.engine()),
        (Ok(Kind::Api), Some(actmake::Engine::Cargo))
    );
    assert_eq!(
        kind("lib", &[("Cargo.toml", "[package]\nname = \"x\"\n")]),
        Ok(Kind::Crate)
    );
    assert_eq!(Kind::Crate.engine(), Some(actmake::Engine::Cargo));
}

#[test]
fn an_npm_project_is_a_package() {
    assert_eq!(kind("npm", &[("package.json", "{}")]), Ok(Kind::Package));
}

#[test]
fn a_tex_file_is_a_tex_project_and_beats_tooling_manifests() {
    assert_eq!(
        kind("tex", &[("main.tex", ""), ("package.json", "{}")]),
        Ok(Kind::Tex)
    );
    assert_eq!(Kind::Tex.engine(), Some(actmake::Engine::Tex));
}

#[test]
fn an_unrecognised_folder_is_an_error_naming_it() {
    let d = folder("empty", &[("README.md", "")]);
    let e = detect(&d).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains(&d.display().to_string()), "{e}");
}

#[test]
fn a_folder_that_cannot_be_read_is_an_error_not_unknown() {
    let missing = std::env::temp_dir().join("actrepo-kind-does-not-exist");
    assert!(matches!(detect(&missing), Err(KindError::Unreadable(_))));
}

#[test]
fn actmake_toml_declares_a_document_and_beats_every_inference() {
    let md = kind(
        "doc-md",
        &[
            ("actmake.toml", "engine = \"markdown\"\n"),
            ("package.json", "{}"),
        ],
    );
    assert_eq!(md, Ok(Kind::Markdown));
    assert_eq!(Kind::Markdown.engine(), Some(actmake::Engine::Markdown));
    let html = kind(
        "doc-html",
        &[
            ("actmake.toml", "# a page\nengine = 'html'\n"),
            ("Cargo.toml", "axum"),
        ],
    );
    assert_eq!(html, Ok(Kind::Html));
    assert_eq!(Kind::Html.engine(), Some(actmake::Engine::Html));
    assert_eq!(
        kind("doc-tex", &[("actmake.toml", "engine = \"tex\"\n")]),
        Ok(Kind::Tex)
    );
}

#[test]
fn an_unknown_engine_or_a_missing_key_is_an_error_not_a_guess() {
    let bad = kind(
        "doc-bad",
        &[("actmake.toml", "engine = \"word\"\n"), ("main.tex", "")],
    );
    assert!(
        matches!(&bad, Err(KindError::BadConfig(m)) if m.contains("word")),
        "{bad:?}"
    );
    let none = kind(
        "doc-none",
        &[("actmake.toml", "# empty\n"), ("main.tex", "")],
    );
    assert!(
        matches!(&none, Err(KindError::BadConfig(m)) if m.contains("engine")),
        "{none:?}"
    );
}

#[test]
fn bad_config_is_reported_with_its_reason() {
    let e = KindError::BadConfig("no engine".into()).to_string();
    assert_eq!(e, "invalid actmake.toml: no engine");
}

#[test]
fn only_apps_run_on_a_device_and_each_names_its_tooling() {
    use actdevice::project::Kind as Tooling;
    assert_eq!(Kind::Flutter.device_tooling(), Some(Tooling::Flutter));
    assert_eq!(Kind::Hook.device_tooling(), Some(Tooling::Hook));
    assert_eq!(Kind::Dioxus.device_tooling(), Some(Tooling::Dioxus));
    for k in [
        Kind::Api,
        Kind::Crate,
        Kind::Package,
        Kind::Tex,
        Kind::Html,
        Kind::Markdown,
    ] {
        assert_eq!(k.device_tooling(), None, "{k:?}");
    }
}
