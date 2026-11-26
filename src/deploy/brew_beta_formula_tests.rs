use super::*;

const STABLE: &str = "class ActebleLinux < Formula\n  # Prebuilt Linux bundle.\n  desc \"Desktop client for the Acteble platform\"\n  homepage \"https://acteble.com\"\n  url \"https://example.test/v0.0.0/acteble-linux-x86_64.tar.gz\"\n  version \"0.0.0\"\n  sha256 \"abc\" # release-bumped\n\n  depends_on :linux\n\n  def install\n    bin.install_symlink libexec/\"acteble\"\n  end\nend\n";

fn beta() -> String {
    beta_formula(STABLE).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_beta_class_is_the_versioned_formula_name_of_homebrew() {
    // Homebrew derives the class of `acteble-linux@beta` as `ActebleLinuxATBeta`.
    assert!(beta().starts_with("class ActebleLinuxATBeta < Formula\n"));
}

#[test]
fn the_beta_conflicts_with_the_stable_formula() {
    assert!(beta().contains(
        "\n  conflicts_with \"acteble-linux\", because: \"both install the acteble binary\"\n"
    ));
}

#[test]
fn the_conflict_comes_right_after_the_class_line() {
    let lines: Vec<String> = beta().lines().map(String::from).collect();
    assert!(lines[1].starts_with("  conflicts_with \"acteble-linux\""));
}

#[test]
fn the_beta_says_it_is_a_beta() {
    assert!(beta().contains("  desc \"Desktop client for the Acteble platform (beta)\"\n"));
}

#[test]
fn the_rest_of_the_formula_is_untouched() {
    let beta = beta();
    for kept in [
        "  # Prebuilt Linux bundle.\n",
        "  version \"0.0.0\"\n",
        "  sha256 \"abc\" # release-bumped\n",
        "  depends_on :linux\n",
        "    bin.install_symlink libexec/\"acteble\"\n",
    ] {
        assert!(beta.contains(kept), "lost: {kept}");
    }
    assert!(beta.ends_with("  end\nend\n"));
}

#[test]
fn a_text_that_is_not_a_formula_is_refused_by_name() {
    let e = beta_formula("cask \"x\" do\nend\n")
        .err()
        .map(|e| e.to_string());
    assert_eq!(
        e.as_deref(),
        Some("not a stable formula: no `class <Name> < Formula` line.")
    );
}

#[test]
fn a_beta_formula_is_not_made_into_a_beta_again() {
    let e = beta_formula("class ActebleLinuxATBeta < Formula\nend\n")
        .err()
        .map(|e| e.to_string());
    assert_eq!(
        e.as_deref(),
        Some("already a beta formula: ActebleLinuxATBeta.")
    );
}

#[test]
fn the_beta_file_is_named_after_the_stable_one() {
    assert_eq!(beta_file_name("acteble-linux"), "acteble-linux@beta.rb");
}
