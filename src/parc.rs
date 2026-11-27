//! Keep `actparc`'s volume indexes true after a folder moved: `actparc move OLD NEW`
//! rewrites a path prefix, a rescan of a volume takes hours. A missing or failing
//! `actparc` is a warning, never a failure of the move that already happened.
use std::path::{Path, PathBuf};
use std::process::Command;

/// The folder `actrepo move ARGS` will move and where it lands (`DIR --into PARENT`, the
/// folder keeps its name), as absolute real paths. `None` when nothing moves (`--dry-run`)
/// or the arguments are not ones actautoma accepts: it reports those itself.
pub fn move_target(args: &[String], cwd: &Path) -> Option<(PathBuf, PathBuf)> {
    if args.iter().any(|a| a == "--dry-run") {
        return None;
    }
    let dir = args.first().filter(|a| !a.starts_with('-'))?;
    let into = args
        .iter()
        .position(|a| a == "--into")
        .and_then(|i| args.get(i + 1))?;
    let old = cwd.join(dir).canonicalize().ok()?;
    let parent = cwd.join(into).canonicalize().ok()?;
    let new = parent.join(old.file_name()?);
    Some((old, new))
}

/// Tell `actparc` (`ACTREPO_ACTPARC` names another binary, for tests) and say why on
/// stderr when it could not be done.
pub fn follow_and_warn(old: &Path, new: &Path) {
    let program = std::env::var("ACTREPO_ACTPARC").unwrap_or_else(|_| "actparc".into());
    let abs = |p: &Path| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    if let Some(w) = follow(&program, &abs(old), &abs(new)) {
        eprintln!("warn: {w}");
    }
}

/// Run `program move OLD NEW`. `Some(warning)` when it could not be done.
pub fn follow(program: &str, old: &Path, new: &Path) -> Option<String> {
    let out = match Command::new(program).arg("move").arg(old).arg(new).output() {
        Ok(out) => out,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => return Some(format!("cannot run {program}: {e}")),
    };
    if out.status.success() {
        return None;
    }
    Some(format!(
        "{program} move {} {} failed, the index keeps the old path until the next scan: {}",
        old.display(),
        new.display(),
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}

#[cfg(test)]
mod tests {
    include!("parc_tests.rs");
}
