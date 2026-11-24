use super::verbs::{close, list, retarget, view};
use crate::fakes::FakeForge;
use actautoma::forge::Mergeable;
use actautoma::pulls::{PrFilter, PrState, PrSummary};

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn summary(number: u64, state: PrState) -> PrSummary {
    PrSummary {
        number,
        title: "Add x".into(),
        head: "feat".into(),
        base: "main".into(),
        author: "bob".into(),
        state,
        mergeable: Mergeable::Conflicting,
    }
}

fn text(out: Vec<u8>) -> String {
    String::from_utf8(out).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn view_prints_title_branches_author_state_and_mergeability() {
    let f = FakeForge::pr("Add x", Mergeable::Mergeable).in_state(PrState::Merged);
    let mut out = vec![];
    ok(view(&f, Some(8), &mut out));
    assert_eq!(
        text(out),
        "#8 Add x\nfeature -> main by ana, MERGED, MERGEABLE\n"
    );
    assert_eq!(f.log(), ["view 8", "state 8"]);
}

#[test]
fn view_without_a_number_asks_for_the_pr_of_the_current_branch() {
    let f = FakeForge::pr("Add x", Mergeable::Mergeable);
    ok(view(&f, None, &mut vec![]));
    assert_eq!(f.log(), ["view", "state"]);
}

#[test]
fn view_failures_reach_the_caller_and_print_nothing() {
    for op in ["view", "state"] {
        let f = FakeForge::pr("x", Mergeable::Mergeable).failing(op);
        let mut out = vec![];
        let e = view(&f, Some(8), &mut out)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(e.contains(&format!("{op} failed")), "{e}");
        assert!(out.is_empty());
    }
}

#[test]
fn list_prints_one_line_per_pr() {
    let f =
        FakeForge::default().listing(vec![summary(3, PrState::Open), summary(4, PrState::Closed)]);
    let mut out = vec![];
    ok(list(&f, PrFilter::All, &mut out));
    assert_eq!(
        text(out),
        "#3 OPEN feat -> main bob CONFLICTING Add x\n#4 CLOSED feat -> main bob CONFLICTING Add x\n"
    );
    assert_eq!(f.log(), ["list All"]);
}

#[test]
fn an_empty_list_says_so() {
    let mut out = vec![];
    ok(list(&FakeForge::default(), PrFilter::Open, &mut out));
    assert_eq!(text(out), "no pull requests\n");
}

#[test]
fn a_list_failure_reaches_the_caller() {
    let f = FakeForge::default().failing("list");
    assert!(list(&f, PrFilter::Open, &mut vec![]).is_err());
}

#[test]
fn retarget_edits_the_base_and_says_so() {
    let f = FakeForge::default();
    let mut out = vec![];
    ok(retarget(&f, 8, "main", &mut out));
    assert_eq!(f.log(), ["retarget 8 main"]);
    assert_eq!(text(out), "#8 now targets main\n");
}

#[test]
fn a_retarget_failure_reaches_the_caller_and_prints_nothing() {
    let f = FakeForge::default().failing("retarget");
    let mut out = vec![];
    assert!(retarget(&f, 8, "main", &mut out).is_err());
    assert!(out.is_empty());
}

#[test]
fn close_passes_the_comment_and_says_so() {
    let f = FakeForge::default();
    let mut out = vec![];
    ok(close(&f, 8, Some("in #11"), &mut out));
    ok(close(&f, 9, None, &mut out));
    assert_eq!(f.log(), ["close 8 in #11", "close 9 -"]);
    assert_eq!(text(out), "#8 closed\n#9 closed\n");
}

#[test]
fn a_close_failure_reaches_the_caller_and_prints_nothing() {
    let f = FakeForge::default().failing("close");
    let mut out = vec![];
    assert!(close(&f, 8, None, &mut out).is_err());
    assert!(out.is_empty());
}
