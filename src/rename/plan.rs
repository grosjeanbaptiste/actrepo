//! What `repo rename` will change, decided before anything is touched.
use crate::registry::file::{Entry, find};
use actautoma::rename::Plan;
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

/// The machine's own folders: where the old name may be written.
pub struct Machine {
    pub home: PathBuf,
    pub state: PathBuf,
    pub dotfiles: PathBuf,
    pub bin_dir: PathBuf,
}

pub struct Request<'a> {
    pub slug: &'a str,
    pub new: &'a str,
    pub dir: Option<&'a Path>,
    pub local_only: bool,
}

pub struct Planned {
    pub plan: Plan,
    /// Folders skipped because they are absent: said out loud, never dropped silently.
    pub warnings: Vec<String>,
}

/// `OWNER/OLD`, both parts present.
fn split(slug: &str) -> Result<(&str, &str)> {
    match slug.split_once('/') {
        Some((owner, old)) if !owner.is_empty() && !old.is_empty() && !old.contains('/') => {
            Ok((owner, old))
        }
        _ => bail!("expected OWNER/NAME, got {slug:?}"),
    }
}

/// A new name is one path component, different from the old one.
fn check_new(new: &str, old: &str) -> Result<()> {
    if new.is_empty() || new == "." || new == ".." || new.contains('/') {
        bail!("{new:?} is not a repository name (one path component)");
    }
    if new == old {
        bail!("the new name is the old one: {old}");
    }
    Ok(())
}

/// Keep the folders that exist; name each absent one in `warnings`.
fn present(
    dirs: &[PathBuf],
    exists: &dyn Fn(&Path) -> bool,
    warnings: &mut Vec<String>,
) -> Vec<PathBuf> {
    let mut kept = vec![];
    for d in dirs {
        if exists(d) {
            kept.push(d.clone());
        } else {
            warnings.push(format!("skipping {} (absent)", d.display()));
        }
    }
    kept
}

pub fn build(
    req: &Request,
    registry: &[Entry],
    machine: &Machine,
    exists: &dyn Fn(&Path) -> bool,
) -> Result<Planned> {
    let (owner, old) = split(req.slug)?;
    check_new(req.new, old)?;
    if find(registry, req.new).is_some() {
        bail!("{} is already registered", req.new);
    }
    let old_dir = match (req.dir, find(registry, old)) {
        (Some(dir), _) => dir.to_path_buf(),
        (None, Some(dir)) => dir,
        (None, None) => bail!("{old} is not registered: give its folder with --dir"),
    };
    let new_dir = old_dir.with_file_name(req.new);
    let memory = machine.home.join("claude-memory");
    let mut warnings = vec![];
    let mut roots = vec![new_dir];
    roots.extend(present(
        &[
            machine.state.clone(),
            machine.dotfiles.clone(),
            memory.clone(),
        ],
        exists,
        &mut warnings,
    ));
    let children = present(
        &[
            memory.join("projects"),
            machine.home.join(".claude/projects"),
        ],
        exists,
        &mut warnings,
    );
    let plan = Plan {
        owner: owner.into(),
        old: old.into(),
        new: req.new.into(),
        old_dir,
        home: machine.home.clone(),
        roots,
        children,
        bin_dir: machine.bin_dir.clone(),
        on_platform: !req.local_only,
    };
    Ok(Planned { plan, warnings })
}

#[cfg(test)]
mod tests {
    include!("plan_tests.rs");
}
