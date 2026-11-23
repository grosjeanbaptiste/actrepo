//! Reads the branches of a repository into the plan's input. Red stub.
use super::plan::Branch;
use anyhow::Result;
use std::path::Path;

/// What git says about a repository, before it is turned into branches.
pub struct Facts<'a> {
    /// `git for-each-ref --format='%(refname:short)%09%(upstream:short)%09%(upstream:track,nobracket)' refs/heads`
    pub refs: &'a str,
    /// The branch checked out in this working tree, empty when detached.
    pub current: &'a str,
    pub dirty: bool,
    /// The default branch of the remote, empty when unknown.
    pub default_branch: &'a str,
}

pub fn parse(_repo: &Path, _facts: &Facts) -> Result<Vec<Branch>> {
    unimplemented!("red")
}
