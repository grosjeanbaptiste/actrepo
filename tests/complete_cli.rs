//! `repo __complete`, `repo completions` and `repo <dir> run|install` end to end: a real registry, scripted `adb`, `xcrun` and `flutter`.
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

struct Fx {
    _d: tempfile::TempDir,
    base: PathBuf,
}

fn script(path: &Path, text: &str) {
    ok(fs::write(path, text));
    ok(fs::set_permissions(path, fs::Permissions::from_mode(0o755)));
}

fn fx() -> Fx {
    let d = ok(tempfile::tempdir());
    let base = ok(d.path().canonicalize());
    for p in ["app", "state", "bin"] {
        ok(fs::create_dir_all(base.join(p)));
    }
    ok(fs::write(base.join("app/pubspec.yaml"), "flutter:\n"));
    ok(fs::write(
        base.join("state/projects.txt"),
        format!("app|{}\nother|/nowhere\n", base.join("app").display()),
    ));
    script(
        &base.join("bin/xcrun"),
        "#!/bin/sh\necho '{\"devices\":{}}'\n",
    );
    script(
        &base.join("bin/adb"),
        "#!/bin/sh\nprintf 'List of devices attached\\nR59X\\tdevice model:Pixel_7\\n'\n",
    );
    script(
        &base.join("bin/flutter"),
        "#!/bin/sh\necho \"flutter $*\" >> \"$STAGE/calls.log\"\n",
    );
    Fx { _d: d, base }
}

fn repo(f: &Fx, args: &[&str]) -> Output {
    ok(Command::new(env!("CARGO_BIN_EXE_repo"))
        .args(args)
        .env("DOTFILES_STATE_ROOT", f.base.join("state"))
        .env("STAGE", &f.base)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", f.base.join("bin").display()),
        )
        .output())
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn complete_offers_registered_names_then_verbs_then_devices() {
    let f = fx();
    assert_eq!(out(&repo(&f, &["__complete", "a"])), "app\n");
    assert_eq!(out(&repo(&f, &["__complete", "app", ""])), "run\ninstall\n");
    assert!(out(&repo(&f, &["__complete", "app", "run", ""])).contains("R59X\tPixel 7\n"));
    assert!(out(&repo(&f, &["__complete", "app", "run", "pix"])).starts_with("R59X\tPixel 7"));
}

#[test]
fn completions_zsh_calls_complete_and_uses_fzf_when_present() {
    let f = fx();
    let o = repo(&f, &["completions", "zsh"]);
    assert!(o.status.success());
    let s = out(&o);
    assert!(
        s.contains("repo __complete") && s.contains("fzf") && s.contains("compdef"),
        "{s}"
    );
}

#[test]
fn an_unknown_shell_is_refused() {
    let f = fx();
    let o = repo(&f, &["completions", "tcsh"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("unsupported shell tcsh"));
}

#[test]
fn run_on_a_named_device_calls_flutter_with_its_id() {
    let f = fx();
    let o = repo(&f, &["app", "run", "pixel"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let log = ok(fs::read_to_string(f.base.join("calls.log")));
    assert_eq!(log, "flutter run -d R59X --debug\n");
}

#[test]
fn an_unknown_device_fails_and_lists_the_available_ones() {
    let f = fx();
    let o = repo(&f, &["app", "run", "galaxy"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("Pixel 7 (R59X)"));
    assert!(!f.base.join("calls.log").exists());
}
