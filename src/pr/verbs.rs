//! `actrepo pr view|list|retarget|close`: the pull-request verbs besides create and merge.
use actautoma::forge::Forge;
use actautoma::pulls::{PrFilter, Pulls};
use anyhow::Result;
use std::io::Write;

/// Title, branches, author, state and mergeability of one PR (the current branch's when `number` is `None`).
pub fn view(f: &(impl Forge + Pulls), number: Option<u64>, out: &mut impl Write) -> Result<()> {
    let pr = f.view_pr(number)?;
    let state = state_word(&f.pr_state(number)?);
    writeln!(
        out,
        "#{} {}\n{} -> {} by {}, {state}, {}",
        pr.number, pr.title, pr.head, pr.base, pr.author, pr.mergeable
    )?;
    Ok(())
}

fn state_word(state: &actautoma::pulls::PrState) -> String {
    use actautoma::pulls::PrState::*;
    match state {
        Open => "OPEN".into(),
        Closed => "CLOSED".into(),
        Merged => "MERGED".into(),
        Other(word) => word.clone(),
    }
}

/// One line per PR; an empty answer is said, not left blank.
pub fn list(f: &impl Pulls, filter: PrFilter, out: &mut impl Write) -> Result<()> {
    let prs = f.list_prs(filter)?;
    if prs.is_empty() {
        writeln!(out, "no pull requests")?;
    }
    for p in prs {
        writeln!(
            out,
            "#{} {} {} -> {} {} {} {}",
            p.number,
            state_word(&p.state),
            p.head,
            p.base,
            p.author,
            p.mergeable,
            p.title
        )?;
    }
    Ok(())
}

pub fn retarget(f: &impl Pulls, number: u64, base: &str, out: &mut impl Write) -> Result<()> {
    f.retarget_pr(number, base)?;
    writeln!(out, "#{number} now targets {base}")?;
    Ok(())
}

pub fn close(
    f: &impl Pulls,
    number: u64,
    comment: Option<&str>,
    out: &mut impl Write,
) -> Result<()> {
    f.close_pr(number, comment)?;
    writeln!(out, "#{number} closed")?;
    Ok(())
}
