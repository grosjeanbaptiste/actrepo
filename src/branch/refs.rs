//! The references to the old branch name in a repository's files: which lines
//! name the branch (and can be rewritten) and which only use the word.

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Byte offsets of the whole-word occurrences of `word` in `line` (as `git grep -w`).
fn occurrences(line: &str, word: &str) -> Vec<usize> {
    line.match_indices(word)
        .filter(|(i, _)| {
            let before = line[..*i].chars().next_back();
            let after = line[i + word.len()..].chars().next();
            !before.is_some_and(is_word) && !after.is_some_and(is_word)
        })
        .map(|(i, _)| i)
        .collect()
}

/// Whole-word match, as `git grep -w`: word characters are letters, digits, `_`.
pub fn mentions(line: &str, word: &str) -> bool {
    !occurrences(line, word).is_empty()
}

const QUOTES: &[char] = &['`', '\'', '"'];

/// The occurrence at `at` reads as the branch: `origin/NAME`, `heads/NAME`,
/// `branch NAME`, `NAME branch`. A bare word in prose ("master thesis") does not.
fn names_the_branch(line: &str, at: usize, word: &str) -> bool {
    let before = line[..at].trim_end_matches(QUOTES);
    let after = line[at + word.len()..].trim_start_matches(QUOTES);
    line[..at].ends_with("origin/")
        || line[..at].ends_with("heads/")
        || before.ends_with("branch ")
        || after.starts_with(" branch")
}

/// `line` with the branch `from` renamed `to`, or `None` when it names `from`
/// nowhere that can be rewritten safely. Under `.github/` (workflows, templates)
/// every whole-word `from` is the branch; elsewhere only the forms above.
pub fn rewritten(line: &str, from: &str, to: &str, in_github: bool) -> Option<String> {
    let at: Vec<usize> = occurrences(line, from)
        .into_iter()
        .filter(|&i| in_github || names_the_branch(line, i, from))
        .collect();
    if at.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(line.len());
    let mut last = 0;
    for i in at {
        out.push_str(&line[last..i]);
        out.push_str(to);
        last = i + from.len();
    }
    out.push_str(&line[last..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    include!("refs_tests.rs");
}
