//! What a rename touches on this machine, decided from the environment only.
use actautoma::rename::Plan;
use anyhow::{Result, bail};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Opts {
    pub new: String,
    pub dir: PathBuf,
    pub local_only: bool,
    pub dry_run: bool,
}

/// The places that hold references to a repository.
#[derive(Debug)]
pub struct Env {
    pub home: PathBuf,
    pub state: PathBuf,
    pub dotfiles: PathBuf,
    pub cargo_bin: PathBuf,
}

/// `(owner, name)` of a GitHub remote URL (ssh, scp-like or https).
pub fn slug(url: &str) -> Result<(String, String)> {
    if !url.contains("://") && !url.contains('@') {
        bail!("cannot read OWNER/NAME from the origin URL {url:?}: not a remote URL");
    }
    let path = url.trim().trim_end_matches('/').trim_end_matches(".git");
    let tail = path.rsplit([':', '/']).take(2).collect::<Vec<_>>();
    match tail[..] {
        [name, owner] if !name.is_empty() && !owner.is_empty() && !owner.contains('@') => {
            Ok((owner.to_string(), name.to_string()))
        }
        _ => bail!("cannot read OWNER/NAME from the origin URL {url:?}"),
    }
}

/// Existing folders only; an absent one is named on `warn`, never dropped quietly.
fn existing(
    dirs: Vec<PathBuf>,
    is_dir: &dyn Fn(&Path) -> bool,
    warn: &mut impl Write,
) -> Result<Vec<PathBuf>> {
    let mut kept = Vec::new();
    for dir in dirs {
        if is_dir(&dir) {
            kept.push(dir);
        } else {
            writeln!(warn, "repo rename: skipping {} (absent)", dir.display())?;
        }
    }
    Ok(kept)
}

pub fn plan(
    opts: &Opts,
    env: &Env,
    origin: &str,
    is_dir: &dyn Fn(&Path) -> bool,
    warn: &mut impl Write,
) -> Result<Plan> {
    let (owner, old) = slug(origin)?;
    let new_dir = opts.dir.with_file_name(&opts.new);
    let memory = env.home.join("claude-memory");
    let mut roots = vec![env.state.clone(), env.dotfiles.clone(), memory.clone()];
    roots = existing(roots, is_dir, warn)?;
    roots.insert(0, new_dir); // the moved folder holds references too, and does not exist yet
    let children = vec![memory.join("projects"), env.home.join(".claude/projects")];
    Ok(Plan {
        owner,
        old,
        new: opts.new.clone(),
        old_dir: opts.dir.clone(),
        home: env.home.clone(),
        bin_dir: env.cargo_bin.clone(),
        roots,
        children: existing(children, is_dir, warn)?,
        on_platform: !opts.local_only,
    })
}

#[cfg(test)]
mod tests {
    include!("plan_tests.rs");
}
