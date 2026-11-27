//! `actrepo move` end to end in a temp HOME: the folder, its linked worktrees and its registry row.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = ok(Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .current_dir(dir)
        .output());
    assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

struct World {
    home: tempfile::TempDir,
}

impl World {
    /// `$HOME/Source/proj` with a linked worktree `$HOME/wt`, and a registry naming the repo.
    fn new() -> Self {
        let home = ok(tempfile::tempdir());
        let h = home.path();
        let proj = h.join("Source/proj");
        ok(fs::create_dir_all(&proj));
        ok(fs::create_dir_all(h.join("Source/dest")));
        ok(fs::create_dir_all(h.join("machine")));
        git(&proj, &["init", "-q", "-b", "main"]);
        git(&proj, &["commit", "-q", "--allow-empty", "-m", "first"]);
        git(&proj, &["worktree", "add", "-q", h.join("wt").to_str().unwrap_or_default(), "-b", "side"]);
        ok(fs::write(h.join("machine/projects.txt"), format!("proj|{}\nother|/x/other\n", proj.display())));
        World { home }
    }
    fn path(&self, p: &str) -> PathBuf {
        self.home.path().join(p)
    }
    fn registry(&self) -> String {
        ok(fs::read_to_string(self.path("machine/projects.txt")))
    }
    fn repo(&self, args: &[&str]) -> Output {
        ok(Command::new(env!("CARGO_BIN_EXE_actrepo"))
            .args(args)
            .env("HOME", self.home.path())
            .env("DOTFILES_STATE_ROOT", self.path("machine"))
            .current_dir(self.home.path())
            .output())
    }
}

fn text(o: &[u8]) -> String {
    String::from_utf8_lossy(o).to_string()
}

#[test]
fn a_move_relocates_the_folder_repairs_its_worktrees_and_follows_in_the_registry() {
    let w = World::new();
    let dest = w.path("Source/dest");
    let o = w.repo(&["move", "proj", dest.to_str().unwrap_or_default()]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(!w.path("Source/proj").exists() && w.path("Source/dest/proj").is_dir());
    assert_eq!(git(&w.path("wt"), &["rev-parse", "--abbrev-ref", "HEAD"]), "side");
    assert!(git(&w.path("Source/dest/proj"), &["worktree", "list"]).contains("wt"));
    assert_eq!(w.registry(), format!("proj|{}\nother|/x/other\n", w.path("Source/dest/proj").display()));
}

#[test]
fn a_dry_run_prints_the_move_and_changes_nothing() {
    let w = World::new();
    let before = w.registry();
    let dest = w.path("Source/dest");
    let o = w.repo(&["move", "proj", dest.to_str().unwrap_or_default(), "--dry-run"]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(text(&o.stdout).contains("dry run"), "{}", text(&o.stdout));
    assert!(w.path("Source/proj").is_dir() && !w.path("Source/dest/proj").exists());
    assert_eq!(w.registry(), before);
}

#[test]
fn a_refused_move_changes_nothing_and_says_why() {
    let w = World::new();
    ok(fs::create_dir_all(w.path("Source/dest/proj")));
    let before = w.registry();
    let dest = w.path("Source/dest");
    let o = w.repo(&["move", "proj", dest.to_str().unwrap_or_default()]);
    assert!(!o.status.success());
    assert!(text(&o.stderr).contains("already exists"), "{}", text(&o.stderr));
    assert!(w.path("Source/proj").is_dir());
    assert_eq!(w.registry(), before);
}
