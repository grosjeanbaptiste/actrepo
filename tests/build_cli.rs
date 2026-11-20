//! `repo <dir> build` end to end: a real registry, a scripted `pdflatex`.
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

struct Fx {
    _d: tempfile::TempDir,
    base: PathBuf,
}

fn fx() -> Fx {
    let d = ok(tempfile::tempdir());
    let base = ok(d.path().canonicalize());
    for p in ["thesis", "lib", "state", "bin"] {
        ok(fs::create_dir_all(base.join(p)));
    }
    ok(fs::write(base.join("thesis/main.tex"), ""));
    ok(fs::write(
        base.join("lib/Cargo.toml"),
        "[package]\nname = \"x\"\n",
    ));
    ok(fs::write(
        base.join("state/projects.txt"),
        format!(
            "thesis|{}\nlib|{}\n",
            base.join("thesis").display(),
            base.join("lib").display()
        ),
    ));
    let tool = base.join("bin/pdflatex");
    ok(fs::write(
        &tool,
        "#!/bin/sh\necho \"pdflatex $*\" >> \"$STAGE/calls.log\"\n",
    ));
    ok(fs::set_permissions(
        &tool,
        fs::Permissions::from_mode(0o755),
    ));
    Fx { _d: d, base }
}

fn repo(f: &Fx, args: &[&str]) -> Output {
    ok(Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .args(args)
        .env("DOTFILES_STATE_ROOT", f.base.join("state"))
        .env("STAGE", &f.base)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", f.base.join("bin").display()),
        )
        .output())
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn build_runs_pdflatex_three_times_in_the_project_folder() {
    let f = fx();
    let o = repo(&f, &["thesis", "build"]);
    assert!(o.status.success(), "{}", err(&o));
    let calls = ok(fs::read_to_string(f.base.join("calls.log")));
    assert_eq!(calls.lines().count(), 3, "{calls}");
    assert!(calls.contains("pdflatex -interaction=nonstopmode main.tex"));
}

#[test]
fn a_project_without_an_engine_is_refused_naming_its_kind() {
    let f = fx();
    let o = repo(&f, &["lib", "build"]);
    assert!(!o.status.success());
    assert!(
        err(&o).contains("lib is a Package project: no build engine for it yet"),
        "{}",
        err(&o)
    );
    assert!(!f.base.join("calls.log").exists());
}

#[test]
fn a_failing_pass_fails_the_command_naming_the_program() {
    let f = fx();
    ok(fs::write(
        f.base.join("bin/pdflatex"),
        "#!/bin/sh\nexit 3\n",
    ));
    let o = repo(&f, &["thesis", "build"]);
    assert!(!o.status.success());
    assert!(err(&o).contains("pdflatex failed"), "{}", err(&o));
}

#[test]
fn clean_removes_the_previous_pdf_and_builds_again() {
    let f = fx();
    ok(fs::write(f.base.join("thesis/main.pdf"), "old"));
    let o = repo(&f, &["thesis", "build", "--clean"]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(!f.base.join("thesis/main.pdf").exists());
    let calls = ok(fs::read_to_string(f.base.join("calls.log")));
    assert_eq!(calls.lines().count(), 6, "{calls}");
}
