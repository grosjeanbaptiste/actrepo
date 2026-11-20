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
        base.join("lib/package.json"),
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

#[test]
fn a_markdown_project_declared_in_actmake_toml_builds_through_pandoc_then_the_browser() {
    let f = fx();
    let doc = f.base.join("doc");
    ok(fs::create_dir_all(&doc));
    ok(fs::write(
        doc.join("actmake.toml"),
        "engine = \"markdown\"\n",
    ));
    ok(fs::write(doc.join("guide.md"), "# T\n"));
    let state = f.base.join("state/projects.txt");
    let mut text = ok(fs::read_to_string(&state));
    text.push_str(&format!("doc|{}\n", doc.display()));
    ok(fs::write(&state, text));
    for tool in ["pandoc", "chrome"] {
        let p = f.base.join("bin").join(tool);
        ok(fs::write(
            &p,
            format!("#!/bin/sh\necho \"{tool} $*\" >> \"$STAGE/calls.log\"\n"),
        ));
        ok(fs::set_permissions(&p, fs::Permissions::from_mode(0o755)));
    }
    let o = ok(Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .args(["doc", "build"])
        .env("DOTFILES_STATE_ROOT", f.base.join("state"))
        .env("STAGE", &f.base)
        .env("ACTMAKE_CHROME", f.base.join("bin/chrome"))
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", f.base.join("bin").display()),
        )
        .output());
    assert!(o.status.success(), "{}", err(&o));
    let calls = ok(fs::read_to_string(f.base.join("calls.log")));
    assert!(
        calls
            .lines()
            .next()
            .is_some_and(|l| l.starts_with("pandoc --standalone guide.md")),
        "{calls}"
    );
    assert!(
        calls
            .lines()
            .nth(1)
            .is_some_and(|l| l.contains("--print-to-pdf=") && l.contains("guide.pdf")),
        "{calls}"
    );
}

fn project(f: &Fx, name: &str, marker: (&str, &str)) {
    let dir = f.base.join(name);
    ok(fs::create_dir_all(&dir));
    ok(fs::write(dir.join(marker.0), marker.1));
    let state = f.base.join("state/projects.txt");
    let mut text = ok(fs::read_to_string(&state));
    text.push_str(&format!("{name}|{}\n", dir.display()));
    ok(fs::write(&state, text));
    let tool = f.base.join("bin/dotfiles");
    ok(fs::write(
        &tool,
        "#!/bin/sh\necho \"dotfiles $*\" >> \"$STAGE/calls.log\"\n",
    ));
    ok(fs::set_permissions(
        &tool,
        fs::Permissions::from_mode(0o755),
    ));
}

fn calls(f: &Fx) -> Vec<String> {
    ok(fs::read_to_string(f.base.join("calls.log")))
        .lines()
        .map(String::from)
        .collect()
}

#[test]
fn a_crate_builds_through_the_heavy_queue_in_release() {
    let f = fx();
    project(&f, "tool", ("Cargo.toml", "[package]\n"));
    let o = repo(&f, &["tool", "build", "--release"]);
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(
        calls(&f),
        vec!["dotfiles heavy run -- cargo build --release"]
    );
}

#[test]
fn a_flutter_app_builds_for_the_target_given() {
    let f = fx();
    project(&f, "app", ("pubspec.yaml", "flutter:\n"));
    let o = repo(&f, &["app", "build", "--target", "apk"]);
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(
        calls(&f),
        vec!["dotfiles heavy run -- flutter build apk --debug"]
    );
}

#[test]
fn a_flutter_app_without_a_target_is_a_usage_error() {
    let f = fx();
    project(&f, "app", ("pubspec.yaml", "flutter:\n"));
    let o = repo(&f, &["app", "build"]);
    assert!(!o.status.success());
    assert!(err(&o).contains("pass a target"), "{}", err(&o));
}
