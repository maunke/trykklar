//! Xtask workflow à la https://github.com/matklad/cargo-xtask
#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
mod tasks;
pub mod tool;
use anyhow::Result;

pub fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .expect("xtask subdirectory on workspace root")
        .to_path_buf()
}

#[derive(Parser)]
#[clap(name = "xtask", about = "trykklar repo automation")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lint
    Lint,
    /// Format: .rs, .md. and .toml files
    Fmt,
    /// Test
    Test,
    /// Validate
    Validate,
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{e:#}");
        std::process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Lint => tasks::lint()?,
        Commands::Fmt => tasks::fmt()?,
        Commands::Test => tasks::test()?,
        Commands::Validate => {
            tasks::lint()?;
            tasks::test()?;
        }
    }
    Ok(())
}
