//! Guard: a test that runs `actrepo rename` must pin `DOTFILES_REPO_ROOT`.
//! `rename` rewrites the old name under that root; inherited from the shell it is the real
//! dotfiles checkout (66 files and every worktree rewritten on 2026-11-27).
use std::fs;
use std::io;
use std::path::Path;

/// Test files that invoke the rename verb but never set the root it rewrites.
fn unisolated(dir: &Path) -> io::Result<Vec<String>> {
    let mut bad = vec![];
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "rs") || path.ends_with("rename_isolation.rs") {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        if text.contains("\"rename\"") && !text.contains("DOTFILES_REPO_ROOT") {
            bad.push(path.display().to_string());
        }
    }
    Ok(bad)
}

#[test]
fn every_test_running_rename_pins_the_dotfiles_root() -> io::Result<()> {
    let bad = unisolated(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"))?;
    assert!(bad.is_empty(), "set DOTFILES_REPO_ROOT in: {bad:?}");
    Ok(())
}

#[test]
fn the_guard_flags_a_file_that_forgets_the_root() -> io::Result<()> {
    let dir = std::env::temp_dir().join(format!("rename-guard-{}", std::process::id()));
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("leaky.rs"),
        "run(&[\"rename\", \"me/old\", \"new\"]);",
    )?;
    let bad = unisolated(&dir);
    fs::remove_dir_all(&dir)?;
    assert_eq!(bad?.len(), 1);
    Ok(())
}
