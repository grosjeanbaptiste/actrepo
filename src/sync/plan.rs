//! The pure plan: what to do for each branch, from its ahead/behind counts. Red stub.
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

pub fn plan(_branches: &[Branch]) -> Vec<Step> {
    unimplemented!("red")
}

impl Step {
    pub fn line(&self) -> String {
        unimplemented!("red")
    }
}
