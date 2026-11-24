//! Reads the branches of a repository into the plan's input.
use super::plan::Branch;
use anyhow::{Context, Result, bail};
use std::path::Path;

/// What git says about a repository, before it is turned into branches.
pub struct Facts<'a> {
    /// `git for-each-ref --format='%(refname:short)%09%(upstream:short)%09%(upstream:track,nobracket)%09%(worktreepath)' refs/heads`
    pub refs: &'a str,
    /// The branch checked out in this working tree, empty when detached.
    pub current: &'a str,
    pub dirty: bool,
    /// The default branch of the remote, empty when unknown.
    pub default_branch: &'a str,
}

/// One branch per line; a line or a track word git would not write is an error, never a guess.
pub fn parse(repo: &Path, facts: &Facts) -> Result<Vec<Branch>> {
    facts.refs.lines().map(|l| branch(repo, facts, l)).collect()
}

fn branch(repo: &Path, facts: &Facts, line: &str) -> Result<Branch> {
    let [name, upstream, track, worktree] = line.split('\t').collect::<Vec<_>>()[..] else {
        bail!("git for-each-ref: unexpected line {line:?}");
    };
    let gone = track == "gone";
    let (ahead, behind) = if gone {
        (0, 0)
    } else {
        counts(track).map_err(|e| anyhow::anyhow!("branch {name}: {e:#}"))?
    };
    Ok(Branch {
        repo: repo.to_path_buf(),
        name: name.into(),
        upstream: (!upstream.is_empty() && !gone).then(|| upstream.into()),
        ahead,
        behind,
        checked_out: name == facts.current,
        dirty: facts.dirty && name == facts.current,
        default_branch: !facts.default_branch.is_empty() && name == facts.default_branch,
        elsewhere: (!worktree.is_empty() && Path::new(worktree) != repo).then(|| worktree.into()),
    })
}

/// `""`, `ahead 2`, `behind 3` or `ahead 1, behind 4`.
fn counts(track: &str) -> Result<(u32, u32)> {
    let (mut ahead, mut behind) = (0, 0);
    for word in track.split(", ").filter(|w| !w.is_empty()) {
        match word.split_once(' ') {
            Some(("ahead", n)) => ahead = n.parse().with_context(|| format!("track {track:?}"))?,
            Some(("behind", n)) => {
                behind = n.parse().with_context(|| format!("track {track:?}"))?
            }
            _ => bail!("unexpected track {track:?}"),
        }
    }
    Ok((ahead, behind))
}
