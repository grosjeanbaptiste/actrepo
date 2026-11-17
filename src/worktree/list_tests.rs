use super::*;

#[test]
fn a_branch_a_detached_head_and_a_gone_folder_are_told_apart() {
    let out = "worktree /r\nHEAD aaa\nbranch refs/heads/main\n\n\
               worktree /r-d\nHEAD bbb\ndetached\n\n\
               worktree /r-gone\nHEAD ccc\nbranch refs/heads/old\nprunable gitdir file points to non-existent location\n";
    let w = |path: &str, branch: Option<&str>, main, detached, prunable| Worktree {
        path: path.into(),
        branch: branch.map(Into::into),
        main,
        detached,
        prunable,
    };
    assert_eq!(
        parse(out),
        vec![
            w("/r", Some("main"), true, false, false),
            w("/r-d", None, false, true, false),
            w("/r-gone", Some("old"), false, false, true),
        ]
    );
}

#[test]
fn nothing_listed_is_an_empty_list() {
    assert_eq!(parse(""), vec![]);
}

#[test]
fn the_text_marks_detached_and_gone() {
    let all = parse("worktree /r\nbranch refs/heads/main\n\nworktree /d\ndetached\nprunable x\n");
    assert_eq!(text(&all), "/r  main\n/d  (detached)  (folder gone)\n");
}
