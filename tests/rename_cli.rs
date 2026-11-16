//! `repo rename` end to end in a temp HOME, `--local-only` so no platform call is made.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = ok(Command::new("git").args(args).current_dir(dir).output());
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

struct World {
    home: tempfile::TempDir,
}

impl World {
    /// `$HOME/Source/old` (a repo whose origin is `me/old`), a state repo naming it, a registry row.
    fn new() -> Self {
        let home = ok(tempfile::tempdir());
        let h = home.path();
        let old = h.join("Source/old");
        ok(fs::create_dir_all(&old));
        git(&old, &["init", "-q"]);
        git(
            &old,
            &["remote", "add", "origin", "git@github.com:me/old.git"],
        );
        ok(fs::create_dir_all(h.join("machine")));
        ok(fs::write(
            h.join("machine/note.txt"),
            format!("see {}\n", old.display()),
        ));
        ok(fs::write(
            h.join("machine/projects.txt"),
            format!("old|{}\nother|/x/other\n", old.display()),
        ));
        World { home }
    }
    fn path(&self, p: &str) -> PathBuf {
        self.home.path().join(p)
    }
    fn repo(&self, args: &[&str]) -> Output {
        let mut c = Command::new(env!("CARGO_BIN_EXE_repo"));
        c.args(args)
            .env("HOME", self.home.path())
            .env("DOTFILES_STATE_ROOT", self.path("machine"))
            .env("DOTFILES_REPO_ROOT", self.path("no-dotfiles"))
            .env("CARGO_HOME", self.path("cargo"))
            .current_dir(self.home.path());
        ok(c.output())
    }
}

fn text(o: &[u8]) -> String {
    String::from_utf8_lossy(o).to_string()
}

#[test]
fn a_local_only_rename_moves_the_folder_and_fixes_every_reference() {
    let w = World::new();
    let o = w.repo(&["rename", "me/old", "new", "--local-only"]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(!w.path("Source/old").exists() && w.path("Source/new").is_dir());
    assert_eq!(
        git(&w.path("Source/new"), &["remote", "get-url", "origin"]),
        "git@github.com:me/new.git"
    );
    let note = ok(fs::read_to_string(w.path("machine/note.txt")));
    assert!(
        note.contains("Source/new") && !note.contains("Source/old"),
        "{note}"
    );
    let reg = ok(fs::read_to_string(w.path("machine/projects.txt")));
    assert!(
        reg.contains(&format!("new|{}", w.path("Source/new").display())),
        "{reg}"
    );
    assert!(
        !reg.contains("old|") && reg.contains("other|/x/other"),
        "{reg}"
    );
    assert!(
        text(&o.stderr).contains("skipping"),
        "absent folders are named: {}",
        text(&o.stderr)
    );
}

#[test]
fn a_dry_run_prints_the_renames_and_changes_nothing() {
    let w = World::new();
    let o = w.repo(&["rename", "me/old", "new", "--dry-run"]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(text(&o.stdout).contains("me/old -> me/new") && text(&o.stdout).contains("dry run"));
    assert!(w.path("Source/old").is_dir() && !w.path("Source/new").exists());
    assert!(ok(fs::read_to_string(w.path("machine/projects.txt"))).contains("old|"));
}

#[test]
fn an_unregistered_repository_without_dir_fails_and_names_the_flag() {
    let w = World::new();
    let o = w.repo(&["rename", "me/ghost", "new", "--local-only"]);
    assert!(!o.status.success());
    assert!(text(&o.stderr).contains("--dir"), "{}", text(&o.stderr));
}

#[test]
fn an_existing_target_folder_is_refused_before_any_change() {
    let w = World::new();
    ok(fs::create_dir_all(w.path("Source/new")));
    let o = w.repo(&["rename", "me/old", "new", "--local-only"]);
    assert!(!o.status.success());
    assert!(w.path("Source/old").is_dir());
    assert_eq!(
        git(&w.path("Source/old"), &["remote", "get-url", "origin"]),
        "git@github.com:me/old.git"
    );
}
