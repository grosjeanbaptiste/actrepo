use super::*;

fn e(name: &str, path: &str) -> Entry {
    Entry { name: name.into(), path: path.into() }
}

#[test]
fn only_the_named_row_changes() {
    let rows = [e("a", "/s/a"), e("acteses", "/s/acteses"), e("b", "/s/b")];
    let got = renamed(&rows, "acteses", "actos", Path::new("/s/actos"))
        .unwrap_or_else(|| panic!("the row acteses was not found"));
    assert_eq!(got, [e("a", "/s/a"), e("actos", "/s/actos"), e("b", "/s/b")]);
}

#[test]
fn no_row_with_that_name_is_none() {
    assert!(renamed(&[e("a", "/s/a")], "acteses", "actos", Path::new("/s/actos")).is_none());
}
