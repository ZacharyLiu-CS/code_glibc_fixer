use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;

mod prepare;
mod fix;
mod watch;

#[derive(Parser)]
#[command(
    name = "code_glibc_fixer",
    version,
    about = "Fix VSCode Server glibc version issues on systems with old glibc (e.g. tlinux2/glibc-2.17)",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download required dynamic libraries and install required tools (patchelf, rpm2cpio, cpio)
    Prepare,

    /// Fix the VSCode Server binary and check-requirements.sh for a given vscode-server bin directory
    Fix {
        /// Path to the .vscode-server/bin directory to watch (enables watch mode)
        #[arg(long, value_name = "DIR")]
        watch: Option<PathBuf>,

        /// Path to a specific vscode-server bin/<hash> directory to fix (e.g. ~/.vscode-server/bin/<hash>).
        /// If omitted (without --watch), fixes all existing entries under ~/.vscode-server/bin
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Prepare => {
            prepare::run()
        }
        Commands::Fix { watch: Some(watch_dir), .. } => {
            // Watch mode: fix all existing entries first, then watch for new ones
            println!("[code_glibc_fixer] Fix + Watch mode: {}", watch_dir.display());
            fix::fix_all_in_dir(&watch_dir)?;
            watch::run(&watch_dir)
        }
        Commands::Fix { watch: None, path } => {
            let dir = path.unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                PathBuf::from(home).join(".vscode-server").join("bin")
            });
            println!("[code_glibc_fixer] Fix mode: {}", dir.display());
            fix::fix_all_in_dir(&dir)
        }
    }
}
