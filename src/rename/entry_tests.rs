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

#[test]
fn a_rename_that_failed_after_moving_the_folder_still_moves_the_row() {
    let rows = [e("a", "/s/a"), e("acteses", "/s/acteses")];
    let got = reconciled(&rows, "acteses", "actos", Path::new("/s/actos"), &|p| {
        p == Path::new("/s/actos")
    })
    .unwrap_or_else(|| panic!("the folder moved, the row must follow"));
    assert_eq!(got, [e("a", "/s/a"), e("actos", "/s/actos")]);
}

#[test]
fn a_rename_that_failed_before_moving_the_folder_leaves_the_row() {
    let rows = [e("acteses", "/s/acteses")];
    let none = reconciled(&rows, "acteses", "actos", Path::new("/s/actos"), &|p| {
        p == Path::new("/s/acteses")
    });
    assert!(none.is_none());
}

#[test]
fn a_folder_present_under_both_names_leaves_the_row() {
    let rows = [e("acteses", "/s/acteses")];
    let none = reconciled(&rows, "acteses", "actos", Path::new("/s/actos"), &|_| true);
    assert!(none.is_none());
}
