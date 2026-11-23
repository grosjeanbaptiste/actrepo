use super::read::*;
use std::path::Path;

fn facts<'a>(refs: &'a str, current: &'a str, dirty: bool) -> Facts<'a> {
    Facts {
        refs,
        current,
        dirty,
        default_branch: "main",
    }
}

fn parse_ok(f: &Facts) -> Vec<super::plan::Branch> {
    parse(Path::new("/r/app"), f).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_track_words_become_ahead_and_behind_counts() {
    let raw = "a\torigin/a\t\nb\torigin/b\tahead 2\nc\torigin/c\tbehind 3\nd\torigin/d\tahead 1, behind 4\n";
    let bs = parse_ok(&facts(raw, "", false));
    let counts: Vec<(u32, u32)> = bs.iter().map(|b| (b.ahead, b.behind)).collect();
    assert_eq!(counts, [(0, 0), (2, 0), (0, 3), (1, 4)]);
    assert_eq!(bs[1].upstream.as_deref(), Some("origin/b"));
    assert_eq!(bs[1].repo, Path::new("/r/app"));
}

#[test]
fn no_upstream_and_a_gone_upstream_have_none() {
    let bs = parse_ok(&facts("a\t\t\nb\torigin/b\tgone\n", "", false));
    assert_eq!(
        (bs[0].upstream.clone(), bs[1].upstream.clone()),
        (None, None)
    );
}

#[test]
fn the_current_branch_is_checked_out_and_carries_the_dirty_flag() {
    let bs = parse_ok(&facts("a\torigin/a\t\nb\torigin/b\t\n", "b", true));
    assert_eq!((bs[0].checked_out, bs[0].dirty), (false, false));
    assert_eq!((bs[1].checked_out, bs[1].dirty), (true, true));
}

#[test]
fn the_remote_default_branch_is_marked() {
    let bs = parse_ok(&facts("main\torigin/main\t\nx\torigin/x\t\n", "", false));
    assert_eq!((bs[0].default_branch, bs[1].default_branch), (true, false));
}

#[test]
fn an_unknown_default_branch_marks_none() {
    let f = Facts {
        refs: "main\torigin/main\t\n",
        current: "",
        dirty: false,
        default_branch: "",
    };
    assert!(!parse_ok(&f)[0].default_branch);
}

#[test]
fn a_track_word_git_may_have_invented_is_an_error_naming_it() {
    let e = parse(
        Path::new("/r"),
        &facts("a\torigin/a\tsideways 2\n", "", false),
    )
    .err()
    .map(|e| e.to_string())
    .unwrap_or_default();
    assert!(e.contains("sideways 2") && e.contains("a"), "{e}");
}

#[test]
fn a_line_of_the_wrong_shape_is_an_error() {
    assert!(parse(Path::new("/r"), &facts("only-one-field\n", "", false)).is_err());
}

#[test]
fn no_refs_is_no_branches() {
    assert!(parse_ok(&facts("", "", false)).is_empty());
}
