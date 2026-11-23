//! `actrepo hook` end to end: the installed stub really pushes after a commit. No network.
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn git(dir: &Path, args: &[&str]) -> String {
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
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

#[test]
fn a_commit_on_a_branch_is_pushed_by_the_installed_hook() {
    let d = ok(tempfile::tempdir());
    let base = ok(d.path().canonicalize());
    let (work, remote, state) = (
        base.join("work"),
        base.join("remote.git"),
        base.join("state"),
    );
    for p in [&work, &remote] {
        ok(fs::create_dir_all(p));
    }
    git(&remote, &["init", "-q", "--bare", "-b", "main"]);
    git(&work, &["init", "-q", "-b", "main"]);
    git(
        &work,
        &["remote", "add", "origin", &remote.display().to_string()],
    );
    ok(fs::write(work.join("a"), "a"));
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "a"]);
    git(&work, &["push", "-q", "-u", "origin", "main"]);
    git(&work, &["switch", "-q", "-c", "feat/x"]);
    let bin = env!("CARGO_BIN_EXE_actrepo");
    let out = ok(Command::new(bin)
        .args(["hook", "install"])
        .current_dir(&work)
        .output());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    ok(fs::write(work.join("b"), "b"));
    git(&work, &["add", "-A"]);
    // The stub calls `actrepo` from PATH: point it at the binary under test.
    let shim = base.join("shim");
    ok(fs::create_dir_all(&shim));
    ok(std::os::unix::fs::symlink(bin, shim.join("actrepo")));
    let path = format!(
        "{}:{}",
        shim.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let o = ok(Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            "b",
        ])
        .env("PATH", path)
        .env("XDG_STATE_HOME", &state)
        .current_dir(&work)
        .output());
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let head = git(&work, &["rev-parse", "HEAD"]);
    let end = Instant::now() + Duration::from_secs(20);
    let mut pushed = String::new();
    while Instant::now() < end {
        let r = Command::new("git")
            .args(["rev-parse", "feat/x"])
            .current_dir(&remote)
            .output();
        pushed = r
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        if pushed == head {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(
        pushed,
        head,
        "log: {}",
        fs::read_to_string(state.join("actrepo/hook.log")).unwrap_or_default()
    );
}
