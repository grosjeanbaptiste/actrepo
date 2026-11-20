// `repo` is the former name of `actrepo`, kept as an alias while consumers migrate:
// the same program, so a module over `main.rs` rather than a second copy.
#[path = "../main.rs"]
mod actrepo_main;

fn main() -> anyhow::Result<()> {
    actrepo_main::main()
}
