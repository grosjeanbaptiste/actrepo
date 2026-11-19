use super::*;

fn machine() -> Machine {
    Machine {
        home: "/h".into(),
        state: "/h/machine".into(),
        dotfiles: "/h/dotfiles".into(),
        bin_dir: "/h/.cargo/bin".into(),
    }
}

fn registered() -> Vec<Entry> {
    vec![Entry { name: "acteses".into(), path: "/h/Source/acteses".into() }]
}

fn req<'a>(new: &'a str, dir: Option<&'a Path>) -> Request<'a> {
    Request { slug: "me/acteses", new, dir, local_only: false, into: None }
}

fn everywhere(_: &Path) -> bool {
    true
}

fn build_ok(r: &Request, exists: &dyn Fn(&Path) -> bool) -> Planned {
    build(r, &registered(), &machine(), exists).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_folder_comes_from_the_registry_when_no_dir_is_given() {
    let p = build_ok(&req("actos", None), &everywhere).plan;
    assert_eq!(p.old_dir, PathBuf::from("/h/Source/acteses"));
    assert_eq!(p.new_dir(), PathBuf::from("/h/Source/actos"));
    assert_eq!((p.owner.as_str(), p.old.as_str(), p.new.as_str()), ("me", "acteses", "actos"));
}

#[test]
fn an_explicit_dir_wins_over_the_registry() {
    let dir = Path::new("/elsewhere/acteses");
    let p = build_ok(&req("actos", Some(dir)), &everywhere).plan;
    assert_eq!(p.old_dir, dir);
}

#[test]
fn an_unregistered_repository_without_dir_is_refused_never_taken_from_the_cwd() {
    let r = Request { slug: "me/other", new: "x", dir: None, local_only: false, into: None };
    let err = match build(&r, &registered(), &machine(), &everywhere) {
        Err(e) => e,
        Ok(_) => panic!("an unregistered repository was accepted"),
    };
    assert!(err.to_string().contains("--dir"), "{err}");
}

#[test]
fn a_bad_slug_or_new_name_is_refused() {
    for slug in ["acteses", "/acteses", "me/"] {
        let r = Request { slug, new: "x", dir: None, local_only: false, into: None };
        assert!(build(&r, &registered(), &machine(), &everywhere).is_err(), "{slug}");
    }
    for new in ["", ".", "..", "a/b", "acteses"] {
        assert!(build(&req(new, None), &registered(), &machine(), &everywhere).is_err(), "{new:?}");
    }
}

#[test]
fn a_registered_new_name_is_refused() {
    let mut taken = registered();
    taken.push(Entry { name: "actos".into(), path: "/h/Source/actos".into() });
    assert!(build(&req("actos", None), &taken, &machine(), &everywhere).is_err());
}

#[test]
fn absent_reference_folders_are_skipped_with_a_warning() {
    let only_state = |p: &Path| p == Path::new("/h/machine");
    let planned = build_ok(&req("actos", None), &only_state);
    assert_eq!(
        planned.plan.roots,
        [PathBuf::from("/h/Source/actos"), PathBuf::from("/h/machine")]
    );
    assert!(planned.plan.children.is_empty());
    assert_eq!(planned.warnings.len(), 4, "{:?}", planned.warnings);
    assert!(planned.warnings.iter().any(|w| w.contains("/h/dotfiles")));
}

#[test]
fn every_present_folder_is_a_root_or_a_children_folder() {
    let p = build_ok(&req("actos", None), &everywhere);
    assert_eq!(
        p.plan.roots,
        [
            PathBuf::from("/h/Source/actos"),
            PathBuf::from("/h/machine"),
            PathBuf::from("/h/dotfiles"),
            PathBuf::from("/h/claude-memory"),
        ]
    );
    assert_eq!(
        p.plan.children,
        [PathBuf::from("/h/claude-memory/projects"), PathBuf::from("/h/.claude/projects")]
    );
    assert!(p.warnings.is_empty());
}

#[test]
fn local_only_means_the_platform_already_has_the_new_name() {
    let r = Request { local_only: true, ..req("actos", None) };
    assert!(!build_ok(&r, &everywhere).plan.on_platform);
    assert!(build_ok(&req("actos", None), &everywhere).plan.on_platform);
}

#[test]
fn into_puts_the_renamed_folder_under_another_parent() {
    let r = Request { slug: "me/acteses", new: "actos", dir: None, local_only: false, into: Some(Path::new("/h/Source/openacte")) };
    let p = build_ok(&r, &everywhere).plan;
    assert_eq!(p.new_dir(), PathBuf::from("/h/Source/openacte/actos"));
    assert!(p.roots.contains(&PathBuf::from("/h/Source/openacte/actos")));
}
