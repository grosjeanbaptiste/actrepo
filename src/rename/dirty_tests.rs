use super::*;
use anyhow::anyhow;

fn roots() -> Vec<PathBuf> {
    vec![PathBuf::from("/a"), PathBuf::from("/b")]
}

#[test]
fn clean_roots_pass() {
    assert!(ensure_clean(&roots(), &|_| Ok(vec![]), false).is_ok());
}

#[test]
fn a_dirty_tree_is_refused_and_named() {
    let dirty = |p: &Path| Ok(if p == Path::new("/b") { vec![PathBuf::from("/b/.worktrees/w")] } else { vec![] });
    let err = ensure_clean(&roots(), &dirty, false).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(err.contains("/b/.worktrees/w") && err.contains("--allow-dirty"), "{err}");
}

#[test]
fn allow_dirty_skips_the_check() {
    let boom = |_: &Path| Err(anyhow!("must not run"));
    assert!(ensure_clean(&roots(), &boom, true).is_ok());
}

#[test]
fn a_failing_check_propagates() {
    let err = ensure_clean(&roots(), &|_| Err(anyhow!("git broke")), false).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(err.contains("git broke"), "{err}");
}

fn init(dir: &Path) {
    for args in [vec!["init", "-q"], vec!["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "i"]] {
        assert!(Command::new("git").arg("-C").arg(dir).args(args).status().is_ok_and(|s| s.success()));
    }
}

#[test]
fn real_git_sees_a_dirty_file_and_a_clean_tree() -> Result<()> {
    let dir = std::env::temp_dir().join(format!("dirty-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    init(&dir);
    assert!(git_dirty_trees(&dir)?.is_empty());
    std::fs::write(dir.join("f"), "x")?;
    let dirty = git_dirty_trees(&dir)?;
    std::fs::remove_dir_all(&dir)?;
    assert_eq!(dirty.len(), 1);
    Ok(())
}

#[test]
fn a_folder_that_is_not_git_is_not_dirty() -> Result<()> {
    let dir = std::env::temp_dir().join(format!("nogit-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let dirty = git_dirty_trees(&dir);
    std::fs::remove_dir_all(&dir)?;
    assert!(dirty?.is_empty());
    Ok(())
}
