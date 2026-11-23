//! Runs a plan: every step is attempted, each outcome is reported. Red stub.
use super::plan::Step;
use crate::scaffold::setup::Exec;
use anyhow::Result;
use std::io::Write;

pub fn execute(_steps: &[Step], _exec: &impl Exec, _out: &mut impl Write) -> Result<()> {
    unimplemented!("red")
}
