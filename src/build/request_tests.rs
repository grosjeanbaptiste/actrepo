use super::*;

fn words(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

#[test]
fn only_name_then_build_is_ours() {
    assert!(is_build_request(&words("thesis build")));
    assert!(is_build_request(&words("thesis build --clean main.tex")));
    assert!(!is_build_request(&words("thesis run")));
    assert!(!is_build_request(&words("status thesis")));
    assert!(!is_build_request(&words("thesis")));
}

#[test]
fn clean_and_a_source_file_are_optional() {
    let r = parse(&words("thesis build")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        (r.name.as_str(), r.opts.clean, r.opts.main),
        ("thesis", false, None)
    );
    let r = parse(&words("thesis build --clean paper.tex")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        (r.opts.clean, r.opts.main.as_deref()),
        (true, Some("paper.tex"))
    );
}

#[test]
fn a_second_file_or_an_unknown_flag_is_refused() {
    assert!(parse(&words("thesis build a.tex b.tex")).is_err());
    assert!(parse(&words("thesis build --fast")).is_err());
    assert!(parse(&words("thesis")).is_err());
}
