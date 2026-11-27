use super::*;

#[test]
fn the_main_worktree_is_skipped_and_every_linked_one_listed() {
    let porcelain = "worktree /s/proj\nHEAD abc\nbranch refs/heads/main\n\n\
                     worktree /s/wt one\nHEAD def\nbranch refs/heads/side\n\n\
                     worktree /t/wt2\nHEAD 123\ndetached\n";
    assert_eq!(linked(porcelain), [PathBuf::from("/s/wt one"), PathBuf::from("/t/wt2")]);
}

#[test]
fn a_repository_without_linked_worktrees_lists_none() {
    assert!(linked("worktree /s/proj\nHEAD abc\nbranch refs/heads/main\n").is_empty());
    assert!(linked("").is_empty());
}
