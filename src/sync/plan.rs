//! The pure plan: what to do for each branch, from its ahead/behind counts. Nothing here touches git.
use std::path::PathBuf;

/// One local branch and how it stands against its upstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub repo: PathBuf,
    pub name: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub checked_out: bool,
    pub dirty: bool,
    pub default_branch: bool,
    /// The other working tree this branch is checked out in, when it is not this one.
    pub elsewhere: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Push {
        repo: PathBuf,
        branch: String,
        count: u32,
    },
    Pull {
        repo: PathBuf,
        branch: String,
        upstream: String,
        count: u32,
        checked_out: bool,
    },
    Diverged {
        repo: PathBuf,
        branch: String,
        ahead: u32,
        behind: u32,
    },
    Skip {
        repo: PathBuf,
        branch: String,
        why: String,
    },
}

/// One step per branch that is not in step with its upstream, in input order.
pub fn plan(branches: &[Branch]) -> Vec<Step> {
    branches.iter().filter_map(step).collect()
}

/// Keeps what only reads from the remote: pulls, and divergences (reported, never acted on).
/// A session start uses it so that nothing is pushed through a gate.
pub fn pull_only(steps: Vec<Step>) -> Vec<Step> {
    steps
        .into_iter()
        .filter(|s| matches!(s, Step::Pull { .. } | Step::Diverged { .. }))
        .collect()
}

fn step(b: &Branch) -> Option<Step> {
    let (repo, branch) = (b.repo.clone(), b.name.clone());
    let skip = |why: &str| {
        Some(Step::Skip {
            repo: repo.clone(),
            branch: branch.clone(),
            why: why.into(),
        })
    };
    let Some(upstream) = b.upstream.clone() else {
        return skip("no upstream");
    };
    match (b.ahead, b.behind) {
        (0, 0) => None,
        (ahead, behind) if ahead > 0 && behind > 0 => Some(Step::Diverged {
            repo,
            branch,
            ahead,
            behind,
        }),
        (_, 0) if b.default_branch => skip("default branch is never pushed"),
        (count, 0) => Some(Step::Push {
            repo,
            branch,
            count,
        }),
        _ if b.checked_out && b.dirty => skip("dirty tree"),
        _ if b.elsewhere.is_some() => skip(&format!(
            "checked out in {}",
            b.elsewhere
                .as_deref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        )),
        (_, count) => Some(Step::Pull {
            repo,
            branch,
            upstream,
            count,
            checked_out: b.checked_out,
        }),
    }
}

impl Step {
    /// `<repo> <branch>: <what>`, the same words in the plan and in the report.
    pub fn line(&self) -> String {
        match self {
            Step::Push {
                repo,
                branch,
                count,
            } => format!("{} {branch}: push {count}", repo.display()),
            Step::Pull {
                repo,
                branch,
                count,
                ..
            } => format!("{} {branch}: pull {count}", repo.display()),
            Step::Diverged {
                repo,
                branch,
                ahead,
                behind,
            } => {
                format!(
                    "{} {branch}: diverged {ahead} ahead, {behind} behind",
                    repo.display()
                )
            }
            Step::Skip { repo, branch, why } => format!("{} {branch}: skip, {why}", repo.display()),
        }
    }
}
