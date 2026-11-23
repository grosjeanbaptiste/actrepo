use super::exec::execute;
use super::plan::Step;
use crate::scaffold::setup::Exec;
use anyhow::Result;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct Fake {
    calls: RefCell<Vec<String>>,
    fail_on: Option<&'static str>,
}

impl Exec for Fake {
    fn run(&self, prog: &str, args: &[&str], cwd: Option<&Path>) -> Result<String> {
        let line = format!(
            "{} {prog} {}",
            cwd.map(|c| c.display().to_string()).unwrap_or_default(),
            args.join(" ")
        );
        self.calls.borrow_mut().push(line.clone());
        if self.fail_on.is_some_and(|f| line.contains(f)) {
            anyhow::bail!("rejected");
        }
        Ok(String::new())
    }
}

fn repo(n: &str) -> PathBuf {
    PathBuf::from(format!("/r/{n}"))
}

fn push(n: &str, branch: &str) -> Step {
    Step::Push {
        repo: repo(n),
        branch: branch.into(),
        count: 1,
    }
}

fn pull(n: &str, branch: &str, checked_out: bool) -> Step {
    Step::Pull {
        repo: repo(n),
        branch: branch.into(),
        upstream: format!("origin/{branch}"),
        count: 2,
        checked_out,
    }
}

fn text(out: Vec<u8>) -> String {
    String::from_utf8(out).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_push_pushes_that_branch_from_its_repo() {
    let f = Fake::default();
    let mut out = vec![];
    execute(&[push("a", "feat")], &f, &mut out).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(f.calls.borrow().as_slice(), ["/r/a git push origin feat"]);
    assert_eq!(text(out), "ok   /r/a feat: push 1\n");
}

#[test]
fn a_pull_of_the_checked_out_branch_merges_fast_forward_only() {
    let f = Fake::default();
    execute(&[pull("a", "feat", true)], &f, &mut vec![]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        f.calls.borrow().as_slice(),
        ["/r/a git merge --ff-only origin/feat"]
    );
}

#[test]
fn a_pull_of_another_branch_fetches_it_fast_forward_only() {
    let f = Fake::default();
    execute(&[pull("a", "feat", false)], &f, &mut vec![]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        f.calls.borrow().as_slice(),
        ["/r/a git fetch origin feat:feat"]
    );
}

#[test]
fn three_pushes_and_three_pulls_are_all_attempted_in_order() {
    let f = Fake::default();
    let steps = [
        push("a", "p1"),
        pull("b", "l1", false),
        push("c", "p2"),
        pull("d", "l2", true),
        push("e", "p3"),
        pull("f", "l3", false),
    ];
    execute(&steps, &f, &mut vec![]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(f.calls.borrow().len(), 6);
    assert!(f.calls.borrow()[0].contains("/r/a") && f.calls.borrow()[5].contains("/r/f"));
}

#[test]
fn one_failure_never_stops_the_others_and_the_run_fails_naming_the_count() {
    let f = Fake {
        fail_on: Some("p2"),
        ..Fake::default()
    };
    let steps = [push("a", "p1"), push("c", "p2"), pull("d", "l2", false)];
    let mut out = vec![];
    let e = execute(&steps, &f, &mut out)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert_eq!(f.calls.borrow().len(), 3);
    assert!(e.contains("1 of 3"), "{e}");
    let t = text(out);
    assert!(t.contains("ok   /r/a p1: push 1"), "{t}");
    assert!(t.contains("FAIL /r/c p2: push 1: rejected"), "{t}");
    assert!(t.contains("ok   /r/d l2: pull 2"), "{t}");
}

#[test]
fn diverged_and_skipped_steps_run_nothing_and_make_the_run_fail() {
    let f = Fake::default();
    let steps = [
        Step::Diverged {
            repo: repo("a"),
            branch: "x".into(),
            ahead: 1,
            behind: 1,
        },
        Step::Skip {
            repo: repo("b"),
            branch: "y".into(),
            why: "dirty tree".into(),
        },
        push("c", "z"),
    ];
    let mut out = vec![];
    let e = execute(&steps, &f, &mut out)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert_eq!(f.calls.borrow().as_slice(), ["/r/c git push origin z"]);
    assert!(e.contains("2 of 3"), "{e}");
    let t = text(out);
    assert!(t.contains("TODO /r/a x: diverged 1 ahead, 1 behind"), "{t}");
    assert!(t.contains("TODO /r/b y: skip, dirty tree"), "{t}");
}

#[test]
fn an_empty_plan_succeeds_and_says_so() {
    let mut out = vec![];
    execute(&[], &Fake::default(), &mut out).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(text(out), "nothing to sync\n");
}
