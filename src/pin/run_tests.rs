use super::*;
use std::cell::RefCell;
use std::path::Path;

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn err<T>(r: Result<T>) -> String {
    match r {
        Ok(_) => panic!("expected an error"),
        Err(e) => format!("{e:#}"),
    }
}

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct Cargo {
    calls: RefCell<Vec<String>>,
    fail: bool,
}

impl Exec for Cargo {
    fn run(&self, prog: &str, args: &[&str], cwd: Option<&Path>) -> Result<String> {
        self.calls.borrow_mut().push(format!("{prog} {} @{}", args.join(" "), cwd.map(|c| c.display().to_string()).unwrap_or_default()));
        if self.fail {
            bail!("cargo failed");
        }
        Ok(String::new())
    }
}

fn cargo() -> Cargo {
    Cargo { calls: RefCell::new(vec![]), fail: false }
}

fn repo(root: &Path, name: &str, rev: Option<&str>) -> (String, PathBuf) {
    let dir = root.join(name);
    ok(std::fs::create_dir_all(&dir));
    if let Some(rev) = rev {
        let t = format!("[dependencies]\nactautoma = {{ git = \"https://x/actautoma\", rev = \"{rev}\" }}\n");
        ok(std::fs::write(dir.join("Cargo.toml"), t));
    }
    (name.to_string(), dir)
}

fn go(f: &[(String, PathBuf)], rev: Option<&str>, c: &Cargo) -> (Result<()>, String) {
    let mut out = Vec::new();
    let r = run(f, "actautoma", rev, c, &mut out);
    (r, String::from_utf8_lossy(&out).into_owned())
}

#[test]
fn agreeing_repos_are_listed_and_succeed() {
    let t = ok(tempfile::tempdir());
    let f = [repo(t.path(), "a", Some(A)), repo(t.path(), "b", Some(A)), repo(t.path(), "docs", None)];
    let (r, out) = go(&f, None, &cargo());
    assert!(r.is_ok(), "{r:?}");
    assert!(out.contains(&format!("a{}actautoma {A}", " ".repeat(25))), "{out}");
}

#[test]
fn disagreeing_repos_fail_naming_the_fix() {
    let t = ok(tempfile::tempdir());
    let f = [repo(t.path(), "a", Some(A)), repo(t.path(), "b", Some(B))];
    let (r, _) = go(&f, None, &cargo());
    assert!(err(r).contains("actrepo pin actautoma <rev>"));
}

#[test]
fn no_repo_with_the_dependency_is_an_error() {
    let t = ok(tempfile::tempdir());
    let (r, _) = go(&[repo(t.path(), "docs", None)], None, &cargo());
    assert!(r.is_err());
}

#[test]
fn a_rev_rewrites_every_repo_and_refreshes_each_lock() {
    let t = ok(tempfile::tempdir());
    let f = [repo(t.path(), "a", Some(A)), repo(t.path(), "b", Some(B))];
    let c = cargo();
    let (r, _) = go(&f, Some(B), &c);
    assert!(r.is_ok(), "{r:?}");
    assert!(ok(std::fs::read_to_string(f[0].1.join("Cargo.toml"))).contains(B));
    assert_eq!(c.calls.borrow().len(), 1, "b already on B: untouched");
    assert!(c.calls.borrow()[0].starts_with("cargo update -p actautoma @"));
}

#[test]
fn a_failing_lock_refresh_fails_loudly_naming_the_repo() {
    let t = ok(tempfile::tempdir());
    let c = Cargo { calls: RefCell::new(vec![]), fail: true };
    let (r, _) = go(&[repo(t.path(), "a", Some(A))], Some(B), &c);
    assert!(err(r).contains("a: cargo update -p actautoma failed"));
}

#[test]
fn an_invalid_rev_is_refused_before_anything_is_written() {
    let t = ok(tempfile::tempdir());
    let f = [repo(t.path(), "a", Some(A))];
    let (r, _) = go(&f, Some("main"), &cargo());
    assert!(r.is_err());
    assert!(ok(std::fs::read_to_string(f[0].1.join("Cargo.toml"))).contains(A));
}
