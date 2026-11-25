# Roadmap

## Directive (owner, 2026-11-23): `actrepo` and `actautoma` handle every git and gh workflow

Every workflow an agent or the owner runs with `git` or `gh` goes through
`actrepo` or `actautoma`; raw `git` / `gh` stays only for what they lack, and
each such use names the missing verb so it can be added here.

Proposed split (mine, to confirm with the owner): `actrepo` holds the single
verbs on a repository: commit, push, pull, branches, worktrees and pull requests; `actautoma`
holds the workflows that chain several of them (ship, merge a stack, report a
problem upstream). Both stay thin over `git` and `gh`, with the strictness of
the rest of the doctrine (no silent failure, a refusal names what and why).

### Verbs missing today (found by shipping 30 PRs on 2026-11-22/23 with raw commands)

`actrepo` has `pr create`, `pr merge`, `push` (a repository and its nested ones), `worktree list|prune`, `status`; it has no `commit` and no `pull`.
Used by hand instead:

| Raw command | Wanted verb | Owner |
|---|---|---|
| `git add PATH… && git commit -m …` (by pathspec, never a peer's file) | `actrepo commit --path P… -m …` (refuses a path that is not listed, keeps the session's attribution trailer, runs the pre-commit gate, never `--no-verify`) | actrepo |
| `git pull`, `git fetch && git merge --ff-only` | `actrepo pull` (fast-forward only; refuses when incoming files overlap a dirty file, naming it; never switches the branch of a checkout) | actrepo |
| `git push -u origin B` of one named branch from a worktree, the gate output kept, the result checked afterwards | `actrepo push --branch B` (today `actrepo push` takes a repository and its nested ones, clean tree only); the result is read back with `branch remote`, not from an exit code | actrepo |
| `git worktree add .worktrees/X -b BRANCH origin/main` | `actrepo worktree add BRANCH [--from origin/main]` | actrepo |
| `git fetch`, `git ls-remote --heads origin B` | `actrepo branch remote B` (is it pushed, at which sha) | actrepo |
| `git branch -d B`, `git worktree remove --force` (generated files only) | `actrepo worktree remove`, `actrepo branch delete --merged` | actrepo |
| `gh pr view N --json state,mergeable,baseRefName` | `actrepo pr view N` | actrepo |
| `gh pr list --state open --json …` | `actrepo pr list` | actrepo |
| `gh pr edit N --base main` | `actrepo pr retarget N --base main` | actrepo |
| `gh pr close N --comment …` | `actrepo pr close N [--comment]` | actrepo |
| `gh pr ready N` (`actrepo pr merge` fails with "still a draft" until then; found 2026-11-24 merging actparc #11, #12) | `actrepo pr ready N`, and `pr merge` naming `pr ready` in its draft error | actrepo |
| `git merge-tree --write-tree BASE HEAD` | `actrepo pr check N` (conflict dry run) | actrepo |
| merge `origin/main` in a detached worktree, fix, `git push origin HEAD:B` | `actautoma resolve-conflicts N` (worktree, merge, stop on conflict, push) | actautoma |
| push, `actrepo pr create` with the last subject, `actrepo pr merge --yes` (`ship.sh`) | `actautoma ship` (gates first, merge only where authorised) | actautoma |
| stacked PRs squash-merged: retarget the tip, dry run, merge, close the contained ones | `actautoma merge-stack` | actautoma |
| `gh issue create`, `gh search issues`, `gh repo fork` for a problem found upstream | `actautoma report-upstream` | actautoma |
| `actrepo pr merge` then restore a peer's checkout it switched to `main` | `actrepo pr merge --keep-checkout` (never switch a checkout that is not the merger's) | actrepo |

Known defects seen while shipping (to fix, test first): `actrepo pr merge`
switched peers' checkouts to `main`; it reports "PR merged, but the local
update is incomplete" when the worktree holds untracked generated files and
leaves the worktree behind (nothing says which file blocks it).

### Bulk case (owner, 2026-11-23): several pushes and several pulls at once

When 3 branches are ahead and 3 are behind (across repositories and worktrees), the tools settle all of them without a human sequencing each command.

- **`actrepo sync [--dry-run]`** (verb, actrepo; built 2026-11-23, see CONTEXT.md; the dirty-overlap refinement and the heavy-lane serialising of pushes remain): for every registered repository and worktree, compute ahead/behind per branch against its upstream, then act:
  - ahead only: push (pre-push gate runs; one slot at a time through the heavy lane; never the default branch of a non-`openacte` repository, never a force);
  - behind only: pull fast-forward only; a checkout that is not the caller's is never switched;
  - diverged: stop for that branch, report both counts and the first conflicting file; `actautoma resolve-conflicts` takes over;
  - dirty tree overlapping incoming files: skip, name the file.
- **Plan then act.** `--dry-run` prints the plan (one line per branch: repo, branch, `push n` / `pull n` / `diverged a/b` / `skip why`); the real run executes the same plan.
- **One failure never hides the others.** Every item is attempted, each outcome is read back (`branch remote`), one report line per item, exit non-zero when any item failed or was skipped.
- **`actautoma sync-all`** (workflow): `actrepo sync`, then for each diverged branch `resolve-conflicts`, then `ship` where merging is authorised. Own ADR before code.
- Tests first: a fixture with 3 ahead, 3 behind, 1 diverged, 1 dirty branch; a failing push must leave the pulls done and the exit code non-zero.

### Central `.gitignore` (owner, 2026-11-24)

`actrepo apply` skips an existing file, so a shared ignore rule never reaches a repository that already has a `.gitignore`. One source (`project-templates/common/.gitignore` in dotfiles) holds the shared rules; `actrepo apply` writes them as a block between `# >>> actrepo managed` and `# <<< actrepo managed`, replaces only that block, keeps the local lines, and `actrepo check` reports a block that differs from the source.

- First rules in the source: cargo-mutants output, and `/graphify-out/*` with `!/graphify-out/GRAPH_REPORT.md` (only the report is versioned).
- Tests first: no file (created with the block), file without the block (block appended, local lines kept), stale block (replaced, local lines kept), two blocks or an unclosed marker (error, file untouched), `check` drift, `--dry-run` writes nothing. `cargo mutants` with 0 missed.
- Migration: each repository with tracked `graph.json` and the other generated graphify files drops them from the index (`git rm --cached`) in its own pull request.

### Order

1. Read-only verbs first (`pr view`, `pr list`, `branch remote`, `pr check`): additive, no risk.
2. Then the writing single verbs (`worktree add|remove`, `pr retarget|close`).
3. Then the `actautoma` workflows, each with its own ADR (what it may do alone, what stops it).
4. Then a rule in `~/.claude/AGENTS.md` (dotfiles) that makes raw `git`/`gh` the exception. The rule text is proposed in the dotfiles pull request of the same day.
5. Then the bulk case above (`actrepo sync`, then `actautoma sync-all`): it needs `pull`, `push --branch` and `branch remote` first.
