# Roadmap

## Directive (owner, 2026-11-23; split changed 2026-11-25): `actautoma` handles every git and gh workflow

Every workflow an agent or the owner runs with `git` or `gh` goes through
`actautoma`; raw `git` / `gh` stays only for what it lacks, and each such use
names the missing verb so it can be added to the `ROADMAP.md` of `actautoma`.

**Split decided by the owner, 2026-11-25 (replaces the proposed split of
2026-11-23):** every git and gh verb belongs to `actautoma`, because each one
is a step of a chain (commit, then gate, then push, then pull request, then
merge). That covers commit, push, pull, fetch, branches, worktrees and pull
requests, and the chains over them (ship, merge a stack, resolve conflicts,
report upstream, sync). `actrepo` keeps only the life of a repository: `new`,
`check`, `apply`, `license`, `docs-paths`, `rename-branch`, `rename`, `scan`,
`list`, the registry, `audit` and `doctor`. `actautoma` stays thin over `git`
and `gh`, with the strictness of the rest of the doctrine (no silent failure,
a refusal names what and why).

**Transition.** The git and gh verbs `actrepo` has today (`pr create`,
`pr merge`, `push`, `sync`, `worktree list|prune`, `status`) keep working until
`actautoma` has the same verb. Then each is deprecated in `actrepo` (alias that
names the new owner), every caller is migrated (`~/Source`, `~/machine`, the
Claude settings, hooks and docs), and the removal is its own pull request.
The table of missing verbs below moves to the `ROADMAP.md` of `actautoma`; the
"Owner" column of every row reads `actautoma` from now on.

### Verbs missing today (found by shipping 30 PRs on 2026-11-22/23 with raw commands)

`actrepo` has `pr create`, `pr merge`, `push` (a repository and its nested ones), `worktree list|prune`, `status`; it has no `commit` and no `pull`.
Used by hand instead:

| Raw command | Wanted verb | Owner |
|---|---|---|
| `git add PATH… && git commit -m …` (by pathspec, never a peer's file) | `actautoma commit --path P… -m …` (refuses a path that is not listed, keeps the session's attribution trailer, runs the pre-commit gate, never `--no-verify`) | actautoma |
| `git pull`, `git fetch && git merge --ff-only` | `actautoma pull` (fast-forward only; refuses when incoming files overlap a dirty file, naming it; never switches the branch of a checkout) | actautoma |
| `git push -u origin B` of one named branch from a worktree, the gate output kept, the result checked afterwards | `actautoma push --branch B` (today `actautoma push` takes a repository and its nested ones, clean tree only); the result is read back with `branch remote`, not from an exit code | actautoma |
| `git worktree add .worktrees/X -b BRANCH origin/main` | `actautoma worktree add BRANCH [--from origin/main]` | actautoma |
| `git fetch`, `git ls-remote --heads origin B` | `actautoma branch remote B` (is it pushed, at which sha) | actautoma |
| `git branch -d B`, `git worktree remove --force` (generated files only) | `actautoma worktree remove`, `actautoma branch delete --merged` | actautoma |
| `gh pr view N --json state,mergeable,baseRefName` | `actautoma pr view N` | actautoma |
| `gh pr list --state open --json …` | `actautoma pr list` | actautoma |
| `gh pr edit N --base main` | `actautoma pr retarget N --base main` | actautoma |
| `gh pr close N --comment …` | `actautoma pr close N [--comment]` | actautoma |
| `git merge-tree --write-tree BASE HEAD` | `actautoma pr check N` (conflict dry run) | actautoma |
| merge `origin/main` in a detached worktree, fix, `git push origin HEAD:B` | `actautoma resolve-conflicts N` (worktree, merge, stop on conflict, push) | actautoma |
| push, `actrepo pr create` with the last subject, `actrepo pr merge --yes` (`ship.sh`) | `actautoma ship` (gates first, merge only where authorised) | actautoma |
| stacked PRs squash-merged: retarget the tip, dry run, merge, close the contained ones | `actautoma merge-stack` | actautoma |
| `gh issue create`, `gh search issues`, `gh repo fork` for a problem found upstream | `actautoma report-upstream` | actautoma |
| `actrepo pr merge` then restore a peer's checkout it switched to `main` | `actautoma pr merge --keep-checkout` (never switch a checkout that is not the merger's) | actautoma |

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

### Order

1. Read-only verbs first (`pr view`, `pr list`, `branch remote`, `pr check`): additive, no risk.
2. Then the writing single verbs (`worktree add|remove`, `pr retarget|close`).
3. Then the `actautoma` workflows, each with its own ADR (what it may do alone, what stops it).
4. Then a rule in `~/.claude/AGENTS.md` (dotfiles) that makes raw `git`/`gh` the exception. The rule text is proposed in the dotfiles pull request of the same day.
5. Then the bulk case above (`actrepo sync`, then `actautoma sync-all`): it needs `pull`, `push --branch` and `branch remote` first.
