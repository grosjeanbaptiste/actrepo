---
name: actrepo
description: Manage git repositories on this machine with the actrepo CLI: create, rename, move, merge a pull request, prune worktrees, check status. Use before any git-hosting or repository-layout task.
---

# actrepo

`actrepo <command>` is the single entry point for repository management. Never call `gh pr merge` or edit the registry by hand.

## Commands to use

- New repository: `actrepo new` (name from actname, then scaffold). Exact name: `--name <n>`.
- Merge a pull request: `actrepo pr merge <N> --yes`. It verifies the merge, fast-forwards the base, removes the merged worktree and branch, and triggers `actinstall`.
- Remove merged worktrees: `actrepo worktree prune [--base main] [-n]`.
- Create a pull request: `actrepo pr create`.
- Rename: `actrepo rename OWNER/OLD NEW [--dir D] [--dry-run]`. Move a folder: `actrepo move DIR --into PARENT [--dry-run]`.
- Status: `actrepo status [NAME]`; health report: `actrepo doctor [PATH]`; list: `actrepo list`.
- Push a repository and its nested ones: `actrepo push [--dry-run] [--yes]`.
- Deploy: `actrepo deploy ENV TARGET [--dry-run]`. TARGET is web, ios, android, mas, homebrew, windows, linux, `mobile` (ios, android and every desktop target) or all. A beta Homebrew deploy (`deploy test homebrew`, with `BREW_TAP_DIR`, no `BREW_DMG` and no `BREW_APP_URL`) builds the dmg itself: Developer ID export (`APPLE_TEAM_ID`), notarization (`ASC_KEY_ID`, `ASC_ISSUER_ID`), dmg, then the GitHub prerelease and the `@beta` cask. Check with `--dry-run` first; `BREW_PUSH` is the only thing that pushes the tap.

## Limits

- Merging is allowed in every repository except `acteble_api` and `melos_apps`.
- `actrepo` never runs `gh` itself: forge operations go through actit and actautoma.
- A command that finds the work tree dirty refuses; commit or move the changes first.

Source of truth for options: `actrepo <command> --help`.
