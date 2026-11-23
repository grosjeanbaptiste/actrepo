//! `actrepo pr view|list|retarget|close`: the pull-request verbs besides create and merge.
use actautoma::forge::Forge;
use actautoma::pulls::{PrFilter, Pulls};
use anyhow::Result;
use std::io::Write;

pub fn view(_f: &(impl Forge + Pulls), _number: Option<u64>, _out: &mut impl Write) -> Result<()> {
    unimplemented!("red")
}

pub fn list(_f: &impl Pulls, _filter: PrFilter, _out: &mut impl Write) -> Result<()> {
    unimplemented!("red")
}

pub fn retarget(_f: &impl Pulls, _number: u64, _base: &str, _out: &mut impl Write) -> Result<()> {
    unimplemented!("red")
}

pub fn close(
    _f: &impl Pulls,
    _number: u64,
    _comment: Option<&str>,
    _out: &mut impl Write,
) -> Result<()> {
    unimplemented!("red")
}
