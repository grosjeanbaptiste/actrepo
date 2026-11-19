//! repo as a library: the registry, status and worktree readers other tools call instead of parsing `repo` output.
pub mod actinstall;
pub mod audit;
pub mod branch;
pub mod delegate;
pub mod deploy;
pub mod device;
pub mod doctor;
#[cfg(test)]
pub mod fakes;
pub mod install;
pub mod license;
pub mod naming;
pub mod need;
pub mod orchestrate;
pub mod pipeline;
pub mod pr;
pub mod push;
pub mod registry;
pub mod release;
pub mod rename;
pub mod root;
pub mod run;
pub mod scaffold;
pub mod workspace;
pub mod worktree;
