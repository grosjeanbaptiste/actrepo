//! `actrepo completions zsh|bash`: the shell side of `actrepo __complete`, with an fzf search when fzf is installed.
use anyhow::{Result, bail};

const ZSH: &str = r#"#compdef actrepo
_repo() {
  local out
  out=$(actrepo __complete "${(@)words[2,CURRENT]}" 2>/dev/null) || { _message "actrepo __complete failed: run it by hand for the reason"; return 1; }
  local -a lines ids disp
  lines=("${(@f)out}")
  [[ -n "$out" ]] || return 1
  if (( $#lines > 1 )) && (( $+commands[fzf] )) && [[ -t 1 ]]; then
    local pick
    pick=$(print -rl -- $lines | fzf --height=40% --reverse --prompt='repo> ') || { zle reset-prompt 2>/dev/null; return 1; }
    lines=("$pick")
    zle reset-prompt 2>/dev/null
  fi
  ids=(${lines%%$'\t'*})
  disp=(${lines//$'\t'/  })
  compadd -l -d disp -a ids
}
compdef _repo actrepo
"#;

const BASH: &str = r#"_repo() {
  local out line
  out=$(actrepo __complete "${COMP_WORDS[@]:1:COMP_CWORD}" 2>/dev/null) || { echo "actrepo __complete failed: run it by hand for the reason" >&2; return 1; }
  [ -n "$out" ] || return 1
  if [ "$(printf '%s\n' "$out" | wc -l)" -gt 1 ] && command -v fzf > /dev/null && [ -t 1 ]; then
    out=$(printf '%s\n' "$out" | fzf --height=40% --reverse --prompt='repo> ') || return 1
  fi
  COMPREPLY=()
  while IFS= read -r line; do COMPREPLY+=("${line%%$'\t'*}"); done <<< "$out"
}
complete -F _repo actrepo
"#;

pub fn script(shell: &str) -> Result<&'static str> {
    match shell {
        "zsh" => Ok(ZSH),
        "bash" => Ok(BASH),
        other => bail!("unsupported shell {other}; expected zsh or bash"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_shell_registers_actrepo_only() {
        for (shell, form) in [
            ("zsh", "compdef _repo actrepo\n"),
            ("bash", "complete -F _repo actrepo\n"),
        ] {
            let s = script(shell).unwrap_or_else(|e| panic!("{e}"));
            assert!(s.contains(form), "{shell}: {form}");
            assert!(!s.contains("actrepo repo"), "{shell} still registers repo");
        }
    }

    #[test]
    fn each_shell_calls_complete_and_offers_fzf() {
        for shell in ["zsh", "bash"] {
            let s = script(shell).unwrap_or_else(|e| panic!("{e}"));
            assert!(
                s.contains("actrepo __complete") && s.contains("fzf"),
                "{shell}"
            );
        }
    }

    #[test]
    fn a_failing_complete_is_reported_not_swallowed() {
        for shell in ["zsh", "bash"] {
            let s = script(shell).unwrap_or_else(|e| panic!("{e}"));
            assert!(s.contains("actrepo __complete failed"), "{shell}");
        }
    }

    #[test]
    fn another_shell_is_refused_by_name() {
        let e = script("tcsh")
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert_eq!(e, "unsupported shell tcsh; expected zsh or bash");
    }
}
