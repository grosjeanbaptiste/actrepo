//! `--json` on `list`, `status` and `worktree list`, and the `repo` library, on temp state and repositories.
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn repo(state: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_repo"))
        .args(args)
        .env("DOTFILES_STATE_ROOT", state)
        .env("HOME", state)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("cannot run repo: {e}"))
}

fn stdout(o: &Output) -> Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    ok(serde_json::from_slice(&o.stdout))
}

fn git(dir: &Path, args: &[&str]) {
    let s = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .current_dir(dir)
        .status();
    assert!(ok(s).success(), "git {args:?}");
}

struct Fx {
    _d: tempfile::TempDir,
    state: std::path::PathBuf,
    alpha: std::path::PathBuf,
}

/// One registered repository `alpha` (main, one commit, no remote, one untracked file).
fn fx() -> Fx {
    let d = ok(tempfile::tempdir());
    let base = ok(d.path().canonicalize());
    let (state, alpha) = (base.join("state"), base.join("alpha"));
    ok(fs::create_dir_all(&state));
    ok(fs::create_dir_all(&alpha));
    git(&alpha, &["init", "-q", "-b", "main"]);
    ok(fs::write(alpha.join("f"), "1"));
    git(&alpha, &["add", "-A"]);
    git(&alpha, &["commit", "-q", "-m", "one"]);
    ok(fs::write(alpha.join("new"), "x"));
    ok(fs::write(
        state.join("projects.txt"),
        format!("alpha|{}\n", alpha.display()),
    ));
    Fx {
        _d: d,
        state,
        alpha,
    }
}

#[test]
fn list_json_is_an_array_of_name_and_path() {
    let f = fx();
    let v = stdout(&repo(&f.state, &f.state, &["list", "--json"]));
    assert_eq!(
        v,
        json!([{"name": "alpha", "path": f.alpha.display().to_string()}])
    );
}

#[test]
fn list_json_on_an_empty_registry_is_an_empty_array() {
    let f = fx();
    ok(fs::remove_file(f.state.join("projects.txt")));
    assert_eq!(
        stdout(&repo(&f.state, &f.state, &["list", "--json"])),
        json!([])
    );
}

#[test]
fn status_json_has_typed_fields_and_null_for_the_missing() {
    let f = fx();
    let v = stdout(&repo(&f.state, &f.state, &["status", "--json"]));
    assert_eq!(
        v,
        json!([{"name": "alpha", "path": f.alpha.display().to_string(), "branch": "main",
                "ahead": null, "behind": null, "dirty": 1, "remote": null}])
    );
}

#[test]
fn status_json_of_one_project_is_an_object() {
    let f = fx();
    let v = stdout(&repo(&f.state, &f.state, &["status", "alpha", "--json"]));
    assert_eq!(v["name"], "alpha");
    assert_eq!(v["branch"], "main");
}

#[test]
fn status_json_of_an_unknown_project_fails_with_its_name() {
    let f = fx();
    let o = repo(&f.state, &f.state, &["status", "nope", "--json"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("Unknown project: nope"));
}

#[test]
fn worktree_list_json_names_the_main_and_the_linked_worktrees() {
    let f = fx();
    let linked = f.alpha.with_file_name("alpha-feat");
    git(
        &f.alpha,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "feat",
            &linked.display().to_string(),
        ],
    );
    let v = stdout(&repo(&f.state, &f.alpha, &["worktree", "list", "--json"]));
    assert_eq!(
        v,
        json!([
            {"path": f.alpha.display().to_string(), "branch": "main", "main": true, "detached": false, "prunable": false},
            {"path": linked.display().to_string(), "branch": "feat", "main": false, "detached": false, "prunable": false},
        ])
    );
}

#[test]
fn worktree_list_text_is_one_line_per_worktree() {
    let f = fx();
    let o = repo(&f.state, &f.alpha, &["worktree", "list"]);
    assert!(o.status.success());
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        format!("{}  main\n", f.alpha.display())
    );
}

#[test]
fn the_library_exposes_the_registry() {
    let entries = ok(repo::registry::file::parse("a|/x\n"));
    assert_eq!(entries[0].name, "a");
}

/// `repo worktree list --owners` with `actsessions` replaced by a script printing `answer`.
fn owners_run(f: &Fx, answer: &str, extra: &[&str]) -> Output {
    let bin = f.alpha.with_file_name("bin");
    ok(fs::create_dir_all(&bin));
    let script = bin.join("actsessions");
    ok(fs::write(
        &script,
        format!("#!/bin/sh\ncat <<'EOF'\n{answer}\nEOF\n"),
    ));
    ok(Command::new("chmod").arg("+x").arg(&script).status());
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut args = vec!["worktree", "list", "--owners"];
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_repo"))
        .args(args)
        .env("PATH", path)
        .current_dir(&f.alpha)
        .output()
        .unwrap_or_else(|e| panic!("cannot run repo: {e}"))
}

#[test]
fn worktree_list_owners_names_the_live_session_working_in_it() {
    let f = fx();
    let answer = format!(
        r#"[{{"id":"s1","agent":"claude","status":"active","cwd":"{}"}},{{"id":"s2","agent":"claude","status":"idle","cwd":"{}"}}]"#,
        f.alpha.join("src").display(),
        f.alpha.display()
    );
    let v = stdout(&owners_run(&f, &answer, &["--json"]));
    assert_eq!(v[0]["owners"], json!([{"id": "s1", "agent": "claude"}]));
}

#[test]
fn worktree_list_owners_fails_loudly_when_actsessions_answers_garbage() {
    let f = fx();
    let o = owners_run(&f, "garbage", &[]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("actsessions list --json"));
}
