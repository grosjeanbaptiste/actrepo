//! repo — name with actname, scaffold/manage with `dotfiles project`, PRs through actautoma.

use actautoma::{gh::Gh, process::ProcessRunner};
use actit::forge_names::ForgeNames;
use actrepo::{
    audit, branch, build, delegate, deploy, device, doctor, hook, install, license, naming, need,
    orchestrate, pin, pipeline, pr, push, registry, release, rename, root, run, scaffold, sync,
    workspace, worktree,
};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use naming::Query;
use pipeline::{Dotfiles, NewOpts, Source};
use scaffold::apply::Mode;
use scaffold::command;
use scaffold::native::{Native, Process, Quiet};
use scaffold::{plan, prompt};
use std::path::Path;

#[derive(Parser)]
#[command(name = "actrepo", bin_name = "actrepo", version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Name a new repository (actname, or --name), check it, then scaffold it.
    New {
        /// Project type, as in `dotfiles project setup` (e.g. package/rust); asked on stdin when omitted.
        kind: Option<String>,
        /// Use this exact name instead of asking actname; it must be free.
        #[arg(long, conflicts_with_all = ["prefixes", "domain"])]
        name: Option<String>,
        #[arg(long = "prefix", required_unless_present = "name")]
        prefixes: Vec<String>,
        #[arg(long, required_unless_present = "name")]
        domain: Option<String>,
        #[arg(long, default_value_t = 10)]
        max_length: usize,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value = "~/Source")]
        root: String,
        #[arg(long)]
        no_push: bool,
        /// Print the chosen name and stop.
        #[arg(long)]
        dry_run: bool,
        /// Scaffold with `dotfiles project setup` instead of the built-in Rust code.
        #[arg(long)]
        via_dotfiles: bool,
        /// Ask actit first whether a tool already covers this need; a reusable one stops the creation.
        #[arg(long)]
        need: Option<String>,
        /// Create the repository even though a reusable tool exists for `--need`.
        #[arg(long, requires = "need")]
        force: bool,
    },
    /// Audit a project against the standard templates.
    Check {
        #[arg(default_value = ".")]
        path: String,
        #[arg(long = "type")]
        kind: Option<String>,
    },
    /// Drop/refresh the standard files in an existing project (skips existing files).
    Apply {
        #[arg(default_value = ".")]
        path: String,
        #[arg(long = "type")]
        kind: Option<String>,
        /// Overwrite files that exist.
        #[arg(long, conflicts_with = "diff")]
        force: bool,
        /// Show what would change; write nothing.
        #[arg(long)]
        diff: bool,
    },
    /// Say which licence a project carries (none, proprietary, open, other).
    License {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Rename a repository's default branch, locally and on GitHub (master -> main).
    RenameBranch {
        #[arg(default_value = ".")]
        path: String,
        #[arg(long, default_value = "master")]
        from: String,
        #[arg(long, default_value = "main")]
        to: String,
        /// Print the steps; change nothing.
        #[arg(long)]
        dry_run: bool,
        /// Also delete the old branch on origin (irreversible).
        #[arg(long)]
        delete_old: bool,
    },
    /// Rename a repository everywhere: platform, origin, folder, written references, registry.
    Rename {
        /// `OWNER/OLD`, or just `OLD` when it is registered (the owner is read from its origin).
        slug: String,
        /// The new repository name.
        new: String,
        /// The local folder (default: the registry's path for OLD).
        #[arg(long)]
        dir: Option<String>,
        /// Move the renamed folder under this parent (default: next to the old one).
        #[arg(long)]
        into: Option<String>,
        /// The platform already carries the new name: skip the platform rename.
        #[arg(long)]
        local_only: bool,
        /// Print the renames; change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// List the registered projects (name and path).
    List {
        /// Print a JSON array instead of the text list.
        #[arg(long)]
        json: bool,
    },
    /// Find git repositories (default root: $HOME) and refresh the registry.
    Scan { roots: Vec<String> },
    /// Branch, ahead/behind, dirty files and remote for one project, or a table of all.
    Status {
        name: Option<String>,
        /// Print JSON (an object for NAME, else an array) instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Push a repository and every repository nested under it (clean tree, ahead of its upstream).
    Push {
        /// Root to scan.
        #[arg(long, default_value = ".")]
        dir: String,
        /// Report what would be pushed; touch nothing.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Push without asking.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Settle every branch of every repository under DIR: push what is ahead, fast-forward what is behind, report what diverged.
    Sync {
        #[arg(default_value = ".")]
        dir: String,
        /// Print the plan; touch nothing.
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
    /// Pull requests: create one from the current branch, merge one and update the local branches, view, list, retarget or close.
    Pr {
        #[command(subcommand)]
        sub: PrCmd,
    },
    /// List the env vars and CI secrets a project references, and where each is fulfilled.
    Audit {
        #[arg(default_value = ".")]
        path: String,
    },
    /// One-glance health report: git, open PRs and CI, quality signals, licence, audit.
    Doctor {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Install a registered Cargo project on PATH (clean, committed, with Cargo.lock; through the heavy queue).
    Install {
        name: String,
        /// Install one member of a Cargo workspace.
        #[arg(long)]
        package: Option<String>,
        /// Install only this binary.
        #[arg(long)]
        bin: Option<String>,
        /// Replace an existing binary from this project.
        #[arg(long)]
        force: bool,
    },
    /// Show where a git dependency is pinned across the workspace repos (fails when they differ), or pin them all to <rev>.
    Pin {
        /// Dependency name, as in Cargo.toml.
        dep: String,
        /// Full 40-character commit hash to pin every repo to.
        rev: Option<String>,
    },
    /// Run the project's release script with the machine profile and `.envrc` loaded (see `actrepo release help`).
    #[command(disable_help_flag = true)]
    Release {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Build and publish a project to an environment and store: web, ios, android, mas, homebrew, windows, linux, all.
    Deploy {
        /// prod | test
        env: String,
        /// web | ios | android | mas | homebrew | windows | linux | all
        target: String,
        /// Project directory.
        #[arg(default_value = ".")]
        path: String,
        /// Cloudflare Pages project name for web (default: the directory name).
        #[arg(long)]
        project: Option<String>,
        /// Do not ask for confirmation.
        #[arg(short, long)]
        yes: bool,
        /// Print what would run without doing it.
        #[arg(long)]
        dry_run: bool,
        /// ios/android only: push the store page from <path>/stores, no build, no binary, no review.
        #[arg(long)]
        store_page: bool,
    },
    /// Run a verb of a project (Makefile target, melos script, cargo/flutter command); without a verb, list them.
    Run {
        /// Directory or registered project name.
        name: String,
        /// The verb, then its arguments verbatim (a `--` among them is kept).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        rest: Vec<String>,
    },
    /// Folders of the current `*.code-workspace`: list, add, remove, status, push.
    Workspace {
        #[command(subcommand)]
        sub: WsCmd,
    },
    /// After a commit: pull the branch, push it. `install` puts the git hook in a repository.
    Hook {
        #[command(subcommand)]
        sub: HookCmd,
    },
    /// Linked worktrees: remove the ones whose branch is merged.
    Worktree {
        #[command(subcommand)]
        sub: WtCmd,
    },
    /// Run actit in-process: `actrepo actit "<need>"`, `actrepo actit --name N [--owner O] [--contains]`.
    Actit {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run actname in-process: `actrepo actname generate --prefix act --domain "<words>"`.
    Actname {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run actautoma in-process: `actrepo actautoma rename-repo OWNER/OLD NEW [--dir D] [--dry-run]`.
    Actautoma {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Move a folder under another parent, one repository or a folder of them (found by scanning): `actrepo move DIR --into PARENT [--dry-run]`.
    Move {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Candidates for the shell completion (used by `actrepo completions`).
    #[command(
        name = "__complete",
        hide = true,
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    Complete { words: Vec<String> },
    /// Print the zsh or bash completion script: `eval "$(actrepo completions zsh)"`.
    Completions { shell: String },
    /// `repo <dir> run|install [DEVICE] [--release]` runs or installs the app of <dir> on a device;
    /// any other command goes to `dotfiles project` (list, status, pr, doctor, …).
    #[command(external_subcommand)]
    Other(Vec<String>),
}

#[derive(Subcommand)]
enum HookCmd {
    /// Write the post-commit hook in the repository of DIR (never replaces another hook).
    Install {
        #[arg(default_value = ".")]
        dir: String,
        /// Say what would be written; touch nothing.
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
    /// What the hook calls: start `hook sync` in the background and return at once.
    Run,
    /// Fast-forward the current branch from origin and push it, now, in the foreground.
    Sync,
}

#[derive(Subcommand)]
enum WtCmd {
    /// Worktrees of the repository in the current folder.
    List {
        /// Print a JSON array instead of text.
        #[arg(long)]
        json: bool,
        /// Add the live agent sessions working in each worktree (runs `actsessions list --json`).
        #[arg(long)]
        owners: bool,
    },
    /// Remove each clean worktree whose pushed branch is contained in origin/BASE, and the branch.
    Prune {
        #[arg(long, default_value = "main")]
        base: String,
        /// Say what would be removed; touch nothing.
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum PrCmd {
    /// Open a PR for the current branch; title and description are asked on stdin when not given.
    Create {
        /// Base branch; the only master/main/develop/dev/staging/release candidate is used when omitted.
        #[arg(long)]
        base: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
    },
    /// Merge a PR (merge commit, branch deleted), then fast-forward the base locally.
    Merge {
        /// PR number; the PR of the current branch when omitted.
        number: Option<u64>,
        /// Merge without asking.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Show a PR: title, branches, author, state and mergeability.
    View {
        /// PR number; the PR of the current branch when omitted.
        number: Option<u64>,
    },
    /// List PRs, one line each.
    List {
        /// open, closed, merged or all.
        #[arg(long, value_enum, default_value = "open")]
        state: PrStateArg,
    },
    /// Change the base branch of a PR.
    Retarget { number: u64, base: String },
    /// Close a PR without merging it.
    Close {
        number: u64,
        /// Comment left on the PR when it is closed.
        #[arg(long)]
        comment: Option<String>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum PrStateArg {
    Open,
    Closed,
    Merged,
    All,
}

#[derive(Subcommand)]
enum WsCmd {
    /// Show the folders.
    List,
    /// Add a folder: adopt ./<name>, or clone or scaffold it first.
    Add {
        name: String,
        /// `git clone` this URL into ./<name>, then add it.
        #[arg(long, conflicts_with = "init")]
        clone: Option<String>,
        /// Scaffold a new project of this type at ./<name>, then add it.
        #[arg(long)]
        init: Option<String>,
    },
    /// Remove a folder from the workspace (the directory stays on disk).
    Remove { name: String },
    /// Git state of every folder.
    Status,
    /// Push every folder that has unpushed commits.
    Push {
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Pr { sub } => match sub {
            PrCmd::Create { base, title, body } => {
                pr::command::run_create(&pr::create::Opts { base, title, body })
            }
            PrCmd::Merge { number, yes } => {
                pr::command::run_merge(&pr::merge::Opts { number, yes })
            }
            PrCmd::View { number } => pr::command::run_view(number),
            PrCmd::List { state } => pr::command::run_list(match state {
                PrStateArg::Open => actautoma::pulls::PrFilter::Open,
                PrStateArg::Closed => actautoma::pulls::PrFilter::Closed,
                PrStateArg::Merged => actautoma::pulls::PrFilter::Merged,
                PrStateArg::All => actautoma::pulls::PrFilter::All,
            }),
            PrCmd::Retarget { number, base } => pr::command::run_retarget(number, &base),
            PrCmd::Close { number, comment } => pr::command::run_close(number, comment.as_deref()),
        },
        Cmd::Hook { sub } => match sub {
            HookCmd::Install { dir, dry_run } => {
                hook::command::run_install(Path::new(&dir), dry_run)
            }
            HookCmd::Run => hook::command::run_detached(),
            HookCmd::Sync => hook::command::run_sync(),
        },
        Cmd::Sync { dir, dry_run } => {
            let (mut out, mut err) = (std::io::stdout().lock(), std::io::stderr().lock());
            sync::run::run(
                Path::new(&dir),
                dry_run,
                &scaffold::native::Process,
                &mut out,
                &mut err,
            )
        }
        Cmd::Worktree { sub } => match sub {
            WtCmd::List { json, owners } => worktree::command::run_list(json, owners),
            WtCmd::Prune { base, dry_run } => worktree::command::run_prune(&base, dry_run),
        },
        Cmd::Workspace { sub } => match sub {
            WsCmd::List => workspace::command::run_list(),
            WsCmd::Status => workspace::command::run_status(),
            WsCmd::Remove { name } => workspace::command::run_remove(&name),
            WsCmd::Push { dry_run } => workspace::command::run_push(dry_run),
            WsCmd::Add { name, clone, init } => {
                let source = match (clone, init) {
                    (Some(url), _) => workspace::edit::Source::Clone(url),
                    (None, Some(kind)) => workspace::edit::Source::Init(kind),
                    (None, None) => workspace::edit::Source::Adopt,
                };
                workspace::command::run_add(&name, &source)
            }
        },
        Cmd::Audit { path } => audit::command::run(Path::new(&path)),
        Cmd::Install {
            name,
            package,
            bin,
            force,
        } => install::command::command(&install::Opts {
            name,
            package,
            bin,
            force,
        }),
        Cmd::Pin { dep, rev } => pin::command::command(&dep, rev.as_deref()),
        Cmd::Release { args } => release::command::command(&args),
        Cmd::Deploy {
            env,
            target,
            path,
            project,
            yes,
            dry_run,
            store_page,
        } => deploy::command::command_line(deploy::command::Args {
            env,
            target,
            path,
            project,
            yes,
            dry_run,
            store_page,
        }),
        Cmd::Run { name, rest } => match run::command::command(&name, &rest)? {
            0 => Ok(()),
            code => std::process::exit(code), // the project's own failure: its code, its diagnostics
        },
        Cmd::Doctor { path } => doctor::command::command(Path::new(&path)),
        Cmd::Actit { args } => {
            let env = |key: &str| std::env::var(key).ok();
            match orchestrate::actit(&args, &env, &mut std::io::stdout().lock())? {
                0 => Ok(()),
                code => std::process::exit(code), // actit's verdict: its code
            }
        }
        Cmd::Actname { args } => orchestrate::actname(&args, &mut std::io::stdout().lock()),
        Cmd::Actautoma { args } => {
            let env = |key: &str| std::env::var(key).ok();
            let cwd = std::env::current_dir().context("no current folder")?;
            orchestrate::actautoma(&args, &cwd, &env, &mut std::io::stdout().lock())
        }
        Cmd::Move { args } => {
            let env = |key: &str| std::env::var(key).ok();
            let cwd = std::env::current_dir().context("no current folder")?;
            let args: Vec<String> = ["move".to_string()].into_iter().chain(args).collect();
            orchestrate::actautoma(&args, &cwd, &env, &mut std::io::stdout().lock())
        }
        Cmd::Complete { words } => device::command::complete(&words),
        Cmd::Completions { shell } => {
            print!("{}", device::completions::script(&shell)?);
            Ok(())
        }
        Cmd::Other(args) if device::request::is_device_request(&args) => {
            device::command::command(&args)
        }
        Cmd::Other(args) if build::is_build_request(&args) => build::command(&args),
        Cmd::Other(args) => delegate::project(&args),
        Cmd::RenameBranch {
            path,
            from,
            to,
            dry_run,
            delete_old,
        } => branch::command::run(&path, &from, &to, dry_run, delete_old),
        Cmd::Rename {
            slug,
            new,
            dir,
            into,
            local_only,
            dry_run,
        } => rename::command::run(
            &slug,
            &new,
            dir.as_deref(),
            into.as_deref(),
            local_only,
            dry_run,
        ),
        Cmd::List { json } => registry::command::run_list(json),
        Cmd::Scan { roots } => registry::command::run_scan(&roots),
        Cmd::Status { name, json } => registry::command::run_status(name.as_deref(), json),
        Cmd::Push { dir, dry_run, yes } => {
            let opts = push::Opts {
                dir: dir.into(),
                dry_run,
                yes,
            };
            let (mut out, mut err) = (std::io::stdout().lock(), std::io::stderr().lock());
            push::run(
                &opts,
                &Quiet,
                &Process,
                &mut std::io::stdin().lock(),
                &mut out,
                &mut err,
            )
        }
        Cmd::License { path } => {
            println!("{}", license::doctor_line(Path::new(&path))?);
            Ok(())
        }
        Cmd::Check { path, kind } => command::run(Path::new(&path), kind.as_deref(), Mode::Check),
        Cmd::Apply {
            path,
            kind,
            force,
            diff,
        } => {
            let mode = if force {
                Mode::Force
            } else if diff {
                Mode::Diff
            } else {
                Mode::Apply
            };
            command::run(Path::new(&path), kind.as_deref(), mode)
        }
        Cmd::New {
            kind,
            name,
            prefixes,
            domain,
            max_length,
            limit,
            root,
            no_push,
            dry_run,
            via_dotfiles,
            need,
            force,
        } => {
            if let Some(need) = &need {
                need::check(need, force, &need::ActitTools, &mut std::io::stderr())?;
            }
            let native = Native::from_env()?;
            let kind = match kind {
                Some(kind) => kind,
                None => prompt::ask(
                    &plan::known_types(&native.templates)?,
                    &mut std::io::stdin().lock(),
                    &mut std::io::stderr(),
                )?,
            };
            let source = match name {
                Some(name) => Source::Named(name),
                None => Source::Generated(Query {
                    prefixes,
                    domain: domain.unwrap_or_default(),
                    max_length,
                    limit,
                }),
            };
            let opts = NewOpts {
                kind,
                source,
                root: root::expand(&root, std::env::var("HOME").ok())?,
                no_push,
                dry_run,
                registry: Some(registry::file::path(|k| std::env::var(k).ok())?),
            };
            let names = ForgeNames::new(Gh::new(ProcessRunner));
            let name = if via_dotfiles {
                pipeline::run(&opts, &names, &Dotfiles)?
            } else {
                pipeline::run(&opts, &names, &native)?
            };
            if dry_run {
                println!("{name}");
            }
            Ok(())
        }
    }
}
