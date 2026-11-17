use super::*;
use std::fs;
use std::process::Command;

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn git(dir: &Path, args: &[&str]) {
    let o = ok(Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .current_dir(dir)
        .output());
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

struct Fx {
    _d: tempfile::TempDir,
    env: Env,
    old: PathBuf,
}

/// `<home>/Source/old` is a repo with origin `git@github.com:me/old.git`; `<home>/machine` mentions it.
fn fx() -> Fx {
    let d = ok(tempfile::tempdir());
    let home = ok(d.path().canonicalize());
    let old = home.join("Source/old");
    ok(fs::create_dir_all(&old));
    git(&old, &["init", "-q", "-b", "main"]);
    git(
        &old,
        &["remote", "add", "origin", "git@github.com:me/old.git"],
    );
    ok(fs::create_dir_all(home.join("machine")));
    ok(fs::write(
        home.join("machine/projects.txt"),
        format!("old|{}\n", old.display()),
    ));
    let env = Env {
        state: home.join("machine"),
        dotfiles: home.join("dotfiles"), // absent on purpose
        cargo_bin: home.join(".cargo/bin"),
        home,
    };
    Fx { _d: d, env, old }
}

fn opts(f: &Fx, local_only: bool, dry_run: bool) -> Opts {
    Opts {
        new: "new".into(),
        dir: f.old.clone(),
        local_only,
        dry_run,
    }
}

fn go(f: &Fx, o: &Opts) -> (Result<()>, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let r = run(
        o,
        &f.env,
        &Gh::new(ProcessRunner),
        &ProcessRunner,
        &mut out,
        &mut err,
    );
    (
        r,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

#[test]
fn a_local_rename_moves_the_folder_repoints_origin_and_rewrites_references() {
    let f = fx();
    let (r, out, err) = go(&f, &opts(&f, true, false));
    assert!(r.is_ok(), "{err}");
    let new = f.old.with_file_name("new");
    assert!(new.is_dir() && !f.old.exists(), "{out}");
    let origin = ok(Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(&new)
        .output());
    assert_eq!(
        String::from_utf8_lossy(&origin.stdout).trim(),
        "git@github.com:me/new.git"
    );
    let registry = ok(fs::read_to_string(f.env.state.join("projects.txt")));
    assert!(registry.contains(&new.display().to_string()), "{registry}");
    assert!(
        err.contains("skipping") && err.contains("dotfiles (absent)"),
        "{err}"
    );
}

#[test]
fn a_dry_run_lists_the_rewrites_and_changes_nothing() {
    let f = fx();
    let (r, out, err) = go(&f, &opts(&f, true, true));
    assert!(r.is_ok(), "{err}");
    assert!(
        out.contains("me/old -> me/new") && out.contains("dry run: nothing was changed"),
        "{out}"
    );
    assert!(f.old.is_dir() && !f.old.with_file_name("new").exists());
}

#[test]
fn a_folder_without_origin_is_an_error_naming_it() {
    let f = fx();
    git(&f.old, &["remote", "remove", "origin"]);
    let (r, _, _) = go(&f, &opts(&f, true, false));
    let msg = format!(
        "{:#}",
        r.err().unwrap_or_else(|| panic!("rename succeeded"))
    );
    assert!(msg.contains("has no origin remote"), "{msg}");
    assert!(f.old.is_dir());
}

#[test]
fn an_existing_target_is_refused_before_any_change() {
    let f = fx();
    ok(fs::create_dir_all(f.old.with_file_name("new")));
    let (r, _, _) = go(&f, &opts(&f, true, false));
    let msg = format!(
        "{:#}",
        r.err().unwrap_or_else(|| panic!("rename succeeded"))
    );
    assert!(msg.contains("already exists"), "{msg}");
    assert!(f.old.is_dir());
}

#[test]
fn env_reads_home_state_dotfiles_and_cargo_from_the_environment() {
    let var = |k: &str| match k {
        "HOME" => Some("/h".to_string()),
        "DOTFILES_REPO_ROOT" => Some("/df".to_string()),
        "CARGO_HOME" => Some("/c".to_string()),
        _ => None,
    };
    let e = ok(env_from(var));
    assert_eq!(
        (e.state, e.dotfiles, e.cargo_bin),
        ("/h/machine".into(), "/df".into(), "/c/bin".into())
    );
    assert!(env_from(|_| None).is_err());
}
