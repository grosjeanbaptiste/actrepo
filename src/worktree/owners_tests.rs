use super::*;

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

struct Scripted(&'static str);
impl Exec for Scripted {
    fn run(&self, prog: &str, args: &[&str], _: Option<&Path>) -> Result<String> {
        assert_eq!((prog, args), ("actsessions", &["list", "--json"][..]));
        Ok(self.0.to_string())
    }
}

struct Failing;
impl Exec for Failing {
    fn run(&self, _: &str, _: &[&str], _: Option<&Path>) -> Result<String> {
        anyhow::bail!("actsessions not found: repo needs it")
    }
}

fn wt(path: &str) -> Worktree {
    Worktree {
        path: path.into(),
        branch: Some("b".into()),
        main: false,
        detached: false,
        prunable: false,
        owners: None,
    }
}

const SESSIONS: &str = r#"[
  {"id":"a1","agent":"claude","status":"active","cwd":"/w/repo"},
  {"id":"a2","agent":"claude","status":"active","cwd":"/w/repo/src"},
  {"id":"a3","agent":"claude","status":"idle","cwd":"/w/repo"},
  {"id":"a4","agent":"claude","status":"active","cwd":null},
  {"id":"a5","agent":"codex","status":"active","cwd":"/w/repo-feat"}
]"#;

#[test]
fn only_active_sessions_with_a_known_cwd_are_candidates() {
    let ids: Vec<_> = ok(live(&Scripted(SESSIONS))).into_iter().map(|s| s.id).collect();
    assert_eq!(ids, ["a1", "a2", "a5"]);
}

#[test]
fn a_session_belongs_to_the_worktree_containing_its_cwd_not_a_name_prefix() {
    let mut all = vec![wt("/w/repo"), wt("/w/repo-feat")];
    assign(&mut all, &ok(live(&Scripted(SESSIONS))));
    let ids = |w: &Worktree| -> Vec<String> {
        w.owners.iter().flatten().map(|o| o.id.clone()).collect()
    };
    assert_eq!(ids(&all[0]), ["a1", "a2"]);
    assert_eq!(ids(&all[1]), ["a5"]);
}

#[test]
fn the_deepest_worktree_wins_when_one_is_inside_another() {
    let mut all = vec![wt("/w/repo"), wt("/w/repo/src")];
    assign(&mut all, &ok(live(&Scripted(SESSIONS))));
    assert_eq!(all[0].owners.iter().flatten().count(), 1);
    assert_eq!(all[1].owners.iter().flatten().map(|o| o.id.as_str()).collect::<Vec<_>>(), ["a2"]);
}

#[test]
fn a_worktree_nobody_works_in_has_an_empty_owner_list_not_none() {
    let mut all = vec![wt("/elsewhere")];
    assign(&mut all, &ok(live(&Scripted(SESSIONS))));
    assert_eq!(all[0].owners, Some(vec![]));
}

#[test]
fn an_unreadable_answer_is_an_error_naming_the_cause() {
    let err = live(&Scripted("not json")).err().map(|e| format!("{e:#}")).unwrap_or_default();
    assert!(err.contains("actsessions list --json"), "{err}");
}

#[test]
fn a_missing_actsessions_is_an_error_not_an_empty_list() {
    assert!(live(&Failing).is_err());
}
