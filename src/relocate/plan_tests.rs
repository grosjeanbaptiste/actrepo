use super::*;

fn e(name: &str, path: &str) -> Entry {
    Entry { name: name.into(), path: path.into() }
}

fn here(present: &'static [&'static str]) -> impl Fn(&Path) -> bool {
    move |p| present.iter().any(|q| Path::new(q) == p)
}

#[test]
fn a_registered_name_moves_under_the_new_parent_keeping_its_folder_name() {
    let rows = [e("a", "/s/a"), e("actorpus", "/s/actorpus")];
    let mv = build(&rows, "actorpus", Path::new("/s/x"), &here(&["/s/actorpus"])).unwrap();
    assert_eq!(mv, Move { name: "actorpus".into(), from: "/s/actorpus".into(), to: "/s/x/actorpus".into() });
}

#[test]
fn a_folder_path_picks_the_row_by_path() {
    let rows = [e("corpus", "/s/a/corpus"), e("corpus", "/s/b/corpus")];
    let mv = build(&rows, "/s/b/corpus", Path::new("/t"), &here(&["/s/b/corpus"])).unwrap();
    assert_eq!(mv.to, Path::new("/t/corpus"));
    assert_eq!(mv.from, Path::new("/s/b/corpus"));
}

#[test]
fn a_name_held_by_several_rows_is_refused_and_names_every_path() {
    let rows = [e("corpus", "/s/a/corpus"), e("corpus", "/s/b/corpus")];
    let err = build(&rows, "corpus", Path::new("/t"), &here(&["/s/a/corpus", "/s/b/corpus"])).unwrap_err();
    let text = format!("{err:#}");
    assert!(text.contains("/s/a/corpus") && text.contains("/s/b/corpus"), "{text}");
}

#[test]
fn an_unregistered_name_is_refused() {
    let err = build(&[e("a", "/s/a")], "b", Path::new("/t"), &here(&[])).unwrap_err();
    assert!(format!("{err:#}").contains("not registered"), "{err:#}");
}

#[test]
fn a_registered_folder_that_is_absent_is_refused() {
    let err = build(&[e("a", "/s/a")], "a", Path::new("/t"), &here(&[])).unwrap_err();
    assert!(format!("{err:#}").contains("absent"), "{err:#}");
}

#[test]
fn a_destination_that_exists_is_refused() {
    let err = build(&[e("a", "/s/a")], "a", Path::new("/t"), &here(&["/s/a", "/t/a"])).unwrap_err();
    assert!(format!("{err:#}").contains("already exists"), "{err:#}");
}

#[test]
fn moving_a_folder_into_its_own_parent_is_refused() {
    let err = build(&[e("a", "/s/a")], "a", Path::new("/s"), &here(&["/s/a"])).unwrap_err();
    assert!(format!("{err:#}").contains("already in"), "{err:#}");
}

#[test]
fn moving_a_folder_into_itself_is_refused() {
    let err = build(&[e("a", "/s/a")], "a", Path::new("/s/a/sub"), &here(&["/s/a"])).unwrap_err();
    assert!(format!("{err:#}").contains("inside itself"), "{err:#}");
}

#[test]
fn only_the_moved_row_changes_path() {
    let rows = [e("corpus", "/s/a/corpus"), e("corpus", "/s/b/corpus"), e("z", "/s/z")];
    let mv = Move { name: "corpus".into(), from: "/s/b/corpus".into(), to: "/t/corpus".into() };
    let got = moved(&rows, &mv).unwrap();
    assert_eq!(got, [e("corpus", "/s/a/corpus"), e("corpus", "/t/corpus"), e("z", "/s/z")]);
}

#[test]
fn no_row_at_the_old_path_is_none() {
    let mv = Move { name: "a".into(), from: "/s/a".into(), to: "/t/a".into() };
    assert!(moved(&[e("a", "/other/a")], &mv).is_none());
}
