//! `actrepo sync`: settle every branch that is ahead (push) or behind (pull) of its upstream.
pub mod exec;
pub mod plan;

#[cfg(test)]
mod exec_tests;
#[cfg(test)]
mod plan_tests;
