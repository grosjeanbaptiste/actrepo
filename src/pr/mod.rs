//! `actrepo pr create|merge|view|list|retarget|close` (port of dotfiles `project pr`).
pub mod after;
pub mod command;
pub mod create;
pub mod merge;
pub mod own;
pub mod sync;
pub mod verbs;
#[cfg(test)]
mod verbs_tests;
