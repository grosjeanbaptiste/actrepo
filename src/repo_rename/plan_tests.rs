use super::*;

fn env() -> Env {
    Env {
        home: "/h".into(),
        state: "/h/machine".into(),
        dotfiles: "/h/dotfiles".into(),
        cargo_bin: "/h/.cargo/bin".into(),
    }
}

fn opts(local_only: bool) -> Opts {
    Opts {
        new: "new".into(),
        dir: "/h/Source/old".into(),
        local_only,
        dry_run: false,
    }
}

fn all_present(_: &Path) -> bool {
    true
}

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn slug_reads_every_remote_shape() {
    for url in [
        "git@github.com:me/old.git",
        "ssh://git@github.com/me/old.git",
        "https://github.com/me/old",
        "https://github.com/me/old.git/\n",
    ] {
        assert_eq!(
            ok(slug(url)),
            ("me".to_string(), "old".to_string()),
            "{url}"
        );
    }
}

#[test]
fn slug_refuses_what_it_cannot_read() {
    for url in ["", "old", "/tmp/old.git", "git@github.com:old"] {
        assert!(slug(url).is_err(), "{url:?} was accepted");
    }
}

#[test]
fn the_plan_moves_to_a_sibling_and_lists_the_machine_references() {
    let mut warn = Vec::new();
    let p = ok(plan(
        &opts(false),
        &env(),
        "git@github.com:me/old.git",
        &all_present,
        &mut warn,
    ));
    assert_eq!(
        (p.owner.as_str(), p.old.as_str(), p.new.as_str()),
        ("me", "old", "new")
    );
    assert_eq!(p.new_dir(), PathBuf::from("/h/Source/new"));
    assert_eq!(
        p.roots,
        [
            "/h/Source/new",
            "/h/machine",
            "/h/dotfiles",
            "/h/claude-memory"
        ]
        .map(PathBuf::from)
    );
    assert_eq!(
        p.children,
        ["/h/claude-memory/projects", "/h/.claude/projects"].map(PathBuf::from)
    );
    assert!(p.on_platform && warn.is_empty());
}

#[test]
fn local_only_skips_the_platform() {
    let mut warn = Vec::new();
    let p = ok(plan(
        &opts(true),
        &env(),
        "git@github.com:me/old.git",
        &all_present,
        &mut warn,
    ));
    assert!(!p.on_platform);
}

#[test]
fn an_absent_reference_folder_is_named_not_dropped_quietly() {
    let mut warn = Vec::new();
    let present = |p: &Path| p != Path::new("/h/dotfiles") && p != Path::new("/h/.claude/projects");
    let p = ok(plan(
        &opts(false),
        &env(),
        "git@github.com:me/old.git",
        &present,
        &mut warn,
    ));
    assert!(!p.roots.contains(&PathBuf::from("/h/dotfiles")));
    assert_eq!(p.children, [PathBuf::from("/h/claude-memory/projects")]);
    let warn = String::from_utf8_lossy(&warn).into_owned();
    assert!(
        warn.contains("skipping /h/dotfiles (absent)")
            && warn.contains("skipping /h/.claude/projects (absent)"),
        "{warn}"
    );
}

#[test]
fn an_unreadable_origin_is_an_error() {
    let mut warn = Vec::new();
    assert!(plan(&opts(false), &env(), "nonsense", &all_present, &mut warn).is_err());
}
