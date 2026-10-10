//! Maintainer commands for the inkpdf repository: `cargo xtask <command>`.
//!
//! - `docs generate|check`: generated reference pages of the documentation site.
//! - `packages …`: the bundled Typst packages (see `packages.rs`).

mod docs;
mod lock;
mod packages;
mod universe;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo xtask", about = "Maintainer commands for inkpdf")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generated pages of the documentation site.
    Docs {
        #[command(subcommand)]
        action: DocsAction,
    },
    /// The bundled Typst packages (`packages/lock.toml` and `packages/vendor/`).
    Packages {
        #[command(subcommand)]
        action: PackagesAction,
    },
}

#[derive(Subcommand)]
enum PackagesAction {
    /// Bundles a package (latest compatible version by default) and its dependencies.
    Add {
        name: String,
        version: Option<String>,
        /// Shows what would change without writing anything.
        #[arg(long)]
        dry_run: bool,
    },
    /// Changes the version of a bundled package (latest compatible by default).
    Update {
        name: String,
        version: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Removes a bundled package and the dependencies nothing else needs.
    Remove {
        name: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Checks digests, archives, selected versions, import closure and generated docs.
    Verify,
    /// Lists the bundled packages.
    List,
}

#[derive(Subcommand)]
enum DocsAction {
    /// Regenerates the generated reference pages.
    Generate,
    /// Fails when a generated page is out of date.
    Check,
}

/// Root of the repository (parent of the `xtask` crate).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in the repository")
        .to_path_buf()
}

fn run_packages(action: PackagesAction) -> anyhow::Result<()> {
    use packages::{Change, State};

    let root = repo_root();
    let state = State::load(&root)?;
    let (change, dry_run) = match &action {
        PackagesAction::Verify => {
            let problems = packages::problems(&state);
            for problem in &problems {
                eprintln!("{problem}");
            }
            docs::check(&root)?;
            if !problems.is_empty() {
                anyhow::bail!("{} problem(s) in the bundled packages", problems.len());
            }
            println!("{} bundled packages verified", state.entries.len());
            return Ok(());
        }
        PackagesAction::List => {
            print!("{}", packages::list(&state)?);
            return Ok(());
        }
        PackagesAction::Add {
            name,
            version,
            dry_run,
        } => (
            Change::Add {
                name,
                version: version.as_deref(),
            },
            *dry_run,
        ),
        PackagesAction::Update {
            name,
            version,
            dry_run,
        } => (
            Change::Update {
                name,
                version: version.as_deref(),
            },
            *dry_run,
        ),
        PackagesAction::Remove { name, dry_run } => (Change::Remove { name }, *dry_run),
    };
    let engine = universe::engine_version(&root)?;
    let (next, summary) = packages::plan(&state, change, &universe::Universe::new(), engine)?;
    for line in &summary {
        println!("{line}");
    }
    if dry_run {
        println!("dry run: nothing written");
    } else {
        packages::write(&root, &state, &next)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Docs {
            action: DocsAction::Generate,
        } => docs::generate(&repo_root()),
        Command::Docs {
            action: DocsAction::Check,
        } => docs::check(&repo_root()),
        Command::Packages { action } => run_packages(action),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
