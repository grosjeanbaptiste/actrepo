//! What linking the skills of the registered repositories into one folder would do.
use crate::registry::file::Entry;
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Link {
        name: String,
        target: PathBuf,
        link: PathBuf,
    },
    Linked {
        name: String,
    },
    Conflict {
        name: String,
        reason: String,
    },
}

/// Skills a repository ships: each folder of `.claude/skills` holding a `SKILL.md`.
fn shipped(entry: &Entry) -> Result<Vec<(String, PathBuf)>> {
    let dir = entry.path.join(".claude/skills");
    let rd = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("cannot read {}", dir.display())),
    };
    let mut skills = Vec::new();
    for item in rd {
        let path = item
            .with_context(|| format!("cannot read {}", dir.display()))?
            .path();
        if let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned())
            && path.join("SKILL.md").is_file()
        {
            skills.push((name, path));
        }
    }
    Ok(skills)
}

pub fn plan(entries: &[Entry], skills_dir: &Path) -> Result<Vec<Step>> {
    let mut by_name: BTreeMap<String, Vec<(&str, PathBuf)>> = BTreeMap::new();
    for e in entries {
        for (name, path) in shipped(e)? {
            by_name.entry(name).or_default().push((&e.name, path));
        }
    }
    let mut steps = Vec::new();
    for (name, owners) in by_name {
        if owners.len() > 1 {
            let names: Vec<&str> = owners.iter().map(|(r, _)| *r).collect();
            steps.push(Step::Conflict {
                name,
                reason: format!("shipped by several repositories: {}", names.join(", ")),
            });
            continue;
        }
        let target = owners[0].1.clone();
        let link = skills_dir.join(&name);
        steps.push(match fs::symlink_metadata(&link) {
            Err(e) if e.kind() == ErrorKind::NotFound => Step::Link { name, target, link },
            Err(e) => return Err(e).with_context(|| format!("cannot read {}", link.display())),
            Ok(m) if !m.is_symlink() => Step::Conflict {
                name,
                reason: format!("{} exists and is not a link", link.display()),
            },
            Ok(_) => {
                let now = fs::read_link(&link)
                    .with_context(|| format!("cannot read link {}", link.display()))?;
                if now == target {
                    Step::Linked { name }
                } else {
                    Step::Conflict {
                        name,
                        reason: format!("{} points to {}", link.display(), now.display()),
                    }
                }
            }
        });
    }
    Ok(steps)
}

impl Step {
    fn name(&self) -> &str {
        match self {
            Step::Link { name, .. } | Step::Linked { name } | Step::Conflict { name, .. } => name,
        }
    }
}

/// Keep the steps of the named skills; a name no repository ships is an error.
pub fn only(steps: Vec<Step>, names: &[String]) -> Result<Vec<Step>> {
    if let Some(n) = names
        .iter()
        .find(|n| steps.iter().all(|s| s.name() != n.as_str()))
    {
        bail!("no registered repository ships a skill named {n}");
    }
    Ok(steps
        .into_iter()
        .filter(|s| names.iter().any(|n| n == s.name()))
        .collect())
}

/// Create every `Link`; conflicts are reported together after the rest is done.
pub fn apply(steps: &[Step]) -> Result<()> {
    let mut conflicts = Vec::new();
    for step in steps {
        match step {
            Step::Link { target, link, .. } => {
                if let Some(parent) = link.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("cannot create {}", parent.display()))?;
                }
                symlink(target, link).with_context(|| format!("cannot link {}", link.display()))?;
            }
            Step::Conflict { name, reason } => conflicts.push(format!("{name}: {reason}")),
            Step::Linked { .. } => {}
        }
    }
    if !conflicts.is_empty() {
        bail!("skills not linked:\n{}", conflicts.join("\n"));
    }
    Ok(())
}
