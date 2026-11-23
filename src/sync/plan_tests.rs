use super::plan::*;
use std::path::PathBuf;

fn b(name: &str, ahead: u32, behind: u32) -> Branch {
    Branch {
        repo: PathBuf::from("/r/app"),
        name: name.into(),
        upstream: Some(format!("origin/{name}")),
        ahead,
        behind,
        checked_out: false,
        dirty: false,
        default_branch: false,
        elsewhere: None,
    }
}

fn repo() -> PathBuf {
    PathBuf::from("/r/app")
}

#[test]
fn a_branch_in_step_with_its_upstream_has_no_step() {
    assert!(plan(&[b("a", 0, 0)]).is_empty());
}

#[test]
fn ahead_is_a_push_and_behind_is_a_pull() {
    let steps = plan(&[b("a", 2, 0), b("b", 0, 3)]);
    assert_eq!(
        steps[0],
        Step::Push {
            repo: repo(),
            branch: "a".into(),
            count: 2
        }
    );
    assert_eq!(
        steps[1],
        Step::Pull {
            repo: repo(),
            branch: "b".into(),
            upstream: "origin/b".into(),
            count: 3,
            checked_out: false
        }
    );
}

#[test]
fn three_pushes_and_three_pulls_are_six_steps_in_input_order() {
    let all = [
        b("p1", 1, 0),
        b("l1", 0, 1),
        b("p2", 4, 0),
        b("l2", 0, 2),
        b("p3", 1, 0),
        b("l3", 0, 5),
    ];
    let steps = plan(&all);
    assert_eq!(steps.len(), 6);
    let pushes = steps
        .iter()
        .filter(|s| matches!(s, Step::Push { .. }))
        .count();
    let pulls = steps
        .iter()
        .filter(|s| matches!(s, Step::Pull { .. }))
        .count();
    assert_eq!((pushes, pulls), (3, 3));
}

#[test]
fn ahead_and_behind_is_diverged_never_pushed_or_pulled() {
    assert_eq!(
        plan(&[b("a", 1, 2)]),
        [Step::Diverged {
            repo: repo(),
            branch: "a".into(),
            ahead: 1,
            behind: 2
        }]
    );
}

#[test]
fn no_upstream_is_skipped_with_the_reason() {
    let mut x = b("a", 3, 0);
    x.upstream = None;
    assert_eq!(
        plan(&[x]),
        [Step::Skip {
            repo: repo(),
            branch: "a".into(),
            why: "no upstream".into()
        }]
    );
}

#[test]
fn the_default_branch_is_never_pushed_but_is_pulled() {
    let mut p = b("main", 2, 0);
    p.default_branch = true;
    assert_eq!(
        plan(&[p]),
        [Step::Skip {
            repo: repo(),
            branch: "main".into(),
            why: "default branch is never pushed".into()
        }]
    );
    let mut l = b("main", 0, 2);
    l.default_branch = true;
    assert!(matches!(plan(&[l])[0], Step::Pull { .. }));
}

#[test]
fn a_dirty_checked_out_branch_is_not_pulled_but_a_dirty_push_is_fine() {
    let mut l = b("a", 0, 2);
    l.checked_out = true;
    l.dirty = true;
    assert_eq!(
        plan(&[l]),
        [Step::Skip {
            repo: repo(),
            branch: "a".into(),
            why: "dirty tree".into()
        }]
    );
    let mut p = b("a", 2, 0);
    p.checked_out = true;
    p.dirty = true;
    assert!(matches!(plan(&[p])[0], Step::Push { .. }));
}

#[test]
fn a_pull_remembers_whether_the_branch_is_checked_out() {
    let mut l = b("a", 0, 1);
    l.checked_out = true;
    assert!(matches!(
        plan(&[l])[0],
        Step::Pull {
            checked_out: true,
            ..
        }
    ));
}

#[test]
fn each_step_has_a_one_line_report() {
    assert_eq!(plan(&[b("a", 2, 0)])[0].line(), "/r/app a: push 2");
    assert_eq!(plan(&[b("b", 0, 3)])[0].line(), "/r/app b: pull 3");
    assert_eq!(
        plan(&[b("c", 1, 2)])[0].line(),
        "/r/app c: diverged 1 ahead, 2 behind"
    );
    let mut x = b("d", 1, 0);
    x.upstream = None;
    assert_eq!(plan(&[x])[0].line(), "/r/app d: skip, no upstream");
}

#[test]
fn pull_only_keeps_pulls_and_divergences_and_drops_pushes_and_skips() {
    let mut no_upstream = b("c", 0, 0);
    no_upstream.upstream = None;
    let steps = plan(&[b("a", 2, 0), b("b", 0, 3), b("d", 1, 1), no_upstream]);
    assert_eq!(steps.len(), 4);
    let kept = pull_only(steps);
    assert_eq!(kept.len(), 2, "{kept:?}");
    assert!(matches!(kept[0], Step::Pull { .. }), "{kept:?}");
    assert!(matches!(kept[1], Step::Diverged { .. }), "{kept:?}");
}

#[test]
fn a_pull_of_a_branch_checked_out_in_another_worktree_is_skipped_with_its_path() {
    let mut other = b("a", 0, 3);
    other.elsewhere = Some(PathBuf::from("/r/other"));
    let steps = plan(&[other]);
    assert_eq!(steps.len(), 1);
    let line = steps[0].line();
    assert!(
        matches!(steps[0], Step::Skip { .. }) && line.contains("checked out in /r/other"),
        "{line}"
    );
}
