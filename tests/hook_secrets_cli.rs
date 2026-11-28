//! `actrepo hook install` + `hook secrets` end to end: the installed hooks really refuse a secret
//! at commit and at push. No network; gitleaks must be on PATH (the hooks fail without it).
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

// Shaped like a GitHub token, made up for this test: not a credential.
const TOKEN: &str = "ghp_aB3dE6gH9jK2mN5pQ8sT1vW4yZ7bC0eF3hJ6";

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

/// A work repository with the hooks installed, its bare remote, and a PATH holding `actrepo`.
struct Fixture {
    _tmp: tempfile::TempDir,
    work: PathBuf,
    path: String,
}

fn git(f: &Fixture, args: &[&str]) -> Output {
    ok(Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .env("PATH", &f.path)
        .current_dir(&f.work)
        .output())
}

fn fixture() -> Fixture {
    let tmp = ok(tempfile::tempdir());
    let base = ok(tmp.path().canonicalize());
    let (work, remote, shim) = (
        base.join("work"),
        base.join("remote.git"),
        base.join("shim"),
    );
    for p in [&work, &remote, &shim] {
        ok(fs::create_dir_all(p));
    }
    let bin = env!("CARGO_BIN_EXE_actrepo");
    ok(std::os::unix::fs::symlink(bin, shim.join("actrepo")));
    let path = format!(
        "{}:{}",
        shim.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let f = Fixture {
        _tmp: tmp,
        work,
        path,
    };
    ok(Command::new("git")
        .args(["init", "-q", "--bare", "-b", "main"])
        .current_dir(&remote)
        .output());
    git(&f, &["init", "-q", "-b", "main"]);
    git(
        &f,
        &["remote", "add", "origin", &remote.display().to_string()],
    );
    let out = ok(Command::new(bin)
        .args(["hook", "install"])
        .current_dir(&f.work)
        .output());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    f
}

fn stage(f: &Fixture, name: &str, text: &str) {
    ok(fs::write(Path::new(&f.work).join(name), text));
    git(f, &["add", name]);
}

#[test]
fn a_commit_with_a_secret_is_refused_by_the_installed_hook() {
    let f = fixture();
    stage(&f, "a.env", &format!("GITHUB_TOKEN={TOKEN}\n"));
    let o = git(&f, &["commit", "-q", "-m", "oops"]);
    assert!(!o.status.success(), "the commit must be refused");
    assert!(String::from_utf8_lossy(&o.stderr).contains("secret scan"));
    assert!(
        !git(&f, &["rev-parse", "-q", "--verify", "HEAD"])
            .status
            .success()
    );
}

#[test]
fn a_clean_commit_passes_the_installed_hook() {
    let f = fixture();
    stage(&f, "a.txt", "nothing to hide\n");
    let o = git(&f, &["commit", "-q", "-m", "ok"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn a_push_carrying_a_secret_is_refused_by_the_installed_hook() {
    let f = fixture();
    stage(&f, "a.env", &format!("GITHUB_TOKEN={TOKEN}\n"));
    // Setup only: the commit is made with the hooks off, as if it predated the hook.
    let c = git(
        &f,
        &[
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-q",
            "-m",
            "old",
        ],
    );
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    let o = git(&f, &["push", "-q", "-u", "origin", "main"]);
    assert!(!o.status.success(), "the push must be refused");
    assert!(String::from_utf8_lossy(&o.stderr).contains("secret scan"));
}

fn install(dir: &Path, flags: &[&str]) -> String {
    let out = ok(Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .args(["hook", "install"])
        .args(flags)
        .current_dir(dir)
        .output());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn bare_repo() -> tempfile::TempDir {
    let tmp = ok(tempfile::tempdir());
    ok(Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(tmp.path())
        .output());
    tmp
}

#[test]
fn a_dry_run_says_what_it_would_write_and_writes_nothing() {
    let repo = bare_repo();
    let said = install(repo.path(), &["-n"]);
    for hook in ["post-commit", "pre-commit", "pre-push"] {
        assert!(said.contains("would write"), "{said}");
        assert!(said.contains(hook), "{said}");
        assert!(!repo.path().join(".git/hooks").join(hook).exists());
    }
    assert!(!said.contains("wrote "), "{said}");
}

#[test]
fn a_real_install_says_it_wrote_and_a_second_one_says_unchanged() {
    let repo = bare_repo();
    let first = install(repo.path(), &[]);
    assert!(first.contains("wrote "), "{first}");
    assert!(!first.contains("would write"), "{first}");
    let again = install(repo.path(), &[]);
    assert!(again.contains("unchanged "), "{again}");
    assert!(!again.contains("wrote "), "{again}");
}
